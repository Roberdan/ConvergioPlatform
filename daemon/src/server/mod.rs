pub mod api_agent_catalog;
mod api_agent_catalog_parse;
mod api_agent_catalog_security;
mod api_plan_db_evidence;
pub mod api_agent_triage;
pub mod api_agents;
pub mod api_build_exec;
pub mod api_audit;
pub mod api_capabilities;
pub mod api_channels;
pub mod api_delegation;
pub mod api_chat;
pub mod api_coordinator;
pub mod api_crdt;
pub mod api_sync;
pub mod api_dashboard;
pub mod api_digest;
pub mod api_deliverables;
pub mod api_deliverables_handlers;
pub mod api_domain;
pub mod api_evolution;
pub mod api_github;
pub mod api_github_handlers;
pub mod api_health_deep;
pub mod api_heartbeat;
pub mod api_heartbeat_handlers;
pub mod api_ideas;
pub mod api_ideas_handlers;
pub mod api_ingest;
pub mod api_ipc;
pub mod api_kernel_audio;
pub mod api_voice;
pub mod api_memory;
pub mod api_memory_mgmt;
pub mod api_memory_mgmt_gc;
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
pub mod api_plan_db_execution_context;
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
pub mod api_node_readiness;
pub mod api_node_roles;
pub mod api_agent_control;
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
pub mod provider;
pub mod api_inference_status;
pub mod routes;
pub mod sse;
pub mod sse_chat;
pub mod sse_delegate;
pub mod sse_preflight;
pub mod sse_stream;
pub mod state;
pub mod state_init;
pub mod telemetry;
pub mod state_init_canon;
mod state_init_migrations;
pub mod static_serve;
pub mod ws;
pub mod ws_brain;
pub mod ws_pty;

pub mod api_decisions;
pub mod api_repositories;

#[cfg(test)]
mod api_memory_mgmt_tests;
#[cfg(test)]
mod api_voice_tests;
#[cfg(test)]
mod api_agent_catalog_tests;
#[cfg(test)]
mod api_agents_brain_tests;
#[cfg(test)]
mod api_channels_tests;
#[cfg(test)]
mod api_health_deep_tests;
#[cfg(test)]
mod api_agents_legacy_tests;
#[cfg(test)]
mod api_agents_tests;
#[cfg(test)]
mod api_cli_integration_tests;
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
mod api_ipc_integration_tests;
#[cfg(test)]
mod api_ipc_integration_tests2;
#[cfg(test)]
mod api_ipc_intel_tests;
#[cfg(test)]
mod api_metrics_tests;
#[cfg(test)]
mod api_openclaw_tests;
#[cfg(test)]
mod api_peers_tests;
#[cfg(test)]
mod api_plan_db_checkpoint_tests;
#[cfg(test)]
mod api_plan_db_execution_context_tests;
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
mod api_delegation_tests;
#[cfg(test)]
mod api_delegation_unit_tests;
#[cfg(test)]
mod api_workspace_tests;
#[cfg(test)]
mod api_agent_control_tests;
#[cfg(test)]
mod api_capabilities_tests;
#[cfg(test)]
mod api_capabilities_tests2;
#[cfg(test)]
mod api_nightly_tests;
#[cfg(test)]
mod api_nightly_tests2;
#[cfg(test)]
mod api_telemetry_tests;
#[cfg(test)]
mod api_plan_db_lifecycle_integration_tests;
#[cfg(test)]
mod api_plan_db_review_integration_tests;
#[cfg(test)]
mod api_plan_db_review_integration_tests2;
#[cfg(test)]
mod api_plan_db_import_integration_tests;
#[cfg(test)]
mod state_init_tests;
#[cfg(test)]
mod ws_pty_tests;
#[cfg(test)]
mod api_cli_integration_tests_ipc;

use axum::Router;
use std::path::{Path, PathBuf};

pub use static_serve::resolve_bind_addr;

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
) -> Result<(), state::ApiError> {
    let router = app(static_dir, crsqlite_path);
    static_serve::run_router(bind_addr, router).await
}

/// Run HTTP server with a pre-configured ServerState (shared IPC engine).
pub async fn run_with_state(
    bind_addr: &str,
    static_dir: impl Into<PathBuf>,
    server_state: state::ServerState,
) -> Result<(), state::ApiError> {
    let router = routes::build_router_with_state(static_dir.into(), server_state);
    static_serve::run_router(bind_addr, router).await
}
