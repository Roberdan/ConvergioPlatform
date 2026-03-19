pub mod api_agents;
pub mod api_chat;
pub mod api_coordinator;
pub mod api_dashboard;
pub mod api_evolution;
pub mod api_github;
pub mod api_heartbeat;
pub mod api_ideas;
pub mod llm_client;
pub mod api_ipc;
pub mod api_mesh;
pub mod api_notify;
pub mod api_peers;
pub mod api_peers_ext;
pub mod api_plan_db;
pub mod api_plan_db_import;
pub mod api_plan_db_lifecycle;
pub mod api_plan_db_ops;
pub mod api_plan_db_query;
pub mod api_plans;
pub mod api_workers;
pub mod mesh_provision;
pub mod middleware;
pub mod routes;
pub mod sse;
pub mod sse_chat;
pub mod sse_delegate;
pub mod sse_preflight;
pub mod state;
pub mod ws;
pub mod ws_brain;
pub mod ws_pty;

#[cfg(test)]
mod api_agents_legacy_tests;
#[cfg(test)]
mod api_ideas_tests;
#[cfg(test)]
mod api_ipc_tests;
#[cfg(test)]
mod api_tests;
#[cfg(test)]
mod ws_pty_tests;

use axum::Router;
use std::path::{Path, PathBuf};

pub const DASHBOARD_STATIC_DIR: &str = "scripts/dashboard_web";

pub fn app(static_dir: impl Into<PathBuf>, crsqlite_path: Option<String>) -> Router {
    routes::build_router(static_dir.into(), crsqlite_path)
}

pub fn resolve_dashboard_static_dir(repo_root: impl AsRef<Path>) -> PathBuf {
    repo_root.as_ref().join(DASHBOARD_STATIC_DIR)
}

pub async fn run(
    bind_addr: &str,
    static_dir: impl Into<PathBuf>,
    crsqlite_path: Option<String>,
) -> Result<(), String> {
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .map_err(|e| format!("server listen failed on {bind_addr}: {e}"))?;
    axum::serve(listener, app(static_dir, crsqlite_path).into_make_service())
        .await
        .map_err(|e| format!("server runtime failed: {e}"))
}
