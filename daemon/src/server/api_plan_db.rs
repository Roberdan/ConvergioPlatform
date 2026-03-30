use super::api_plan_db_evidence;
use super::api_plan_db_gates;
use super::api_plan_db_task_evidence;
use super::state::{query_one, query_rows, ApiError, ServerState};
use super::ws_brain::broadcast_brain_task_update;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route(
            "/api/evidence/clear",
            get(api_plan_db_evidence::handle_clear_evidence_cache),
        )
        .route("/api/plan-db/context/:plan_id", get(handle_get_context))
        .route("/api/plan-db/json/:plan_id", get(handle_get_json))
        .route("/api/plan-db/task/update", post(handle_task_update))
        .route(
            "/api/plan-db/task/evidence",
            post(api_plan_db_task_evidence::handle_record_evidence),
        )
        .route(
            "/api/plan-db/task/evidence/:task_id",
            get(api_plan_db_task_evidence::handle_list_evidence),
        )
        .route(
            "/api/plan-db/agent/start",
            post(super::api_plan_db_agents::handle_agent_start),
        )
        .route(
            "/api/plan-db/agent/complete",
            post(super::api_plan_db_agents::handle_agent_complete),
        )
}

/// GET /api/plan-db/context/:plan_id — full plan+waves+tasks for execution
    #[tracing::instrument(skip_all)]
async fn handle_get_context(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let plan = query_one(
        conn,
        "SELECT id, name, status, project_id, execution_host, worktree_path, \
         description, human_summary, parallel_mode \
         FROM plans WHERE id = ?1",
        rusqlite::params![plan_id],
    )?
    .ok_or_else(|| ApiError::bad_request(format!("plan {plan_id} not found")))?;

    let waves = query_rows(
        conn,
        "SELECT id, wave_id, name, status, worktree_path \
         FROM waves WHERE plan_id = ?1 ORDER BY id",
        rusqlite::params![plan_id],
    )?;

    let tasks = query_rows(
        conn,
        "SELECT id, task_id, title, description, status, type, priority, \
         wave_id_fk, assignee, test_criteria, started_at, completed_at \
         FROM tasks WHERE plan_id = ?1 ORDER BY wave_id_fk, id",
        rusqlite::params![plan_id],
    )?;

    Ok(Json(json!({
        "ok": true,
        "plan": plan,
        "waves": waves,
        "tasks": tasks,
    })))
}

/// GET /api/plan-db/json/:plan_id — compact plan JSON (same as plan-db.sh json)
    #[tracing::instrument(skip_all)]
async fn handle_get_json(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let plan = query_one(
        conn,
        "SELECT id, name, status, tasks_total, tasks_done, \
         execution_host, created_at, started_at, completed_at, \
         COALESCE(waves_total, 0) AS waves_total, \
         COALESCE(waves_merged, 0) AS waves_merged, \
         CASE WHEN COALESCE(waves_total, 0) > 0 \
           THEN COALESCE(waves_merged, 0) * 100 / waves_total \
           ELSE 0 END AS merge_pct, \
         (SELECT COUNT(*) FROM deliverables d JOIN tasks t ON d.task_id = t.id \
           WHERE t.plan_id = plans.id AND d.status = 'approved') AS deliverables_approved, \
         (SELECT COUNT(*) FROM deliverables d JOIN tasks t ON d.task_id = t.id \
           WHERE t.plan_id = plans.id AND COALESCE(d.output_type, '') != 'pr') AS deliverables_total \
         FROM plans WHERE id = ?1",
        rusqlite::params![plan_id],
    )?
    .ok_or_else(|| ApiError::bad_request(format!("plan {plan_id} not found")))?;

    let waves = query_rows(
        conn,
        "SELECT id, wave_id, name, status FROM waves \
         WHERE plan_id = ?1 ORDER BY id",
        rusqlite::params![plan_id],
    )?;

    let tasks = query_rows(
        conn,
        "SELECT id, task_id, title, status, type, priority, wave_id_fk \
         FROM tasks WHERE plan_id = ?1 ORDER BY wave_id_fk, id",
        rusqlite::params![plan_id],
    )?;

    Ok(Json(json!({
        "ok": true,
        "plan": plan,
        "waves": waves,
        "tasks": tasks,
    })))
}

