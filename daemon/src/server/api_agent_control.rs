// Agent control — interrupt blocked agents, reschedule tasks to other nodes.
// The kernel uses these to intervene when it detects stalls or failures.

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;

use super::state::ServerState;

#[derive(Debug, Deserialize)]
pub struct InterruptRequest {
    pub agent_name: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct RescheduleRequest {
    pub task_id: i64,
    pub from_node: String,
    pub to_node: String,
    pub reason: String,
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/agent/interrupt", post(interrupt_agent))
        .route("/api/task/reschedule", post(reschedule_task))
}

/// Interrupt a blocked/stalled agent via IPC bus message.
/// Sends INTERRUPT message to the agent's IPC channel.
async fn interrupt_agent(
    State(state): State<ServerState>,
    Json(body): Json<InterruptRequest>,
) -> Json<serde_json::Value> {
    let conn = match state.get_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    };
    // Record interrupt event
    let _ = conn.execute(
        "INSERT INTO ipc_messages (sender, recipient, content, msg_type, created_at)
         VALUES ('kernel', ?1, ?2, 'INTERRUPT', datetime('now'))",
        rusqlite::params![body.agent_name, body.reason],
    );
    // Mark agent as interrupted in agent_activity
    let updated = conn.execute(
        "UPDATE agent_activity SET status='interrupted', notes=?2
         WHERE agent_name=?1 AND status='running'",
        rusqlite::params![body.agent_name, body.reason],
    ).unwrap_or(0);

    tracing::info!(
        agent = body.agent_name,
        reason = body.reason,
        "agent interrupted via kernel"
    );
    Json(serde_json::json!({
        "ok": true,
        "agent": body.agent_name,
        "interrupted": updated > 0,
        "message": format!("INTERRUPT sent to {}", body.agent_name)
    }))
}

/// Reschedule a task from one node to another.
/// Resets task to pending and updates execution_host.
async fn reschedule_task(
    State(state): State<ServerState>,
    Json(body): Json<RescheduleRequest>,
) -> Json<serde_json::Value> {
    let conn = match state.get_conn() {
        Ok(c) => c,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    };
    // Reset task to pending
    let updated = conn.execute(
        "UPDATE tasks SET status='pending', notes=?3
         WHERE id=?1 AND status IN ('in_progress','blocked','submitted')",
        rusqlite::params![
            body.task_id,
            body.to_node,
            format!("Rescheduled from {} to {}: {}", body.from_node, body.to_node, body.reason)
        ],
    ).unwrap_or(0);
    // Log delegation
    let _ = conn.execute(
        "INSERT INTO delegation_log (plan_id, executor_agent, task_id, cost_estimate, timestamp)
         SELECT plan_id, ?2, id, 0, datetime('now') FROM tasks WHERE id=?1",
        rusqlite::params![body.task_id, format!("reschedule-{}", body.to_node)],
    );

    tracing::info!(
        task_id = body.task_id,
        from = body.from_node,
        to = body.to_node,
        "task rescheduled via kernel"
    );
    Json(serde_json::json!({
        "ok": true,
        "task_id": body.task_id,
        "rescheduled": updated > 0,
        "to_node": body.to_node
    }))
}
