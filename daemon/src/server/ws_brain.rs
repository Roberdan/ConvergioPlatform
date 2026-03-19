// Brain WS push — broadcasts real-time events to /ws/brain clients
// via the shared state.ws_tx broadcast channel.

use super::state::{query_rows, ServerState};
use serde_json::{json, Value};

/// Broadcast a brain event to all connected /ws/brain clients.
/// Failures are silently ignored (no subscribers = no-op).
pub fn broadcast_brain_event(state: &ServerState, event_type: &str, payload: Value) {
    let event = json!({
        "kind": "brain_event",
        "event_type": event_type,
        "payload": payload,
    });
    let _ = state.ws_tx.send(event);
}

/// Broadcast agent_update: current list of registered IPC agents.
/// Called after agent register/unregister to push live state to brain viz.
pub fn broadcast_brain_agent_update(state: &ServerState) {
    let agents = match state.get_conn() {
        Ok(conn) => query_rows(
            &conn,
            "SELECT name, host, agent_type, pid, metadata, registered_at, last_seen \
             FROM ipc_agents ORDER BY last_seen DESC",
            [],
        )
        .unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    broadcast_brain_event(state, "agent_update", json!({ "agents": agents }));
}

/// Broadcast task_update: fired when a task status changes.
/// Includes the task_id, new status, and owning plan_id for targeted UI updates.
pub fn broadcast_brain_task_update(state: &ServerState, task_id: i64, status: &str) {
    let plan_id = state
        .get_conn()
        .ok()
        .and_then(|conn| {
            conn.query_row(
                "SELECT plan_id FROM tasks WHERE id = ?1",
                rusqlite::params![task_id],
                |r| r.get::<_, i64>(0),
            )
            .ok()
        })
        .unwrap_or(0);
    broadcast_brain_event(
        state,
        "task_update",
        json!({
            "task_id": task_id,
            "status": status,
            "plan_id": plan_id,
        }),
    );
}

/// Broadcast session_update: current running sessions from agent_activity.
/// Called when session changes are detected (register/unregister/heartbeat).
pub fn broadcast_brain_session_update(state: &ServerState) {
    let sessions = match state.get_conn() {
        Ok(conn) => query_rows(
            &conn,
            "SELECT agent_id, agent_type AS type, description, status, metadata, \
             started_at, tokens_total, cost_usd, model \
             FROM agent_activity WHERE agent_id LIKE 'session-%' AND status='running' \
             ORDER BY started_at",
            [],
        )
        .unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    broadcast_brain_event(state, "session_update", json!({ "sessions": sessions }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn broadcast_brain_event_sends_to_channel() {
        let state = ServerState::new(PathBuf::from("/tmp/test-ws-brain.db"), None);
        let mut rx = state.ws_tx.subscribe();
        broadcast_brain_event(&state, "test_event", json!({"key": "value"}));
        let msg = rx.try_recv().expect("should receive brain event");
        assert_eq!(msg["kind"], "brain_event");
        assert_eq!(msg["event_type"], "test_event");
        assert_eq!(msg["payload"]["key"], "value");
    }

    #[test]
    fn broadcast_brain_event_noop_without_subscribers() {
        let state = ServerState::new(PathBuf::from("/tmp/test-ws-brain2.db"), None);
        // No subscribers — should not panic
        broadcast_brain_event(&state, "orphan", json!({}));
    }
}
