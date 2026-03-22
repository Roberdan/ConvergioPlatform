//! Plan lifecycle guard functions.
//!
//! Each guard checks a precondition and returns `Err(String)` on violation.
//! Handlers convert the error string into a 409 Conflict response.

use rusqlite::Connection;

/// Verify that an approved review exists for the given plan.
///
/// Checks `plan_reviews` for `verdict IN ('approved', 'proceed', 'APPROVED')`.
pub fn require_review(plan_id: i64, conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM plan_reviews \
             WHERE plan_id = ?1 AND verdict IN ('approved', 'proceed', 'APPROVED')",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("REVIEW_CHECK_FAILED: query error: {e}"))?;

    if count == 0 {
        return Err(format!(
            "REVIEW_REQUIRED: plan {plan_id} has no approved review"
        ));
    }
    Ok(())
}

/// Verify that the plan exists and return its current status.
pub fn require_plan_exists(plan_id: i64, conn: &Connection) -> Result<String, String> {
    conn.query_row(
        "SELECT status FROM plans WHERE id = ?1",
        rusqlite::params![plan_id],
        |r| r.get::<_, String>(0),
    )
    .map_err(|_| format!("PLAN_NOT_FOUND: plan {plan_id} does not exist"))
}

/// Verify that the plan exists and is in an importable state (draft/todo/approved).
pub fn require_plan_importable(plan_id: i64, conn: &Connection) -> Result<(), String> {
    let status = require_plan_exists(plan_id, conn)?;
    match status.as_str() {
        "draft" | "todo" | "approved" => Ok(()),
        other => Err(format!(
            "PLAN_NOT_IMPORTABLE: plan {plan_id} status is '{other}', \
             expected draft/todo/approved"
        )),
    }
}

/// Verify that the plan is ready to start:
/// - Has imported tasks (tasks_total > 0)
/// - Has an approved review
pub fn require_plan_startable(plan_id: i64, conn: &Connection) -> Result<(), String> {
    let tasks_total: i64 = conn
        .query_row(
            "SELECT COALESCE(tasks_total, 0) FROM plans WHERE id = ?1",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .map_err(|_| format!("PLAN_NOT_FOUND: plan {plan_id} does not exist"))?;

    if tasks_total == 0 {
        return Err(format!(
            "NO_SPEC_IMPORTED: plan {plan_id} has no tasks (tasks_total=0)"
        ));
    }

    require_review(plan_id, conn)?;
    Ok(())
}

#[cfg(test)]
#[path = "plan_lifecycle_guards_tests.rs"]
mod tests;
