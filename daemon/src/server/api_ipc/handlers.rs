// handlers: Plan 634 coordination read handlers
// Agent write handlers (register/unregister/heartbeat/list/deregister) → handlers_ext.rs
pub use super::handlers_ext::{
    api_ipc_agents_deregister, api_ipc_agents_heartbeat, api_ipc_agents_list,
    api_ipc_agents_register, api_ipc_agents_tree, api_ipc_agents_unregister,
};
pub use super::handlers_ext2::api_ipc_send_direct;

use super::super::state::{query_rows, ApiError, ServerState};
use super::super::ws_brain::broadcast_brain_message_event;
use super::super::ws_brain_org::{broadcast_org_message, broadcast_org_topology};
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
        "SELECT name, host, agent_type, pid, metadata, parent_agent, registered_at, last_seen
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
        .and_then(|v| v.parse::<i64>().ok()) // intentional: invalid limit falls back to safe default
        .unwrap_or(50);

    let to_agent = qs.get("to_agent").cloned().unwrap_or_default();
    let rows = match (channel.is_empty(), to_agent.is_empty()) {
        (true, true) => query_rows(
            &conn,
            "SELECT id, channel, from_agent, to_agent, content, created_at
             FROM ipc_messages ORDER BY created_at DESC LIMIT ?1",
            [limit],
        )?,
        (false, true) => query_rows(
            &conn,
            "SELECT id, channel, from_agent, to_agent, content, created_at
             FROM ipc_messages WHERE channel = ?1
             ORDER BY created_at DESC LIMIT ?2",
            rusqlite::params![channel, limit],
        )?,
        (true, false) => query_rows(
            &conn,
            "SELECT id, channel, from_agent, to_agent, content, created_at
             FROM ipc_messages WHERE to_agent = ?1 OR to_agent IS NULL
             ORDER BY created_at DESC LIMIT ?2",
            rusqlite::params![to_agent, limit],
        )?,
        (false, false) => query_rows(
            &conn,
            "SELECT id, channel, from_agent, to_agent, content, created_at
             FROM ipc_messages WHERE channel = ?1
             AND (to_agent = ?2 OR to_agent IS NULL)
             ORDER BY created_at DESC LIMIT ?3",
            rusqlite::params![channel, to_agent, limit],
        )?,
    };
    Ok(Json(json!({ "ok": true, "messages": rows })))
}

pub async fn api_ipc_channels(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
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
    // Use ipc_shared_context — the canonical context table managed by IpcEngine.
    // The former ipc_context table (api_ipc/mod.rs) has been removed; all context
    // reads/writes must go through ipc_shared_context to stay consistent.
    ensure_ipc_schema(&state)?;
    let conn = state.get_conn()?;
    let rows = query_rows(
        &conn,
        "SELECT key, value, version, set_by, updated_at FROM ipc_shared_context ORDER BY key",
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

pub async fn api_ipc_worktrees(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
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

pub async fn api_ipc_conflicts(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
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
    let channel = body.channel.as_deref().unwrap_or("general");

    // Use the shared IPC engine for consistent schema + Notify wake
    if let Some(ref ipc) = state.ipc_engine {
        ipc.broadcast(&body.sender_name, &body.content, "event", Some(channel))
            .map_err(|e| ApiError::internal(format!("ipc broadcast failed: {e}")))?;
    } else {
        // Fallback: direct DB write (no Notify, legacy path)
        ensure_ipc_schema(&state)?;
        let conn = state.get_conn()?;
        conn.execute(
            "INSERT INTO ipc_channels(name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            rusqlite::params![channel],
        ).map_err(|e| ApiError::internal(format!("channel upsert failed: {e}")))?;
        conn.execute(
            "INSERT INTO ipc_messages(id, channel, from_agent, content) VALUES (
                 lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-' || lower(hex(randomblob(6))),
                 ?1, ?2, ?3)",
            rusqlite::params![channel, body.sender_name, body.content],
        ).map_err(|e| ApiError::internal(format!("message insert failed: {e}")))?;
    }

    if let Err(e) = state.ws_tx.send(json!({
        "type": "ipc_message",
        "channel": channel,
        "sender": body.sender_name,
        "content": body.content,
    })) {
        tracing::debug!("ws ipc_message broadcast (no subscribers): {e}");
    }
    broadcast_brain_message_event(&state, &body.sender_name, channel, &body.content);
    if channel.starts_with("org:") || channel.starts_with("inter-org:") {
        broadcast_org_message(&state, channel, &body.sender_name, &body.content);
        if channel.starts_with("inter-org:") {
            broadcast_org_topology(&state);
        }
    }

    Ok(Json(json!({ "ok": true })))
}
