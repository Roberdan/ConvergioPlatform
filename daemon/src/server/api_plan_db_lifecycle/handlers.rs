use super::super::plan_lifecycle_guards;
use super::super::state::{ApiError, ServerState};
use super::lifecycle_validation::{
    check_all_tasks_done, check_deliverables_approved, run_post_complete_cleanup,
    worktree_cleanup_paths,
};
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/plan-db/create", post(handle_create))
        .route("/api/plan-db/start/:plan_id", post(handle_start))
        .route("/api/plan-db/complete/:plan_id", post(handle_complete))
        .route("/api/plan-db/cancel/:plan_id", post(handle_cancel))
        .route("/api/plan-db/approve/:plan_id", post(handle_approve))
}

/// POST /api/plan-db/create — create a new plan
/// Body: {project_id, name, source_file?, description?, parent_plan_id?}
pub(super) async fn handle_create(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let project_id = body
        .get("project_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing project_id"))?;
    let name = body
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing name"))?;
    let source_file = body.get("source_file").and_then(Value::as_str);
    let description = body.get("description").and_then(Value::as_str);
    let parent_plan_id = body.get("parent_plan_id").and_then(Value::as_i64);

    let conn = state.get_conn()?;
    let conn = &conn;

    // If parent specified, promote it to master
    if let Some(pid) = parent_plan_id {
        conn.execute(
            "UPDATE plans SET is_master = 1 WHERE id = ?1 AND is_master = 0",
            rusqlite::params![pid],
        )
        .map_err(|e| ApiError::internal(format!("promote parent failed: {e}")))?;
    }

    conn.execute(
        "INSERT INTO plans (project_id, name, status, source_file, description, \
         parent_plan_id, created_at, updated_at) \
         VALUES (?1, ?2, 'draft', ?3, ?4, ?5, datetime('now'), datetime('now'))",
        rusqlite::params![project_id, name, source_file, description, parent_plan_id],
    )
    .map_err(|e| ApiError::internal(format!("create failed: {e}")))?;

    let plan_id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(|e| ApiError::internal(format!("rowid query failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "status": "draft",
        "parent_plan_id": parent_plan_id,
    })))
}

/// POST /api/plan-db/start/:plan_id — set status=doing, started_at=now
pub(super) async fn handle_start(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    // Guard: plan must have imported tasks and an approved review
    plan_lifecycle_guards::require_plan_startable(plan_id, conn)?;

    let changed = conn
        .execute(
            "UPDATE plans SET status = 'doing', \
             started_at = COALESCE(started_at, datetime('now')), \
             updated_at = datetime('now') \
             WHERE id = ?1 AND status IN ('draft', 'approved', 'todo')",
            rusqlite::params![plan_id],
        )
        .map_err(|e| ApiError::internal(format!("start failed: {e}")))?;

    if changed == 0 {
        return Err(ApiError::bad_request(format!(
            "plan {plan_id} not found or not in startable state"
        )));
    }

    // Broadcast plan_started to Ali orchestrator
    if let Some(ref ipc) = state.ipc_engine {
        let content = serde_json::json!({"type": "plan_started", "plan_id": plan_id}).to_string();
        let _ = ipc.broadcast("api", &content, "event", Some("#orchestration"));
    }

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "status": "doing",
    })))
}

/// POST /api/plan-db/complete/:plan_id — set status=completed
/// Guard: all tasks must be done/cancelled/skipped.
/// For non-code plans (all tasks output_type != 'pr'), completion requires
/// all deliverables approved. Mixed plans require both PRs merged AND
/// non-code deliverables approved.
pub(super) async fn handle_complete(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    check_all_tasks_done(conn, plan_id)?;
    check_deliverables_approved(conn, plan_id)?;

    let changed = conn
        .execute(
            "UPDATE plans SET status = 'completed', \
             completed_at = datetime('now'), updated_at = datetime('now') \
             WHERE id = ?1 AND status IN ('doing', 'approved')",
            rusqlite::params![plan_id],
        )
        .map_err(|e| ApiError::internal(format!("complete failed: {e}")))?;

    if changed == 0 {
        return Err(ApiError::bad_request(format!(
            "plan {plan_id} not found or not in completable state"
        )));
    }

    // Auto-cleanup worktrees and temp files after plan completion
    let wt_paths = worktree_cleanup_paths(conn, plan_id);
    tokio::spawn(async move {
        run_post_complete_cleanup(plan_id, &wt_paths);
    });

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "status": "completed",
    })))
}

/// POST /api/plan-db/cancel/:plan_id — cancel plan + all pending tasks
/// Body: {reason?}
pub(super) async fn handle_cancel(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let reason = body
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("cancelled");

    let conn = state.get_conn()?;
    let conn = &conn;

    // Cancel all pending/in_progress tasks
    let tasks_cancelled = conn
        .execute(
            "UPDATE tasks SET status = 'cancelled', \
             completed_at = datetime('now') \
             WHERE plan_id = ?1 AND status IN ('pending', 'in_progress')",
            rusqlite::params![plan_id],
        )
        .map_err(|e| ApiError::internal(format!("cancel tasks failed: {e}")))?;

    // Cancel all pending waves
    conn.execute(
        "UPDATE waves SET status = 'cancelled', \
         cancelled_at = datetime('now'), cancelled_reason = ?2 \
         WHERE plan_id = ?1 AND status IN ('pending', 'in_progress')",
        rusqlite::params![plan_id, reason],
    )
    .map_err(|e| ApiError::internal(format!("cancel waves failed: {e}")))?;

    let changed = conn
        .execute(
            "UPDATE plans SET status = 'cancelled', \
             cancelled_at = datetime('now'), cancelled_reason = ?2, \
             updated_at = datetime('now') \
             WHERE id = ?1 AND status NOT IN ('completed', 'cancelled')",
            rusqlite::params![plan_id, reason],
        )
        .map_err(|e| ApiError::internal(format!("cancel plan failed: {e}")))?;

    if changed == 0 {
        return Err(ApiError::bad_request(format!(
            "plan {plan_id} not found or already completed/cancelled"
        )));
    }

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "status": "cancelled",
        "tasks_cancelled": tasks_cancelled,
    })))
}

/// POST /api/plan-db/approve/:plan_id — set status=approved (from draft)
pub(super) async fn handle_approve(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let changed = conn
        .execute(
            "UPDATE plans SET status = 'approved', updated_at = datetime('now') \
             WHERE id = ?1 AND status IN ('draft', 'todo')",
            rusqlite::params![plan_id],
        )
        .map_err(|e| ApiError::internal(format!("approve failed: {e}")))?;
    if changed == 0 {
        return Err(ApiError::bad_request(format!(
            "plan {plan_id} not found or not in approvable state"
        )));
    }
    Ok(Json(json!({ "ok": true, "plan_id": plan_id, "status": "approved" })))
}
