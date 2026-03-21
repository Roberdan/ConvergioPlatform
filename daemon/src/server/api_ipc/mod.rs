// api_ipc: IPC coordination and intelligence endpoints
mod handlers;
mod handlers_ext;
mod routes;

use super::state::ServerState;
use axum::routing::{get, post};
use axum::Router;
use std::collections::VecDeque;
use std::sync::Mutex;

// --- IPC schema aligned with actual DB (Plan 633 schema.rs + CRDT) ---
const IPC_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS ipc_agents (
    name        TEXT NOT NULL,
    host        TEXT NOT NULL,
    agent_type  TEXT NOT NULL DEFAULT 'claude',
    pid         INTEGER,
    metadata    TEXT,
    registered_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    last_seen   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    PRIMARY KEY (name, host)
);
CREATE TABLE IF NOT EXISTS ipc_messages (
    id TEXT PRIMARY KEY NOT NULL,
    from_agent TEXT NOT NULL DEFAULT '',
    to_agent TEXT,
    channel TEXT,
    content TEXT NOT NULL DEFAULT '',
    msg_type TEXT NOT NULL DEFAULT 'text',
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    read_at TEXT
);
CREATE TABLE IF NOT EXISTS ipc_channels (
    name TEXT PRIMARY KEY NOT NULL,
    description TEXT,
    created_by TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now'))
);
CREATE TABLE IF NOT EXISTS ipc_context (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_by TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS ipc_file_locks (
    file_path TEXT PRIMARY KEY NOT NULL,
    locked_by TEXT NOT NULL DEFAULT '',
    lock_type TEXT NOT NULL DEFAULT 'exclusive',
    acquired_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now')),
    expires_at TEXT
);
CREATE TABLE IF NOT EXISTS ipc_worktrees (
    path TEXT PRIMARY KEY NOT NULL,
    plan_id INTEGER,
    branch TEXT,
    owner_agent TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now'))
);
CREATE INDEX IF NOT EXISTS idx_ipc_messages_channel ON ipc_messages(channel, created_at DESC);
";

pub fn ensure_ipc_schema(state: &ServerState) -> Result<(), super::state::ApiError> {
    let conn = state.get_conn()?;
    conn.execute_batch(IPC_SCHEMA)
        .map_err(|err| super::state::ApiError::internal(format!("ipc schema init failed: {err}")))?;
    // Drop CRDT triggers on IPC tables — IPC is local-only, not replicated
    let _ = conn.execute_batch(
        "DROP TRIGGER IF EXISTS ipc_agents__crsql_itrig;
         DROP TRIGGER IF EXISTS ipc_agents__crsql_utrig;
         DROP TRIGGER IF EXISTS ipc_agents__crsql_dtrig;
         DROP TRIGGER IF EXISTS ipc_messages__crsql_itrig;
         DROP TRIGGER IF EXISTS ipc_messages__crsql_utrig;
         DROP TRIGGER IF EXISTS ipc_messages__crsql_dtrig;
         DROP TRIGGER IF EXISTS ipc_context__crsql_itrig;
         DROP TRIGGER IF EXISTS ipc_context__crsql_utrig;
         DROP TRIGGER IF EXISTS ipc_context__crsql_dtrig;
         DROP TRIGGER IF EXISTS ipc_channels__crsql_itrig;
         DROP TRIGGER IF EXISTS ipc_channels__crsql_utrig;
         DROP TRIGGER IF EXISTS ipc_channels__crsql_dtrig;
         DROP TRIGGER IF EXISTS ipc_file_locks__crsql_itrig;
         DROP TRIGGER IF EXISTS ipc_file_locks__crsql_utrig;
         DROP TRIGGER IF EXISTS ipc_file_locks__crsql_dtrig;
         DROP TRIGGER IF EXISTS ipc_worktrees__crsql_itrig;
         DROP TRIGGER IF EXISTS ipc_worktrees__crsql_utrig;
         DROP TRIGGER IF EXISTS ipc_worktrees__crsql_dtrig;",
    );
    Ok(())
}

// --- Plan 635: Intelligence log buffer ---

const LOG_BUFFER_MAX: usize = 1000;

pub struct IpcLogEntry {
    pub timestamp: String,
    pub level: String,
    pub module: String,
    pub message: String,
}

pub static IPC_LOG_BUFFER: Mutex<Option<VecDeque<IpcLogEntry>>> = Mutex::new(None);

pub fn ipc_log(level: &str, module: &str, message: &str) {
    let entry = IpcLogEntry {
        timestamp: format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ),
        level: level.to_string(),
        module: module.to_string(),
        message: message.to_string(),
    };
    if let Ok(mut guard) = IPC_LOG_BUFFER.lock() {
        let buf = guard.get_or_insert_with(|| VecDeque::with_capacity(LOG_BUFFER_MAX));
        if buf.len() >= LOG_BUFFER_MAX {
            buf.pop_front();
        }
        buf.push_back(entry);
    }
}

// --- Combined router ---

pub fn router() -> Router<ServerState> {
    Router::new()
        // Plan 634: Coordination endpoints
        .route("/api/ipc/agents", get(handlers::api_ipc_agents))
        .route("/api/ipc/messages", get(handlers::api_ipc_messages))
        .route("/api/ipc/channels", get(handlers::api_ipc_channels))
        .route("/api/ipc/context", get(handlers::api_ipc_context))
        .route("/api/ipc/locks", get(handlers::api_ipc_locks))
        .route("/api/ipc/worktrees", get(handlers::api_ipc_worktrees))
        .route("/api/ipc/conflicts", get(handlers::api_ipc_conflicts))
        .route("/api/ipc/status", get(handlers::api_ipc_status))
        .route("/api/ipc/send", post(handlers::api_ipc_send))
        // Plan 668: Agent write endpoints
        .route("/api/ipc/agents/register", post(handlers::api_ipc_agents_register))
        .route("/api/ipc/agents/unregister", post(handlers::api_ipc_agents_unregister))
        .route("/api/ipc/agents/heartbeat", post(handlers::api_ipc_agents_heartbeat))
        // Plan 635: Intelligence endpoints
        .route("/api/ipc/budget", get(routes::api_ipc_budget))
        .route("/api/ipc/models", get(routes::api_ipc_models))
        .route("/api/ipc/skills", get(routes::api_ipc_skills))
        .route("/api/ipc/auth-status", get(routes::api_ipc_auth_status))
        .route("/api/ipc/route-history", get(routes::api_ipc_route_history))
        .route("/api/ipc/metrics", get(routes::api_ipc_metrics))
        .route("/api/ipc/logs", get(routes::api_ipc_logs))
}
