mod migration;
#[cfg_attr(not(feature = "crsqlite"), allow(dead_code))]
pub(crate) mod migration_helpers;
mod sync;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "crdt_feature_tests.rs"]
mod crdt_feature_tests;

#[cfg(feature = "crsqlite")]
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[cfg(feature = "crsqlite")]
pub use migration::mark_required_tables;
pub use sync::io_as_sql_error;

// Operational tables CRR-enabled for automatic row-level replication.
// Only tables with active INSERT/SELECT in production code.
// Removed 11 dead tables (zero production usage): conversation_logs,
// file_snapshots, collector_runs, debt_items, env_vault_log, merge_queue,
// metrics_history, notification_triggers, schema_metadata, session_state,
// snapshots. Cleaned 23 Marzo 2026 after DB audit (Plan 706).
const REQUIRED_CRDT_TABLES: [&str; 51] = [
    "agent_activity",
    "agent_runs",
    "audit_log",
    "chat_messages",
    "chat_sessions",
    "coordinator_events",
    "daemon_config",
    "delegation_log",
    "domain_skill_map",
    "execution_runs",
    "github_events",
    "host_heartbeats",
    "idea_notes",
    "ideas",
    "ipc_agent_skills",
    "ipc_agents",
    "ipc_auth_tokens",
    "ipc_budget_log",
    "ipc_channels",
    "ipc_file_locks",
    "ipc_messages",
    "ipc_model_registry",
    "ipc_node_capabilities",
    "ipc_shared_context",
    "ipc_subscriptions",
    "ipc_worktrees",
    "knowledge_base",
    "mesh_events",
    "mesh_sync_stats",
    "nightly_job_definitions",
    "nightly_jobs",
    "notification_queue",
    "notifications",
    "peer_heartbeats",
    "plan_actuals",
    "plan_approvals",
    "plan_business_assessments",
    "plan_commits",
    "plan_learnings",
    "plan_reviews",
    "plan_token_estimates",
    "plan_versions",
    "plans",
    "projects",
    "tasks",
    "token_usage",
    "waves",
    "workspace_events",
    "workspaces",
    "earned_skills",
    "solve_sessions",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrdtChange {
    pub table_name: String,
    pub pk: String,
    pub cid: String,
    pub val: Option<String>,
    pub col_version: i64,
    pub db_version: i64,
    pub site_id: String,
    pub cl: i64,
    pub seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SyncSummary {
    pub peer: String,
    pub sent: usize,
    pub received: usize,
    pub applied: usize,
}

pub fn required_crdt_tables() -> Vec<&'static str> {
    REQUIRED_CRDT_TABLES.to_vec()
}

/// Load the crsqlite extension into a connection.
///
/// Only available when the `crsqlite` feature is enabled. When disabled,
/// the timestamp-based `libsql_adapter` module handles replication instead.
#[cfg(feature = "crsqlite")]
pub fn load_crsqlite(conn: &Connection, extension: &str) -> rusqlite::Result<()> {
    unsafe { conn.load_extension_enable()? };
    unsafe { conn.load_extension(extension, None::<&str>) }?;
    Ok(())
}
