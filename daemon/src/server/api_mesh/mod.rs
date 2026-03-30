pub mod api_mesh_convergence;
pub mod handlers;
pub mod handlers_admin;
pub mod peer_conf;
pub mod sync_ops;

use super::state::ServerState;
use axum::routing::{get, post};
use axum::Router;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/mesh", get(handlers::api_mesh))
        .route("/api/mesh/convergence", get(api_mesh_convergence::handle_mesh_convergence))
        .route("/api/mesh/logs", get(sync_ops::api_mesh_logs))
        .route("/api/mesh/metrics", get(sync_ops::api_mesh_metrics))
        .route("/api/mesh/sync-stats", get(sync_ops::api_mesh_sync_stats))
        .route("/api/mesh/sync-status", get(sync_ops::api_mesh_sync_status))
        .route("/api/mesh/traffic", get(sync_ops::api_mesh_traffic))
        .route("/api/mesh/init", post(handlers::api_mesh_init))
        .route("/api/mesh/action", get(handlers_admin::handle_mesh_action))
        .route(
            "/api/mesh/delegate/:id/cancel",
            post(handlers::handle_delegate_cancel),
        )
}
