// handlers: Plan 634 coordination + Plan 668 agent write handlers
use super::super::state::{query_rows, ApiError, ServerState};
use super::super::ws_brain::{broadcast_brain_agent_update, broadcast_brain_session_update};
use super::ensure_ipc_schema;
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

// --- Plan 634: Coordination handlers ---

pub async fn api_ipc_agents(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
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

pub async fn api_ipc_messages(
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
            "SELECT id, channel, from_agent, content, created_at
             FROM ipc_messages ORDER BY created_at DESC LIMIT ?1",
            [limit],
        )?
    } else {
        query_rows(
            &conn,
            "SELECT id, channel, from_agent, content, created_at
             FROM ipc_messages WHERE channel = ?1
             ORDER BY created_at DESC LIMIT ?2",
            rusqlite::params![channel, limit],
        )?
    };
    Ok(Json(json!({ "ok": true, "messages": rows })))
}

pub async fn api_ipc_channels(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT name, description, created_by, created_at FROM ipc_channels ORDER BY name",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "channels": rows })))
}

pub async fn api_ipc_context(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT key, value, updated_by, updated_at FROM ipc_context ORDER BY key",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "context": rows })))
}

pub async fn api_ipc_locks(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT file_path, locked_by, lock_type, acquired_at, expires_at
         FROM ipc_file_locks ORDER BY acquired_at DESC",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "locks": rows })))
}

pub async fn api_ipc_worktrees(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT path, plan_id, branch, owner_agent, status, created_at
         FROM ipc_worktrees ORDER BY created_at DESC",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "worktrees": rows })))
}

pub async fn api_ipc_conflicts(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT file_path, locked_by, lock_type
         FROM ipc_file_locks
         WHERE expires_at IS NULL OR expires_at > strftime('%Y-%m-%dT%H:%M:%f','now')
         ORDER BY acquired_at DESC",
        [],
    )?;
    Ok(Json(json!({ "ok": true, "conflicts": rows })))
}

pub async fn api_ipc_status(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let conn = &conn;

    let agent_count = query_rows(conn, "SELECT COUNT(*) as c FROM ipc_agents", [])?
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
            SELECT file_path FROM ipc_file_locks
            GROUP BY file_path HAVING COUNT(DISTINCT locked_by) > 1
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
pub struct SendMessage {
    channel: Option<String>,
    content: String,
    sender_name: String,
}

pub async fn api_ipc_send(
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
        "INSERT INTO ipc_messages(id, channel, from_agent, content) VALUES (
             lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(6))),
             ?1, ?2, ?3)",
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

#[derive(Deserialize)]
pub struct RegisterAgent {
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
pub struct UnregisterAgent {
    agent_id: String,
    host: String,
}

#[derive(Deserialize)]
pub struct HeartbeatAgent {
    agent_id: String,
    host: String,
    current_task: Option<String>,
}

pub async fn api_ipc_agents_register(
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
            body.agent_id,
            body.host,
            body.agent_type,
            body.pid,
            body.metadata
        ],
    )
    .map_err(|e| ApiError::internal(format!("agent register failed: {e}")))?;

    let _ = state.ws_tx.send(json!({
        "type": "agent_registered",
        "agent_id": body.agent_id,
        "host": body.host,
    }));

    // Push live agent list + session state to brain viz
    broadcast_brain_agent_update(&state);
    broadcast_brain_session_update(&state);

    Ok(Json(json!({ "ok": true, "agent_id": body.agent_id })))
}

pub async fn api_ipc_agents_unregister(
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

    // Push updated agent list + session state to brain viz
    broadcast_brain_agent_update(&state);
    broadcast_brain_session_update(&state);

    Ok(Json(json!({ "ok": true })))
}

pub async fn api_ipc_agents_heartbeat(
    State(state): State<ServerState>,
    Json(body): Json<HeartbeatAgent>,
) -> Result<Json<Value>, ApiError> {
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    // Update last_seen timestamp; store current_task in metadata JSON
    let metadata = body
        .current_task
        .as_deref()
        .map(|t| serde_json::json!({"current_task": t}).to_string());
    conn.execute(
        "UPDATE ipc_agents SET last_seen = strftime('%Y-%m-%dT%H:%M:%f','now'),
         metadata = COALESCE(?3, metadata)
         WHERE name = ?1 AND host = ?2",
        rusqlite::params![body.agent_id, body.host, metadata],
    )
    .map_err(|e| ApiError::internal(format!("agent heartbeat failed: {e}")))?;

    Ok(Json(json!({ "ok": true })))
}
