use super::super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct AskBody {
    from: String,
    to: String,
    message: String,
    timeout_secs: Option<u64>,
}

pub async fn api_ipc_ask(
    State(state): State<ServerState>,
    Json(body): Json<AskBody>,
) -> Result<Json<Value>, ApiError> {
    let ipc = state
        .ipc_engine
        .as_ref()
        .ok_or_else(|| ApiError::internal("ipc engine unavailable"))?;
    ipc.send_message(&body.from, &body.to, &body.message, "direct", 0)
        .map_err(|e| ApiError::internal(format!("ask send failed: {e}")))?;
    let timeout = body.timeout_secs.unwrap_or(120);
    let recv = ipc
        .receive_wait(&body.from, Some(&body.to), None, 1, timeout)
        .await
        .map_err(|e| ApiError::internal(format!("ask receive failed: {e}")))?;
    let crate::ipc::IpcResponse::MessageList { messages } = recv else {
        return Err(ApiError::internal("ask unexpected IPC response"));
    };
    if let Some(reply) = messages.first() {
        return Ok(Json(json!({
            "ok": true,
            "reply": {
                "from": reply.from_agent,
                "to": reply.to_agent,
                "content": reply.content,
                "ts": reply.created_at
            }
        })));
    }
    Ok(Json(json!({
        "ok": false,
        "error": {
            "code": "TIMEOUT",
            "message": format!("No reply from {} within {}s", body.to, timeout),
        }
    })))
}