/// POST /api/plan-db/task/update — update task status
/// Body: {"task_id": N, "status": "...", "notes": "...", "test_criteria": "..."}
    #[tracing::instrument(skip_all)]
async fn handle_task_update(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let task_id = body
        .get("task_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing task_id"))?;
    let status = body
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing status"))?;
    let notes = body.get("notes").and_then(Value::as_str).unwrap_or("");
    let tokens = body.get("tokens").and_then(Value::as_i64).unwrap_or(0);
    let validated_by = body.get("validated_by").and_then(Value::as_str);
    let test_criteria = body.get("test_criteria").and_then(Value::as_str);
    let agent = body.get("agent").and_then(Value::as_str);

    let conn = state.get_conn()?;
    let conn = &conn;

    // ── Gate 1: TestGate — BLOCK submitted without test evidence ────────────
    // WHY: Constitution Article VI; plan v20 found 8/17 features fake.
    // Skipped in tests to allow unit test isolation.
    #[cfg(not(test))]
    if status == "submitted" {
        api_plan_db_gates::run_test_gate(conn, task_id)?;
    }

    // ── Gate 2: ValidatorGate — BLOCK done without Thor verdict ─────────────
    // WHY: Constitution Article VI status flow: submitted → validated → done.
    #[cfg(not(test))]
    if status == "done" {
        api_plan_db_gates::run_validator_gate(conn, task_id)?;
    }

    // Kernel evidence gate (deep artifact + worktree checks, kernel feature only).
    // WHY: Article VI of the Constitution — "done" must be backed by evidence.
    #[cfg(all(feature = "kernel", not(test)))]
    if matches!(status, "done" | "submitted") {
        api_plan_db_evidence::run_evidence_gate(conn, task_id, status)?;
    }

    let changed = conn
        .execute(
            "UPDATE tasks SET status = ?1, \
             validated_by = COALESCE(?4, validated_by), \
             started_at = CASE WHEN ?1 = 'in_progress' AND started_at IS NULL \
               THEN datetime('now') ELSE started_at END, \
             completed_at = CASE WHEN ?1 IN ('done','submitted') \
               THEN datetime('now') ELSE completed_at END, \
             tokens = tokens + ?2 \
             WHERE id = ?3",
            rusqlite::params![status, tokens, task_id, validated_by],
        )
        .map_err(|e| ApiError::internal(format!("update failed: {e}")))?;

    if changed == 0 {
        return Err(ApiError::bad_request(format!("task {task_id} not found")));
    }

    // Audit: record every state-changing task-update API call
    super::api_audit::log_audit(
        conn,
        agent,
        "task_update",
        Some(&task_id.to_string()),
        Some(status),
    );

    // Update notes field (verify command source for mechanical gates validator)
    if !notes.is_empty() {
        conn.execute(
            "UPDATE tasks SET notes = ?1 WHERE id = ?2",
            rusqlite::params![notes, task_id],
        )
        .map_err(|e| ApiError::internal(format!("notes update failed: {e}")))?;
    }

    // Update test_criteria if provided (mechanical gate: must be non-empty)
    if let Some(tc) = test_criteria {
        conn.execute(
            "UPDATE tasks SET test_criteria = ?1 WHERE id = ?2",
            rusqlite::params![tc, task_id],
        )
        .map_err(|e| ApiError::internal(format!("test_criteria update failed: {e}")))?;
    }

    // Push task status change to brain viz via WS
    broadcast_brain_task_update(&state, task_id, status);

    // Counter updates + IPC broadcast → api_plan_db_counters.rs
    super::api_plan_db_counters::update_counters_and_broadcast(conn, &state, task_id, status)?;

    Ok(Json(json!({
        "ok": true,
        "task_id": task_id,
        "status": status,
        "rows_changed": changed,
    })))
}

// Agent start/complete handlers → api_plan_db_agents.rs
