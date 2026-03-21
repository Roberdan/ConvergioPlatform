// handlers: Plan 634 coordination read handlers
// Agent write handlers (register/unregister/heartbeat) → handlers_ext.rs
pub use super::handlers_ext::{
    api_ipc_agents_heartbeat, api_ipc_agents_register, api_ipc_agents_unregister,
};

use super::super::state::{query_rows, ApiError, ServerState};
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

