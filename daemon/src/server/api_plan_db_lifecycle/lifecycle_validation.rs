// Validation checks for plan lifecycle state transitions.
// Extracted from handlers.rs — pure query + guard logic.

use crate::server::state::{query_one, ApiError};
use rusqlite::Connection;
use serde_json::Value;

/// Verify all tasks in the plan are in a terminal state before completion.
/// Returns `Ok(())` or `Err` with a descriptive message.
pub(super) fn check_all_tasks_done(conn: &Connection, plan_id: i64) -> Result<(), ApiError> {
    let pending = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM tasks \
         WHERE plan_id = ?1 AND status NOT IN ('done', 'cancelled', 'skipped')",
        rusqlite::params![plan_id],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    if pending > 0 {
        return Err(ApiError::bad_request(format!(
            "plan {plan_id} has {pending} incomplete tasks"
        )));
    }
    Ok(())
}

/// Verify all non-code deliverables linked to done tasks are approved.
/// Returns `Ok(())` or `Err` with a descriptive message.
pub(super) fn check_deliverables_approved(conn: &Connection, plan_id: i64) -> Result<(), ApiError> {
    let unapproved = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM deliverables d \
         JOIN tasks t ON d.task_id = t.id \
         WHERE t.plan_id = ?1 AND t.status = 'done' \
         AND COALESCE(d.output_type, '') != 'pr' \
         AND d.status != 'approved'",
        rusqlite::params![plan_id],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    if unapproved > 0 {
        return Err(ApiError::bad_request(format!(
            "plan {plan_id} has {unapproved} unapproved non-code deliverables"
        )));
    }
    Ok(())
}
