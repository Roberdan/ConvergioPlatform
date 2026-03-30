pub mod api_routes;
mod health;
pub mod rate_limiter;

pub use api_routes::{DELETE_ROUTES, GET_ROUTES, POST_ROUTES, PUT_ROUTES, SSE_ROUTES, WS_ROUTES};


use super::api_validation;
use super::api_agent_catalog;
use super::api_agent_triage;
use super::api_agents;
use super::api_agent_profiles;
use super::api_build_exec;
use super::api_audit;
use super::api_budget;
use super::api_capabilities;
use super::api_channels;
use super::api_delegation;
use super::api_chat;
use super::api_coordinator;
use super::api_crdt;
use super::api_sync;
use super::api_dashboard;
use super::api_digest;
use super::api_deliverables;
use super::api_domain;
use super::api_evolution;
use super::api_github;
use super::api_health_deep;
use super::api_heartbeat;
use super::api_ideas;
use super::api_ingest;
use super::api_ipc;
use super::api_kernel_audio;
use super::api_voice;
use super::api_memory;
use super::api_memory_mgmt;
use super::api_mesh;
use super::api_metrics;
use super::api_notify;
use super::api_openclaw;
use super::api_peers;
use super::api_peers_ext;
use super::api_plan_db;
use super::api_plan_db_checkpoint;
use super::api_plan_db_execution_context;
use super::api_plan_db_import;
use super::api_plan_db_lifecycle;
use super::api_plan_db_ops;
use super::api_plan_db_query;
use super::api_plan_db_review;
use super::api_plans;
use super::api_project_tree;
use super::api_agent_control;
use super::api_node_readiness;
use super::api_node_roles;
use super::api_readiness;
use super::api_rollback;
use super::api_runs;
use super::api_tracking;
use super::api_workers;
use super::api_workspace;
use super::api_workspace_events;
use super::api_decisions;
use super::api_goal;
use super::api_repositories;
use super::api_inference_status;
use super::mesh_provision;
use super::middleware as server_mw;
use super::sse;
use super::state::ServerState;
use super::telemetry;
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
use axum::Router;
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
    let state = ServerState::new(db_path, crsqlite_path);
    build_router_with_state(static_dir, state)
}

/// Build router with a pre-configured ServerState (for unified daemon with shared IPC engine).
pub fn build_router_with_state(static_dir: PathBuf, state: ServerState) -> Router {
    let static_files = ServeDir::new(static_dir).append_index_html_on_directories(true);
    let rate_limiter = RateLimiter::default();

    Router::new()
        .merge(api_validation::router())
        .merge(api_dashboard::router())
        .merge(api_digest::router())
        .merge(api_build_exec::router())
        .merge(api_budget::router())
        .merge(api_ideas::router())
        .merge(api_plans::router())
        .merge(api_agent_catalog::router())
        .merge(api_agent_triage::router())
        .merge(api_agents::router())
        .merge(api_agent_profiles::router())
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
        .merge(api_plan_db_review::router())
        .merge(api_plan_db_checkpoint::router())
        .merge(api_plan_db_execution_context::router())
        .merge(api_project_tree::router())
        .merge(api_node_readiness::router())
        .merge(api_node_roles::router())
        .merge(api_agent_control::router())
        .merge(api_readiness::router())
        .merge(api_tracking::router())
        .merge(api_workers::router())
        .merge(api_delegation::router())
        .merge(api_evolution::router())
        .merge(api_rollback::router())
        .merge(api_runs::router())
        .merge(api_metrics::router())
        .merge(api_ingest::router())
        .merge(api_deliverables::router())
        .merge(api_audit::router())
        .merge(api_domain::router())
        .merge(api_openclaw::router())
        .merge(api_crdt::router())
        .merge(api_sync::router())
        .merge(api_capabilities::router())
        .merge(api_channels::router())
        .merge(api_health_deep::router())
        .merge(api_kernel_audio::router())
        .merge(api_voice::router())
        .merge(api_memory::router())
        .merge(api_memory_mgmt::router())
        .merge(api_workspace::router())
        .merge(api_workspace_events::router())
        .merge(api_decisions::router())
        .merge(api_goal::router())
        .merge(api_repositories::router())
        .merge(api_inference_status::router())
        // Kernel inference routes (feature-gated; uses own KernelState)
        .merge({
            #[cfg(feature = "kernel")]
            {
                use crate::kernel::api::handlers::KernelState;
                use crate::kernel::engine::KernelConfig;
                let ks = KernelState::new(KernelConfig::default());
                crate::kernel::api::handlers::router().with_state(ks)
            }
            #[cfg(not(feature = "kernel"))]
            {
                Router::new()
            }
        })
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
        .route("/api/health", get(health::api_health))
        .route("/api/telemetry", get(health::api_telemetry))
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
        .layer(from_fn(telemetry::telemetry_layer))
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
