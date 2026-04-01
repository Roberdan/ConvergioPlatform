pub mod api_routes;
pub mod api_routes_ext;
mod health;
pub mod rate_limiter;

pub use api_routes::{DELETE_ROUTES, GET_ROUTES, POST_ROUTES, PUT_ROUTES, SSE_ROUTES, WS_ROUTES};

use super::{
    api_agent_catalog, api_agent_control, api_agent_history, api_agent_profiles, api_agent_triage, api_agents,
    api_audit, api_budget, api_build_exec, api_capabilities, api_channels, api_chat,
    api_coordinator, api_crdt, api_dashboard, api_decisions, api_delegation, api_deliverables,
    api_digest, api_domain, api_evolution, api_github, api_goal, api_health_deep,
    api_health_post_merge, api_heartbeat, api_ideas, api_inference_status, api_ingest, api_ipc,
    api_kernel_audio, api_memory, api_memory_mgmt, api_mesh, api_mesh_update, api_metrics,
    api_nightly, api_notify,
    api_marketplace, api_marketplace_ops, api_night,
    api_node_readiness, api_node_roles, api_openclaw, api_org_chart, api_org_chart_global,
    api_org_metrics, api_org_timeline, api_orgs, api_plan_org, api_peers, api_peers_ext, api_plan_db,
    api_plan_db_checkpoint, api_plan_db_execution_context, api_plan_db_import,
    api_plan_db_lifecycle, api_plan_db_ops, api_plan_db_query, api_plan_db_review, api_plans,
    api_policy, api_project_tree, api_readiness, api_repositories, api_rollback, api_runs,
    api_sync, api_tracking, api_validation, api_voice, api_workers, api_workspace,
    api_workspace_events, mesh_provision, middleware as server_mw, middleware_audit, sse,
    state::ServerState, telemetry, ws, ws_pty,
};
use crate::inference::health_loop::{create_shared_health, spawn_health_probe_loop};
use api_routes::{endpoint_category, RateLimiter};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, State},
    http::{Request, StatusCode},
    middleware::{from_fn, from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::{get, get_service},
    Router,
};
use std::{env, path::PathBuf, time::Duration};
use tower_http::{services::ServeDir, timeout::TimeoutLayer};

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
    let dashboard_web_files =
        ServeDir::new(PathBuf::from(super::DASHBOARD_STATIC_DIR)).append_index_html_on_directories(true);
    let rate_limiter = RateLimiter::default();

    // Shared inference health state: probed every 60s in background.
    let health_state = create_shared_health();
    spawn_health_probe_loop(health_state.clone());
    api_orgs::spawn_background_jobs(state.clone());

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
        .merge(api_agent_history::router())
        .merge(api_agents::router())
        .merge(api_agent_profiles::router())
        .merge(api_mesh::router())
        .route("/api/mesh/update-status", get(api_mesh_update::handle_update_status))
        .merge(api_peers::router())
        .merge(api_peers_ext::router())
        .merge(api_nightly::router())
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
        .merge(api_orgs::router())
        .merge(api_org_timeline::router())
        .merge(api_org_metrics::router())
        .merge(api_org_chart::router())
        .merge(api_org_chart_global::router())
        .merge(api_plan_org::router())
        .merge(api_marketplace::router())
        .merge(api_marketplace_ops::router())
        .merge(api_night::router())
        .merge(api_crdt::router())
        .merge(api_sync::router())
        .merge(api_capabilities::router())
        .merge(api_channels::router())
        .merge(api_health_deep::router())
        .merge(api_health_post_merge::router())
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
        .merge(api_policy::router())
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
        .layer(axum::Extension(health_state))
        .layer(from_fn_with_state(state.clone(), middleware_audit::audit_mutations))
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
        .nest_service("/scripts/dashboard_web", dashboard_web_files)
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
