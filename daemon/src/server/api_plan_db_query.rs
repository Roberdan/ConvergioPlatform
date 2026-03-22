use super::api_plan_db_query_fmt::handle_execution_tree;
use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/plan-db/list", get(handle_list))
        .route(
            "/api/plan-db/execution-tree/:plan_id",
            get(handle_execution_tree),
        )
        .route("/api/plan-db/drift-check/:plan_id", get(handle_drift_check))
        .route(
            "/api/plan-db/validate-task/:task_id/:plan_id",
            get(handle_validate_task),
        )
}

/// GET /api/plan-db/list — active plans with task counts
async fn handle_list(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let plans = query_rows(
        &conn,
        "SELECT p.id, p.name, p.status, p.project_id, p.execution_host, \
         p.worktree_path, p.description, p.human_summary, p.parallel_mode, \
         p.tasks_total, p.tasks_done, p.created_at, p.started_at, \
         COALESCE(p.waves_total, 0) AS waves_total, \
         COALESCE(p.waves_merged, 0) AS waves_merged, \
         CASE WHEN COALESCE(p.waves_total, 0) > 0 \
           THEN COALESCE(p.waves_merged, 0) * 100 / p.waves_total \
           ELSE 0 END AS merge_pct \
         FROM plans p \
         WHERE p.status NOT IN ('completed', 'cancelled') \
         ORDER BY p.id DESC",
        [],
    )?;

    Ok(Json(json!({ "ok": true, "plans": plans })))
}

/// GET /api/plan-db/drift-check/:plan_id — check plan staleness
async fn handle_drift_check(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let plan = query_one(
        conn,
        "SELECT id, name, status, worktree_path, started_at, updated_at \
         FROM plans WHERE id = ?1",
        rusqlite::params![plan_id],
    )?
    .ok_or_else(|| ApiError::bad_request(format!("plan {plan_id} not found")))?;

    // Check tasks with stale status
    let stale_tasks = query_rows(
        conn,
        "SELECT id, task_id, title, status, started_at \
         FROM tasks WHERE plan_id = ?1 AND status = 'in_progress' \
         AND started_at < datetime('now', '-24 hours') \
         ORDER BY started_at",
        rusqlite::params![plan_id],
    )?;

    let in_progress: i64 = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM tasks \
         WHERE plan_id = ?1 AND status = 'in_progress'",
        rusqlite::params![plan_id],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    Ok(Json(json!({
        "ok": true,
        "plan": plan,
        "stale_tasks": stale_tasks,
        "in_progress_count": in_progress,
        "has_drift": !stale_tasks.is_empty(),
    })))
}

/// GET /api/plan-db/validate-task/:task_id/:plan_id — mechanical validation
///
/// Runs deterministic mechanical gates (status, test_criteria, file checks).
/// Thor AI validation runs at wave level, not per-task.
async fn handle_validate_task(
    State(state): State<ServerState>,
    Path((task_id, plan_id)): Path<(i64, i64)>,
) -> Result<Json<Value>, ApiError> {
    use crate::validation::mechanical_gates;

    let conn = state.get_conn()?;
    let conn = &conn;

    let task = query_one(
        conn,
        "SELECT id, task_id, title, status, test_criteria, notes, \
         validated_at, validated_by, validation_report, \
         started_at, completed_at \
         FROM tasks WHERE id = ?1 AND plan_id = ?2",
        rusqlite::params![task_id, plan_id],
    )?
    .ok_or_else(|| ApiError::bad_request(format!("task {task_id} not found in plan {plan_id}")))?;

    let status = task
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let test_criteria = task.get("test_criteria").and_then(Value::as_str);
    let notes = task.get("notes").and_then(Value::as_str);

    // Parse verify commands from notes (newline-separated)
    let verify_cmds: Vec<&str> = notes
        .map(|n| n.lines().filter(|l| !l.trim().is_empty()).collect())
        .unwrap_or_default();

    // Run mechanical gates (no file paths stored in DB — use empty slice)
    let result = mechanical_gates::validate_task(
        status,
        test_criteria,
        &[],
        &verify_cmds,
    );

    let is_validated = task.get("validated_by").is_some()
        && !task
            .get("validated_by")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty();

    Ok(Json(json!({
        "ok": result.all_passed(),
        "task": task,
        "mechanical": result,
        "is_validated": is_validated,
        "can_complete": status == "submitted" || is_validated,
    })))
}
