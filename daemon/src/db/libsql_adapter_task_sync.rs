use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::db::libsql_adapter::SyncChange;

const AUTHORISED_SYNC_VALIDATORS: &[&str] = &[
    "thor",
    "thor-quality-assurance-guardian",
    "thor-per-wave",
    "forced-admin",
];
const AUTHORISED_EXECUTOR_STATUSES: &[&str] =
    &["idle", "running", "paused", "completed", "failed"];

pub(super) fn normalise_task_sync_change(
    conn: &Connection,
    change: &SyncChange,
    row_exists: bool,
) -> rusqlite::Result<serde_json::Value> {
    let mut data = change.data.clone();
    if change.table_name != "tasks" {
        return Ok(data);
    }

    let executor_status = data.get("executor_status").and_then(Value::as_str);
    if !executor_status
        .is_some_and(|value| AUTHORISED_EXECUTOR_STATUSES.contains(&value))
    {
        data["executor_status"] = serde_json::Value::String("idle".to_string());
    }

    if data.get("status").and_then(Value::as_str) != Some("done") {
        return Ok(data);
    }

    let validated_by = data.get("validated_by").and_then(Value::as_str);
    if !validated_by
        .is_some_and(|value| AUTHORISED_SYNC_VALIDATORS.contains(&value))
    {
        data["validated_by"] = serde_json::Value::String("forced-admin".to_string());
    }

    if row_exists {
        let local_status: Option<String> = conn
            .query_row("SELECT status FROM tasks WHERE id = ?1", params![change.pk], |row| {
                row.get(0)
            })
            .optional()?;
        if let Some(status) = local_status {
            if status != "submitted" && status != "done" {
                conn.execute(
                    "UPDATE tasks SET status = 'submitted' WHERE id = ?1",
                    params![change.pk],
                )?;
            }
        }
    }

    Ok(data)
}
