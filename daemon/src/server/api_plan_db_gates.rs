// Hard mechanical enforcement gates — TestGate + ValidatorGate.
// WHY: Constitution Article VI; plan v20 audit found 8/17 features fake.
// These gates are framework-agnostic: evidence is posted by the executor
// via POST /api/plan-db/task/evidence regardless of lang/framework.

use super::api_plan_db_task_evidence::has_evidence;
use super::state::ApiError;
use rusqlite::Connection;

// ── Gate 1: TestGate ─────────────────────────────────────────────────────────

/// Block status=submitted when no test_pass evidence has been recorded.
///
/// Executors MUST POST /api/plan-db/task/evidence with evidence_type='test_pass'
/// after running their test suite before calling task/update status=submitted.
pub fn run_test_gate(conn: &Connection, task_id: i64) -> Result<(), ApiError> {
    if has_evidence(conn, task_id, "test_pass") {
        return Ok(());
    }
    Err(ApiError::bad_request(format!(
        "TestGate: no test evidence recorded for task {task_id}. \
         Run tests and POST /api/plan-db/task/evidence \
         {{\"task_id\":{task_id},\"evidence_type\":\"test_pass\",...}} first."
    )))
}

// ── Gate 2: ValidatorGate ────────────────────────────────────────────────────

/// Block status=done when no passing Thor verdict exists for this task.
///
/// Flow: executor → submitted → Thor validates → POST verdict → done.
/// If the validation_queue / validation_verdicts tables don't exist yet
/// (e.g. test DB), the gate is a no-op (best-effort; not punitive during bootstrap).
pub fn run_validator_gate(conn: &Connection, task_id: i64) -> Result<(), ApiError> {
    // Check whether the table exists before querying — guard for fresh DBs.
    let table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type='table' AND name='validation_verdicts'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if table_exists == 0 {
        // Tables not yet migrated — allow through with a warning logged.
        tracing::warn!(
            task_id,
            "ValidatorGate: validation_verdicts table absent; gate skipped"
        );
        return Ok(());
    }

    let pass_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM validation_verdicts v \
             JOIN validation_queue q ON v.queue_id = q.id \
             WHERE q.task_id = ?1 AND v.verdict = 'pass'",
            rusqlite::params![task_id],
            |r| r.get(0),
        )
        .unwrap_or(0);

    if pass_count > 0 {
        return Ok(());
    }

    Err(ApiError::bad_request(format!(
        "ValidatorGate: no passing Thor verdict for task {task_id}. \
         Task must reach status=submitted first, then be validated \
         by Thor before transitioning to done."
    )))
}
