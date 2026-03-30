// Counter and IPC broadcast logic for task status updates.

use super::state::{ApiError, ServerState};

/// Update wave/plan counters and broadcast task_done event after a task completes.
pub fn update_counters_and_broadcast(
    conn: &rusqlite::Connection,
    state: &ServerState,
    task_id: i64,
    status: &str,
) -> Result<(), ApiError> {
    if status != "done" {
        return Ok(());
    }

    conn.execute(
        "UPDATE waves SET tasks_done = (
            SELECT COUNT(*) FROM tasks WHERE wave_id_fk = waves.id AND status = 'done'
        ) WHERE id = (SELECT wave_id_fk FROM tasks WHERE id = ?1)",
        rusqlite::params![task_id],
    )
    .map_err(|e| ApiError::internal(format!("wave counter update: {e}")))?;
    conn.execute(
        "UPDATE plans SET tasks_done = (
            SELECT COUNT(*) FROM tasks WHERE plan_id = plans.id AND status = 'done'
        ) WHERE id = (SELECT plan_id FROM tasks WHERE id = ?1)",
        rusqlite::params![task_id],
    )
    .map_err(|e| ApiError::internal(format!("plan counter update: {e}")))?;

    // Auto-complete wave when all tasks are done
    let wave_complete: bool = conn
        .query_row(
            "SELECT tasks_done = tasks_total FROM waves \
             WHERE id = (SELECT wave_id_fk FROM tasks WHERE id = ?1)",
            rusqlite::params![task_id],
            |row| row.get(0),
        )
        .unwrap_or(false);
    if wave_complete {
        conn.execute(
            "UPDATE waves SET status = 'done' \
             WHERE id = (SELECT wave_id_fk FROM tasks WHERE id = ?1) AND status != 'done'",
            rusqlite::params![task_id],
        )
        .ok(); // intentional: best-effort wave auto-complete
    }

    // Broadcast task_done to Ali orchestrator
    if let Some(ref ipc) = state.ipc_engine {
        let plan_id: Option<i64> = match conn.query_row(
            "SELECT plan_id FROM tasks WHERE id = ?1",
            rusqlite::params![task_id],
            |row| row.get(0),
        ) {
            Ok(v) => Some(v),
            Err(e) => { tracing::warn!("plan_id lookup for task {task_id}: {e}"); None }
        };
        if let Some(pid) = plan_id {
            let content = serde_json::json!({
                "type": "task_done",
                "task_id": task_id.to_string(),
                "plan_id": pid,
            })
            .to_string();
            if let Err(e) = ipc.broadcast("api", &content, "event", Some("#orchestration")) {
                tracing::warn!("ipc task_done broadcast failed: {e}");
            }
        }
    }

    Ok(())
}
