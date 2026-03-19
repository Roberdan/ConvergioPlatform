// SSE delegation helpers: event builders, task status updates, WS broadcast.

use crate::server::state::ServerState;
use crate::server::ws_brain::broadcast_brain_task_update;
use axum::response::sse::Event;
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;

pub(super) type Events = Vec<Result<Event, Infallible>>;

pub(super) fn broadcast_ws(state: &ServerState, qs: &HashMap<String, String>, status: &str) {
    if let Some(tid) = qs.get("task_id").filter(|v| !v.is_empty()) {
        if let Ok(id) = tid.parse::<i64>() {
            broadcast_brain_task_update(state, id, status);
        }
    }
}

pub(super) fn build_agent_command(
    cli: &str,
    plan_id: &str,
    qs: &HashMap<String, String>,
) -> String {
    let task_id = qs.get("task_id").cloned().unwrap_or_default();
    let wave_id = qs.get("wave_id").cloned().unwrap_or_default();
    let dir = "~/GitHub/ConvergioPlatform";
    match cli {
        "claude" | "copilot" => {
            let mut cmd = format!(
                "cd {dir} && claude --dangerously-skip-permissions -p 'Execute plan {plan_id}"
            );
            if !task_id.is_empty() {
                cmd.push_str(&format!(" task {task_id}"));
            }
            if !wave_id.is_empty() {
                cmd.push_str(&format!(" wave {wave_id}"));
            }
            cmd.push('\'');
            cmd
        }
        _ => format!("cd {dir} && {cli} --plan {plan_id}"),
    }
}

pub(super) fn stage(s: &str, peer: &str, detail: &str) -> serde_json::Value {
    json!({"type": "stage", "stage": s, "peer": peer, "detail": detail})
}

pub(super) fn push(events: &mut Events, event_type: &str, data: &serde_json::Value) {
    events.push(Ok(Event::default()
        .event(event_type)
        .data(data.to_string())));
}

pub(super) fn update_task_status(state: &ServerState, qs: &HashMap<String, String>, status: &str) {
    let task_id = match qs.get("task_id").filter(|v| !v.is_empty()) {
        Some(id) => id,
        None => return,
    };
    let plan_id = match qs.get("plan_id").filter(|v| !v.is_empty()) {
        Some(id) => id,
        None => return,
    };
    if let Ok(conn) = state.get_conn() {
        if let Err(e) = conn.execute(
            "UPDATE tasks SET status=?1 WHERE plan_id=?2 AND id=?3",
            &[status, plan_id.as_str(), task_id.as_str()],
        ) {
            tracing::warn!("delegate task status update failed: {e}");
        }
    }
}
