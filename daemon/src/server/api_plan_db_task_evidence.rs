// Task evidence recording — TestGate + ValidatorGate support.
// WHY: Constitution Article VI — terminal status transitions require evidence.
//
// POST /api/plan-db/task/evidence  — record a test/build/lint pass
// GET  /api/plan-db/task/evidence/:task_id — list evidence for a task

use super::state::{query_rows, ApiError, ServerState};
use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};

/// POST /api/plan-db/task/evidence
/// Body: { task_id, evidence_type, command?, output_summary?, exit_code? }
///
/// evidence_type: 'test_pass' | 'build_pass' | 'lint_pass' | 'curl_output'
#[tracing::instrument(skip_all)]
pub async fn handle_record_evidence(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let task_id = body
        .get("task_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing task_id"))?;
    let evidence_type = body
        .get("evidence_type")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing evidence_type"))?;

    // Allowlist — prevents garbage evidence types slipping through.
    match evidence_type {
        "test_pass" | "build_pass" | "lint_pass" | "curl_output" => {}
        other => {
            return Err(ApiError::bad_request(format!(
                "unknown evidence_type '{other}'. \
                 Valid: test_pass|build_pass|lint_pass|curl_output"
            )));
        }
    }

    let command = body.get("command").and_then(Value::as_str).unwrap_or("");
    let output_summary = body
        .get("output_summary")
        .and_then(Value::as_str)
        .unwrap_or("");
    let exit_code = body.get("exit_code").and_then(Value::as_i64).unwrap_or(0);

    let conn = state.get_conn()?;
    let conn = &conn;

    // Verify the task actually exists before inserting evidence.
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE id = ?1",
            rusqlite::params![task_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if exists == 0 {
        return Err(ApiError::bad_request(format!("task {task_id} not found")));
    }

    conn.execute(
        "INSERT INTO task_evidence \
         (task_db_id, evidence_type, command, output_summary, exit_code) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![task_id, evidence_type, command, output_summary, exit_code],
    )
    .map_err(|e| ApiError::internal(format!("insert evidence failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "task_id": task_id,
        "evidence_type": evidence_type,
        "recorded": true,
    })))
}

/// GET /api/plan-db/task/evidence/:task_id — list all evidence rows for a task.
#[tracing::instrument(skip_all)]
pub async fn handle_list_evidence(
    State(state): State<ServerState>,
    Path(task_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let rows = query_rows(
        conn,
        "SELECT id, task_db_id, evidence_type, command, \
         output_summary, exit_code, created_at \
         FROM task_evidence WHERE task_db_id = ?1 ORDER BY id",
        rusqlite::params![task_id],
    )?;

    Ok(Json(json!({
        "ok": true,
        "task_id": task_id,
        "evidence": rows,
    })))
}

/// Query: does task_id have at least one evidence row of the given type?
/// Called inline from the gate checks in api_plan_db_gates.rs.
pub fn has_evidence(
    conn: &rusqlite::Connection,
    task_id: i64,
    evidence_type: &str,
) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM task_evidence \
         WHERE task_db_id = ?1 AND evidence_type = ?2",
        rusqlite::params![task_id, evidence_type],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}
