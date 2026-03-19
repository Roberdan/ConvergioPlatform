use super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Mutex;

// --- Plan 634: Coordination schema (inline fallback) ---

// Schema matches daemon/src/ipc/schema.rs (Plan 633) — ipc_agents uses (name, host) PK
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
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    channel TEXT NOT NULL DEFAULT 'general',
    sender TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS ipc_channels (
    name TEXT PRIMARY KEY,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS ipc_context (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_by TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS ipc_file_locks (
    file_pattern TEXT NOT NULL,
    agent TEXT NOT NULL,
    host TEXT NOT NULL,
    pid INTEGER NOT NULL,
    locked_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (file_pattern, agent, host)
);
CREATE TABLE IF NOT EXISTS ipc_worktrees (
    agent TEXT NOT NULL,
    host TEXT NOT NULL,
    branch TEXT NOT NULL,
    path TEXT NOT NULL,
    registered_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (agent, host)
);
CREATE INDEX IF NOT EXISTS idx_ipc_messages_channel ON ipc_messages(channel, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ipc_file_locks_pattern ON ipc_file_locks(file_pattern);
";

fn ensure_ipc_schema(state: &ServerState) -> Result<(), ApiError> {
    let conn = state.get_conn()?;
    conn.execute_batch(IPC_SCHEMA)
        .map_err(|err| ApiError::internal(format!("ipc schema init failed: {err}")))?;
    Ok(())
}

// --- Plan 635: Intelligence log buffer ---

const LOG_BUFFER_MAX: usize = 1000;

struct IpcLogEntry {
    timestamp: String,
    level: String,
    module: String,
    message: String,
}

static IPC_LOG_BUFFER: Mutex<Option<VecDeque<IpcLogEntry>>> = Mutex::new(None);

fn log_buffer() -> &'static Mutex<Option<VecDeque<IpcLogEntry>>> {
    &IPC_LOG_BUFFER
}

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
    if let Ok(mut guard) = log_buffer().lock() {
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
        .route("/api/ipc/agents", get(api_ipc_agents))
        .route("/api/ipc/messages", get(api_ipc_messages))
        .route("/api/ipc/channels", get(api_ipc_channels))
        .route("/api/ipc/context", get(api_ipc_context))
        .route("/api/ipc/locks", get(api_ipc_locks))
        .route("/api/ipc/worktrees", get(api_ipc_worktrees))
        .route("/api/ipc/conflicts", get(api_ipc_conflicts))
        .route("/api/ipc/status", get(api_ipc_status))
        .route("/api/ipc/send", post(api_ipc_send))
        // Plan 668: Agent write endpoints
        .route("/api/ipc/agents/register", post(api_ipc_agents_register))
        .route("/api/ipc/agents/unregister", post(api_ipc_agents_unregister))
        .route("/api/ipc/agents/heartbeat", post(api_ipc_agents_heartbeat))
        // Plan 635: Intelligence endpoints
        .route("/api/ipc/budget", get(api_ipc_budget))
        .route("/api/ipc/models", get(api_ipc_models))
        .route("/api/ipc/skills", get(api_ipc_skills))
        .route("/api/ipc/auth-status", get(api_ipc_auth_status))
        .route("/api/ipc/route-history", get(api_ipc_route_history))
        .route("/api/ipc/metrics", get(api_ipc_metrics))
        .route("/api/ipc/logs", get(api_ipc_logs))
}

// --- Plan 634: Coordination handlers ---

async fn api_ipc_agents(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT name, host, agent_type, pid, metadata, registered_at, last_seen
         FROM ipc_agents ORDER BY last_seen DESC",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "agents": rows })))
}

async fn api_ipc_messages(
    State(state): State<ServerState>,
    Query(qs): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let channel = qs.get("channel").cloned().unwrap_or_default();
    let limit = qs
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(50);

    let rows = if channel.is_empty() {
        query_rows(
            &conn,
            "SELECT id, channel, sender, content, created_at
             FROM ipc_messages ORDER BY created_at DESC LIMIT ?1",
            [limit],
        )?
    } else {
        query_rows(
            &conn,
            "SELECT id, channel, sender, content, created_at
             FROM ipc_messages WHERE channel = ?1
             ORDER BY created_at DESC LIMIT ?2",
            rusqlite::params![channel, limit],
        )?
    };
    Ok(Json(json!({ "ok": true, "messages": rows })))
}

async fn api_ipc_channels(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT name, description, created_at FROM ipc_channels ORDER BY name",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "channels": rows })))
}

async fn api_ipc_context(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT key, value, updated_by, updated_at FROM ipc_context ORDER BY key",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "context": rows })))
}

async fn api_ipc_locks(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT file_pattern, agent, host, pid, locked_at
         FROM ipc_file_locks ORDER BY locked_at DESC",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "locks": rows })))
}

async fn api_ipc_worktrees(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT agent, host, branch, path, registered_at
         FROM ipc_worktrees ORDER BY registered_at DESC",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "worktrees": rows })))
}

async fn api_ipc_conflicts(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT file_pattern, GROUP_CONCAT(DISTINCT agent) as agents,
                COUNT(DISTINCT agent) as agent_count
         FROM ipc_file_locks
         GROUP BY file_pattern
         HAVING agent_count > 1
         ORDER BY agent_count DESC",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "conflicts": rows })))
}

async fn api_ipc_status(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let conn = &conn;

    let agent_count = query_rows(
        conn,
        "SELECT COUNT(*) as c FROM ipc_agents",
        [],
    )?
    .first()
    .and_then(|v| v.get("c"))
    .and_then(Value::as_i64)
    .unwrap_or(0);
    let lock_count = query_rows(conn, "SELECT COUNT(*) as c FROM ipc_file_locks", [])?
        .first()
        .and_then(|v| v.get("c"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let message_count = query_rows(conn, "SELECT COUNT(*) as c FROM ipc_messages", [])?
        .first()
        .and_then(|v| v.get("c"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let conflict_count = query_rows(
        conn,
        "SELECT COUNT(*) as c FROM (
            SELECT file_pattern FROM ipc_file_locks
            GROUP BY file_pattern HAVING COUNT(DISTINCT agent) > 1
        )",
        [],
    )?
    .first()
    .and_then(|v| v.get("c"))
    .and_then(Value::as_i64)
    .unwrap_or(0);

    Ok(Json(json!({
        "ok": true,
        "agents_active": agent_count,
        "locks_active": lock_count,
        "messages_total": message_count,
        "conflicts": conflict_count,
    })))
}

#[derive(Deserialize)]
struct SendMessage {
    channel: Option<String>,
    content: String,
    sender_name: String,
}

#[derive(Deserialize)]
struct RegisterAgent {
    agent_id: String,
    host: String,
    #[serde(default = "default_agent_type")]
    agent_type: String,
    pid: Option<i64>,
    metadata: Option<String>,
}

fn default_agent_type() -> String {
    "claude".into()
}

#[derive(Deserialize)]
struct UnregisterAgent {
    agent_id: String,
    host: String,
}

#[derive(Deserialize)]
struct HeartbeatAgent {
    agent_id: String,
    host: String,
    current_task: Option<String>,
}

async fn api_ipc_send(
    State(state): State<ServerState>,
    Json(body): Json<SendMessage>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let channel = body.channel.as_deref().unwrap_or("general");

    conn.execute(
        "INSERT INTO ipc_channels(name) VALUES (?1)
             ON CONFLICT(name) DO NOTHING",
        rusqlite::params![channel],
    )
    .map_err(|e| ApiError::internal(format!("channel upsert failed: {e}")))?;

    conn.execute(
        "INSERT INTO ipc_messages(channel, sender, content) VALUES (?1, ?2, ?3)",
        rusqlite::params![channel, body.sender_name, body.content],
    )
    .map_err(|e| ApiError::internal(format!("message insert failed: {e}")))?;

    let _ = state.ws_tx.send(json!({
        "type": "ipc_message",
        "channel": channel,
        "sender": body.sender_name,
        "content": body.content,
    }));

    Ok(Json(json!({ "ok": true })))
}

// --- Plan 668: Agent write handlers ---

async fn api_ipc_agents_register(
    State(state): State<ServerState>,
    Json(body): Json<RegisterAgent>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    conn.execute(
        "INSERT OR REPLACE INTO ipc_agents
         (name, host, agent_type, pid, metadata, registered_at, last_seen)
         VALUES (?1, ?2, ?3, ?4, ?5,
                 strftime('%Y-%m-%dT%H:%M:%f','now'),
                 strftime('%Y-%m-%dT%H:%M:%f','now'))",
        rusqlite::params![
            body.agent_id, body.host, body.agent_type, body.pid, body.metadata
        ],
    )
    .map_err(|e| ApiError::internal(format!("agent register failed: {e}")))?;

    let _ = state.ws_tx.send(json!({
        "type": "agent_registered",
        "agent_id": body.agent_id,
        "host": body.host,
    }));

    Ok(Json(json!({ "ok": true, "agent_id": body.agent_id })))
}

async fn api_ipc_agents_unregister(
    State(state): State<ServerState>,
    Json(body): Json<UnregisterAgent>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    conn.execute(
        "DELETE FROM ipc_agents WHERE name = ?1 AND host = ?2",
        rusqlite::params![body.agent_id, body.host],
    )
    .map_err(|e| ApiError::internal(format!("agent unregister failed: {e}")))?;

    let _ = state.ws_tx.send(json!({
        "type": "agent_unregistered",
        "agent_id": body.agent_id,
        "host": body.host,
    }));

    Ok(Json(json!({ "ok": true })))
}

async fn api_ipc_agents_heartbeat(
    State(state): State<ServerState>,
    Json(body): Json<HeartbeatAgent>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    // Update last_seen timestamp; store current_task in metadata JSON
    let metadata = body.current_task.as_deref().map(|t| {
        serde_json::json!({"current_task": t}).to_string()
    });
    conn.execute(
        "UPDATE ipc_agents SET last_seen = strftime('%Y-%m-%dT%H:%M:%f','now'),
         metadata = COALESCE(?3, metadata)
         WHERE name = ?1 AND host = ?2",
        rusqlite::params![body.agent_id, body.host, metadata],
    )
    .map_err(|e| ApiError::internal(format!("agent heartbeat failed: {e}")))?;

    Ok(Json(json!({ "ok": true })))
}

// --- Plan 635: Intelligence handlers ---

async fn api_ipc_budget(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let subs = crate::ipc::models::list_subscriptions(conn)
        .map_err(|e| ApiError::internal(format!("subs: {e}")))?;
    let mut budgets = Vec::new();
    for s in &subs {
        let status = crate::ipc::budget::get_budget_status(conn, &s.name)
            .map_err(|e| ApiError::internal(format!("budget: {e}")))?;
        let alert = crate::ipc::budget::check_budget_thresholds(conn, &s.name)
            .map_err(|e| ApiError::internal(format!("alert: {e}")))?;
        budgets.push(json!({
            "subscription": s.name, "provider": s.provider,
            "plan": s.plan, "budget_usd": s.budget_usd,
            "status": status, "alert": alert,
        }));
    }
    Ok(Json(json!({ "budgets": budgets })))
}

async fn api_ipc_models(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let models = crate::ipc::models::get_all_models(conn)
        .map_err(|e| ApiError::internal(format!("models: {e}")))?;
    let capabilities = crate::ipc::models::get_all_capabilities(conn)
        .map_err(|e| ApiError::internal(format!("caps: {e}")))?;
    Ok(Json(
        json!({ "models": models, "capabilities": capabilities }),
    ))
}

async fn api_ipc_skills(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let pool = crate::ipc::skills::get_skill_pool(conn)
        .map_err(|e| ApiError::internal(format!("skills: {e}")))?;
    let flat: Vec<_> = pool.values().flatten().collect();
    Ok(Json(json!({ "skills": flat })))
}

async fn api_ipc_auth_status(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let health = crate::ipc::auth_sync::check_token_sync_health(conn)
        .map_err(|e| ApiError::internal(format!("health: {e}")))?;
    let tokens = crate::ipc::auth_sync::list_tokens(conn)
        .map_err(|e| ApiError::internal(format!("tokens: {e}")))?;
    Ok(Json(json!({ "health": health, "tokens": tokens })))
}

async fn api_ipc_route_history(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let mut stmt = conn
        .prepare(
            "SELECT subscription, date, tokens_in, tokens_out, estimated_cost_usd, model, task_ref
             FROM ipc_budget_log ORDER BY id DESC LIMIT 20",
        )
        .map_err(|e| ApiError::internal(format!("prepare: {e}")))?;
    let entries: Vec<Value> = stmt
        .query_map([], |row| {
            Ok(json!({
                "subscription": row.get::<_, String>(0)?,
                "date": row.get::<_, String>(1)?,
                "tokens_in": row.get::<_, i64>(2)?,
                "tokens_out": row.get::<_, i64>(3)?,
                "cost": row.get::<_, f64>(4)?,
                "model": row.get::<_, String>(5)?,
                "task": row.get::<_, String>(6)?,
            }))
        })
        .map_err(|e| ApiError::internal(format!("query: {e}")))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(Json(json!({ "history": entries })))
}

async fn api_ipc_metrics(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;
    let model_count: i64 = conn
        .query_row("SELECT count(*) FROM ipc_model_registry", [], |r| r.get(0))
        .unwrap_or(0);
    let agent_count: i64 = conn
        .query_row(
            "SELECT count(DISTINCT agent) FROM ipc_agent_skills WHERE agent != ''",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let ipc_message_rate: i64 = conn
        .query_row(
            "SELECT count(*) FROM ipc_budget_log WHERE date >= date('now', '-1 day')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let budget_usage: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(estimated_cost_usd), 0) FROM ipc_budget_log",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);
    let skill_requests_active: i64 = conn.query_row(
        "SELECT count(*) FROM session_state WHERE key LIKE 'skill_req:%' AND value LIKE '%pending%'",
        [], |r| r.get(0),
    ).unwrap_or(0);
    Ok(Json(json!({
        "ipc_message_rate_1d": ipc_message_rate,
        "agent_count": agent_count,
        "model_count": model_count,
        "avg_route_latency_ms": 0,
        "budget_usage": budget_usage,
        "skill_requests_active": skill_requests_active,
    })))
}

#[derive(Deserialize)]
struct LogsQuery {
    limit: Option<usize>,
}

async fn api_ipc_logs(Query(q): Query<LogsQuery>) -> Result<Json<Value>, ApiError> {
    let limit = q.limit.unwrap_or(100).min(1000);
    let guard = log_buffer()
        .lock()
        .map_err(|e| ApiError::internal(format!("lock: {e}")))?;
    let entries: Vec<Value> = guard
        .as_ref()
        .map(|buf| {
            buf.iter()
                .rev()
                .take(limit)
                .map(|e| {
                    json!({
                        "timestamp": e.timestamp, "level": e.level,
                        "module": e.module, "message": e.message,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Json(json!({ "logs": entries, "count": entries.len() })))
}
