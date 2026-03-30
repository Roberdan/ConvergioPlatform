// handlers_ext2: IPC send-direct handler.
use super::super::state::{ApiError, ServerState};
use super::ensure_ipc_schema;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct SendDirectMessage {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) content: String,
}

/// POST /api/ipc/send-direct — send a message to a named session/agent.
pub async fn api_ipc_send_direct(
    State(state): State<ServerState>,
    Json(body): Json<SendDirectMessage>,
) -> Result<Json<Value>, ApiError> {
    if let Some(ref ipc) = state.ipc_engine {
        ipc.send_message(&body.from, &body.to, &body.content, "direct", 0)
            .map_err(|e| ApiError::internal(format!("ipc send_direct failed: {e}")))?;
    } else {
        ensure_ipc_schema(&state)?;
        let conn = state.get_conn()?;
        conn.execute(
            "INSERT INTO ipc_messages(id, channel, from_agent, to_agent, content) VALUES (
                 lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(6))),
                 'direct', ?1, ?2, ?3)",
            rusqlite::params![body.from, body.to, body.content],
        ).map_err(|e| ApiError::internal(format!("direct message insert failed: {e}")))?;
    }

    if let Err(e) = state.ws_tx.send(json!({
        "type": "ipc_direct_message",
        "from": body.from,
        "to": body.to,
        "content": body.content,
    })) {
        tracing::debug!("ws direct_message (no subscribers): {e}");
    }
    Ok(Json(json!({ "ok": true })))
}
