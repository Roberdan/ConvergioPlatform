// Reactor — core event loop. Blocks on IPC receive_wait, dispatches to handlers.

use crate::ipc::{IpcEngine, IpcResponse, MessageInfo};
use std::path::PathBuf;
use std::sync::Arc;

use super::handlers;

pub async fn run(engine: Arc<IpcEngine>, db_path: PathBuf) {
    loop {
        let resp = engine
            .receive_wait(
                super::ALI_AGENT,
                None,
                Some(super::CHANNEL),
                10,
                300, // 5 min keepalive timeout
            )
            .await;

        match resp {
            Ok(IpcResponse::MessageList { messages }) => {
                for msg in &messages {
                    if msg.from_agent == super::ALI_AGENT {
                        continue; // skip own broadcasts
                    }
                    if let Err(e) = handle_message(&engine, &db_path, msg).await {
                        tracing::error!("ali: handler error for msg {}: {e}", msg.id);
                        emit_error(&engine, &e.to_string());
                    }
                }
            }
            Ok(_) => {} // empty on timeout, re-loop
            Err(e) => {
                tracing::error!("ali: receive_wait error: {e}, retrying in 5s");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }
}

async fn handle_message(
    engine: &Arc<IpcEngine>,
    db_path: &PathBuf,
    msg: &MessageInfo,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let payload: serde_json::Value = serde_json::from_str(&msg.content)?;
    let event_type = payload
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    tracing::info!("ali: received event={event_type} from={}", msg.from_agent);

    match event_type {
        "plan_started" | "plan_ready" => {
            let plan_id = require_i64(&payload, "plan_id")?;
            handlers::on_plan_ready(engine, db_path, plan_id).await?;
        }
        "task_done" => {
            let task_id = payload
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let plan_id = require_i64(&payload, "plan_id")?;
            handlers::on_task_done(engine, db_path, &task_id, plan_id).await?;
        }
        "wave_done" => {
            let wave_id = require_i64(&payload, "wave_id")?;
            let plan_id = require_i64(&payload, "plan_id")?;
            handlers::on_wave_done(engine, wave_id, plan_id)?;
        }
        "wave_validated" => {
            let wave_id = require_i64(&payload, "wave_id")?;
            let plan_id = require_i64(&payload, "plan_id")?;
            handlers::on_wave_validated(engine, db_path, wave_id, plan_id)?;
        }
        "plan_done" => {
            let plan_id = require_i64(&payload, "plan_id")?;
            handlers::on_plan_done(engine, db_path, plan_id)?;
        }
        "delegation_failed" => {
            let plan_id = require_i64(&payload, "plan_id")?;
            let peer = payload
                .get("peer")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let reason = payload
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            handlers::on_delegation_failed(engine, db_path, plan_id, &peer, &reason).await?;
        }
        other => {
            tracing::debug!("ali: ignoring unknown event type: {other}");
        }
    }

    Ok(())
}

fn require_i64(
    payload: &serde_json::Value,
    field: &str,
) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    payload
        .get(field)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| format!("missing or invalid field: {field}").into())
}

fn emit_error(engine: &IpcEngine, detail: &str) {
    let content = serde_json::json!({"type": "error", "detail": detail}).to_string();
    let _ = engine.broadcast(super::ALI_AGENT, &content, "error", Some(super::CHANNEL));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_i64_extracts_valid_field() {
        let payload = serde_json::json!({"plan_id": 719});
        assert_eq!(require_i64(&payload, "plan_id").unwrap(), 719);
    }

    #[test]
    fn require_i64_errors_on_missing_field() {
        let payload = serde_json::json!({"other": 1});
        assert!(require_i64(&payload, "plan_id").is_err());
    }

    #[test]
    fn require_i64_errors_on_string_value() {
        let payload = serde_json::json!({"plan_id": "not_a_number"});
        assert!(require_i64(&payload, "plan_id").is_err());
    }
}
