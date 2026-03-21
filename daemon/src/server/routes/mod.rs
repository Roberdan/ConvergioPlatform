pub mod api_routes;

pub use api_routes::{
    DELETE_ROUTES, GET_ROUTES, POST_ROUTES, PUT_ROUTES, SSE_ROUTES, WS_ROUTES,
};

use super::api_agents;
use super::api_chat;
use super::api_coordinator;
use super::api_dashboard;
use super::api_evolution;
use super::api_github;
use super::api_heartbeat;
use super::api_ideas;
use super::api_ipc;
use super::api_mesh;
use super::api_notify;
use super::api_peers;
use super::api_peers_ext;
use super::api_plan_db;
use super::api_plan_db_import;
use super::api_plan_db_lifecycle;
use super::api_plan_db_ops;
use super::api_plan_db_query;
use super::api_plans;
use super::api_workers;
use super::mesh_provision;
use super::middleware as server_mw;
use super::sse;
use super::state::ServerState;
use super::ws;
use super::ws_pty;
use api_routes::{endpoint_category, RateLimiter};
use axum::body::Body;
use axum::extract::DefaultBodyLimit;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::middleware::{from_fn, from_fn_with_state, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, get_service};
use axum::{Json, Router};
use std::env;
use std::path::PathBuf;
use std::time::Duration;
use tower_http::services::ServeDir;
use tower_http::timeout::TimeoutLayer;

pub fn build_router(static_dir: PathBuf, crsqlite_path: Option<String>) -> Router {
    let db_path = env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".claude/data/dashboard.db");
    build_router_with_db(static_dir, db_path, crsqlite_path)
}

pub fn build_router_with_db(
    static_dir: PathBuf,
    db_path: PathBuf,
    crsqlite_path: Option<String>,
) -> Router {
    let static_files = ServeDir::new(static_dir).append_index_html_on_directories(true);
    let state = ServerState::new(db_path, crsqlite_path);
    let rate_limiter = RateLimiter::default();

    Router::new()
        .merge(api_dashboard::router())
        .merge(api_ideas::router())
        .merge(api_plans::router())
        .merge(api_agents::router())
        .merge(api_mesh::router())
        .merge(api_peers::router())
        .merge(api_peers_ext::router())
        .merge(api_notify::router())
        .merge(api_chat::router())
        .merge(api_coordinator::router())
        .merge(api_github::router())
        .merge(api_heartbeat::router())
        .merge(api_ipc::router())
        .merge(api_plan_db::router())
        .merge(api_plan_db_lifecycle::router())
        .merge(api_plan_db_query::router())
        .merge(api_plan_db_import::router())
        .merge(api_plan_db_ops::router())
        .merge(api_workers::router())
        .merge(api_evolution::router())
        .route("/api/chat/stream/:sid", get(sse::chat_stream_sse))
        .route("/api/mesh/action/stream", get(sse::mesh_action_sse))
        .route("/api/mesh/fullsync", get(sse::mesh_action_sse))
        .route("/api/plan/preflight", get(sse::plan_preflight_sse))
        .route("/api/plan/delegate", get(sse::plan_delegate_sse))
        .route("/api/plan/start", get(sse::plan_start_sse))
        .route("/api/mesh/pull-db", get(sse::mesh_action_sse))
        .route("/ws/brain", get(ws::ws_brain))
        .route("/ws/dashboard", get(ws::ws_dashboard))
        .route("/ws/pty", get(ws_pty::ws_pty))
        .route("/api/mesh/provision", get(mesh_provision::provision_all))
        .route("/api/health", get(api_health))
        .layer(from_fn_with_state(rate_limiter, basic_rate_limit))
        .layer(from_fn(server_mw::require_auth))
        .layer(from_fn(server_mw::set_cache_headers))
        .layer(DefaultBodyLimit::max(1_048_576))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(server_mw::cors_layer())
        .layer(
            tower_http::compression::CompressionLayer::new()
                .gzip(true)
                .no_br()
                .no_deflate()
                .no_zstd(),
        )
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(state)
        .fallback_service(get_service(static_files))
}

async fn basic_rate_limit(
    State(rate_limiter): State<RateLimiter>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();
    let category = endpoint_category(path);
    // Tiered limits: reads get 600/min, writes 300/min, SSE/WS unlimited
    let limit = if path.starts_with("/ws/") || path.contains("/stream") {
        return next.run(request).await; // no limit on streaming
    } else if request.method() == axum::http::Method::GET {
        600
    } else {
        300
    };
    let allowed = rate_limiter
        .allow(category, limit, Duration::from_secs(60))
        .await;
    if !allowed {
        return (StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").into_response();
    }
    next.run(request).await
}

async fn api_health(State(state): State<ServerState>) -> Json<serde_json::Value> {
    // Cache health response for 5 seconds to avoid repeated DB queries
    static CACHE: std::sync::OnceLock<tokio::sync::Mutex<(std::time::Instant, serde_json::Value)>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| {
        tokio::sync::Mutex::new((
            std::time::Instant::now() - Duration::from_secs(10),
            serde_json::json!({}),
        ))
    });

    let mut guard = cache.lock().await;
    if guard.0.elapsed() < Duration::from_secs(5) {
        // Update only uptime (cheap)
        let mut cached = guard.1.clone();
        cached["uptime_secs"] = serde_json::json!(state.started_at.elapsed().as_secs());
        return Json(cached);
    }

    let uptime_secs = state.started_at.elapsed().as_secs();
    let db = state.open_db();
    let db_ok = db.is_ok();
    let (table_count, agent_activity_ok, peer_count) = match db {
        Ok(db) => {
            let conn = db.connection();
            let tables = super::state::query_one(
                conn,
                "SELECT COUNT(*) AS c FROM sqlite_master WHERE type='table'",
                [],
            )
            .ok()
            .flatten()
            .and_then(|v| v.get("c").and_then(serde_json::Value::as_i64))
            .unwrap_or(0);
            let aa_ok = conn.prepare("SELECT 1 FROM agent_activity LIMIT 0").is_ok();
            let peers =
                super::state::query_one(conn, "SELECT COUNT(*) AS c FROM peer_heartbeats", [])
                    .ok()
                    .flatten()
                    .and_then(|v| v.get("c").and_then(serde_json::Value::as_i64))
                    .unwrap_or(0);
            (tables, aa_ok, peers)
        }
        Err(_) => (0, false, 0),
    };
    let result = serde_json::json!({
        "ok": db_ok && agent_activity_ok,
        "db": db_ok,
        "tables": table_count,
        "agent_activity": agent_activity_ok,
        "peers": peer_count,
        "uptime_secs": uptime_secs,
        "version": env!("CARGO_PKG_VERSION"),
    });
    *guard = (std::time::Instant::now(), result.clone());
    Json(result)
}
