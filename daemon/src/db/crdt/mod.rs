mod migration;
pub(crate) mod migration_helpers;
mod sync;

#[cfg(test)]
mod tests;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub use migration::mark_required_tables;
pub use sync::io_as_sql_error;

// ALL operational tables CRR-enabled for automatic row-level replication.
// Excluded: plan_versions_backup (no PK — it's a raw dump backup table)
const REQUIRED_CRDT_TABLES: [&str; 57] = [
    "agent_activity",
    "agent_runs",
    "chat_messages",
    "chat_requirements",
    "chat_sessions",
    "collector_runs",
    "conversation_logs",
    "coordinator_events",
    "daemon_config",
    "debt_items",
    "delegation_log",
    "env_vault_log",
    "file_locks",
    "file_snapshots",
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
    "merge_queue",
    "mesh_events",
    "mesh_sync_stats",
    "metrics_history",
    "nightly_job_definitions",
    "nightly_jobs",
    "notification_queue",
    "notification_triggers",
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
    "schema_metadata",
    "session_state",
    "snapshots",
    "tasks",
    "token_usage",
    "waves",
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

pub fn load_crsqlite(conn: &Connection, extension: &str) -> rusqlite::Result<()> {
    unsafe { conn.load_extension_enable()? };
    unsafe { conn.load_extension(extension, None::<&str>) }?;
    Ok(())
}
