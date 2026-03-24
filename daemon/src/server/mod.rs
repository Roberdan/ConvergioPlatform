pub mod api_agent_catalog;
mod api_agent_catalog_parse;
pub mod api_agent_triage;
pub mod api_agents;
pub mod api_audit;
pub mod api_chat;
pub mod api_coordinator;
pub mod api_crdt;
pub mod api_dashboard;
pub mod api_deliverables;
pub mod api_deliverables_handlers;
pub mod api_domain;
pub mod api_evolution;
pub mod api_github;
pub mod api_github_handlers;
pub mod api_heartbeat;
pub mod api_heartbeat_handlers;
pub mod api_ideas;
pub mod api_ideas_handlers;
pub mod api_ingest;
pub mod api_ipc;
pub mod api_mesh;
pub mod api_metrics;
pub mod api_metrics_queries;
pub mod api_notify;
pub mod api_openclaw;
pub mod api_peers;
pub mod api_peers_ext;
pub mod api_plan_db;
pub mod api_plan_db_agents;
pub mod api_plan_db_checkpoint;
pub mod api_plan_db_import;
pub mod api_plan_db_import_defaults;
pub mod api_plan_db_import_parsers;
pub mod api_plan_db_lifecycle;
pub mod api_plan_db_ops;
pub mod api_plan_db_query;
pub mod api_plan_db_query_fmt;
pub mod api_plan_db_review;
pub mod api_plans;
pub mod api_project_tree;
pub mod api_readiness;
pub mod api_runs;
pub mod api_runs_handlers;
pub mod api_tracking;
pub mod api_workers;
pub mod api_workspace;
pub mod api_workspace_events;
pub mod llm_client;
pub mod mesh_provision;
pub mod middleware;
pub mod plan_lifecycle_guards;
#[cfg(test)]
mod plan_lifecycle_guards_tests;
pub mod routes;
pub mod sse;
pub mod sse_chat;
pub mod sse_delegate;
pub mod sse_preflight;
pub mod sse_stream;
pub mod state;
pub mod state_init;
pub mod state_init_canon;
mod state_init_migrations;
pub mod ws;
pub mod ws_brain;
pub mod ws_pty;

#[cfg(test)]
mod api_agent_catalog_tests;
#[cfg(test)]
mod api_agents_brain_tests;
#[cfg(test)]
mod api_agents_legacy_tests;
#[cfg(test)]
mod api_agents_tests;
#[cfg(test)]
mod api_audit_tests;
#[cfg(test)]
mod api_chat_tests;
#[cfg(test)]
mod api_chat_tests_msg;
#[cfg(test)]
mod api_coordinator_tests;
#[cfg(test)]
mod api_cross_feature_helpers;
#[cfg(test)]
mod api_cross_feature_plan_tests;
#[cfg(test)]
mod api_cross_feature_tests;
#[cfg(test)]
mod api_deliverables_tests;
#[cfg(test)]
mod api_domain_tests;
#[cfg(test)]
mod api_evolution_tests;
#[cfg(test)]
mod api_github_tests;
#[cfg(test)]
mod api_heartbeat_tests;
#[cfg(test)]
mod api_ideas_tests;
#[cfg(test)]
mod api_ideas_tests_filter;
#[cfg(test)]
mod api_ingest_tests;
#[cfg(test)]
mod api_ipc_tests;
#[cfg(test)]
mod api_metrics_tests;
#[cfg(test)]
mod api_openclaw_tests;
#[cfg(test)]
mod api_peers_tests;
#[cfg(test)]
mod api_plan_db_checkpoint_tests;
#[cfg(test)]
mod api_plan_db_query_tests;
#[cfg(test)]
mod api_plans_tests;
#[cfg(test)]
mod api_runs_tests;
#[cfg(test)]
mod api_runs_tests_lifecycle;
#[cfg(test)]
mod api_tests;
#[cfg(test)]
mod api_tracking_tests;
#[cfg(test)]
mod api_workspace_events_tests;
#[cfg(test)]
mod api_workspace_integration_tests;
#[cfg(test)]
mod api_workspace_tests;
#[cfg(test)]
mod state_init_tests;
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

/// Inner logic: given the token presence state, determine effective bind address.
/// Separated for deterministic unit testing without env-var races.
pub(crate) fn resolve_bind_addr_with(requested: &str, dev_mode: bool, has_token: bool) -> String {
    if dev_mode && !has_token {
        // Force 127.0.0.1 to prevent accidental network exposure of an
        // unauthenticated server. Keep the original port if parseable.
        if let Some(port) = requested.rsplit(':').next() {
            if port.parse::<u16>().is_ok() {
                return format!("127.0.0.1:{port}");
            }
        }
        return "127.0.0.1:8420".to_string();
    }
    requested.to_string()
}

/// Determine the effective bind address, reading token state from the environment.
///
/// When `dev_mode` is true and no `CONVERGIO_AUTH_TOKEN` is set, we force
/// 127.0.0.1 regardless of the requested bind address to prevent accidental
/// network exposure of an unauthenticated server.
pub fn resolve_bind_addr(requested: &str, dev_mode: bool) -> String {
    use std::env;
    let has_token = env::var("CONVERGIO_AUTH_TOKEN")
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    resolve_bind_addr_with(requested, dev_mode, has_token)
}

pub async fn run(
    bind_addr: &str,
    static_dir: impl Into<PathBuf>,
    crsqlite_path: Option<String>,
) -> Result<(), state::ApiError> {
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .map_err(|e| {
            state::ApiError::internal(format!("server listen failed on {bind_addr}: {e}"))
        })?;
    axum::serve(listener, app(static_dir, crsqlite_path).into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| state::ApiError::internal(format!("server runtime failed: {e}")))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};

        if let Ok(mut sigterm) = signal(SignalKind::terminate()) {
            sigterm.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
}

#[cfg(test)]
mod bind_addr_tests {
    use super::resolve_bind_addr_with;

    #[test]
    fn dev_mode_no_token_forces_localhost() {
        let addr = resolve_bind_addr_with("0.0.0.0:8420", true, false);
        assert_eq!(addr, "127.0.0.1:8420");
    }

    #[test]
    fn dev_mode_with_token_keeps_requested_addr() {
        // Token is set: dev-mode does NOT force localhost.
        let addr = resolve_bind_addr_with("0.0.0.0:8420", true, true);
        assert_eq!(addr, "0.0.0.0:8420");
    }

    #[test]
    fn production_mode_keeps_requested_addr() {
        let addr = resolve_bind_addr_with("0.0.0.0:8420", false, false);
        assert_eq!(addr, "0.0.0.0:8420");
    }

    #[test]
    fn dev_mode_preserves_custom_port() {
        let addr = resolve_bind_addr_with("192.168.1.1:9000", true, false);
        assert_eq!(addr, "127.0.0.1:9000");
    }
}
