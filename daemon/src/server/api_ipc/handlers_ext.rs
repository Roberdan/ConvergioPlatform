// handlers_ext: Plan 668 agent write handlers (register/unregister/heartbeat)
use super::super::state::{ApiError, ServerState};
use super::super::ws_brain::{broadcast_brain_agent_update, broadcast_brain_session_update};
use super::ensure_ipc_schema;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

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
