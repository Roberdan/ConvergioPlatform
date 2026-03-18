use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/plan-db/context/:plan_id", get(handle_get_context))
        .route("/api/plan-db/json/:plan_id", get(handle_get_json))
        .route("/api/plan-db/task/update", post(handle_task_update))
        .route("/api/plan-db/agent/start", post(handle_agent_start))
        .route("/api/plan-db/agent/complete", post(handle_agent_complete))
}

/// GET /api/plan-db/context/:plan_id — full plan+waves+tasks for execution
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
async fn handle_get_json(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let plan = query_one(
        conn,
        "SELECT id, name, status, tasks_total, tasks_done, \
         execution_host, created_at, started_at, completed_at \
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
/// Body: {"task_id": N, "status": "...", "notes": "..."}
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

    let conn = state.get_conn()?;
    let conn = &conn;

    let changed = conn
        .execute(
            "UPDATE tasks SET status = ?1, \
             started_at = CASE WHEN ?1 = 'in_progress' AND started_at IS NULL \
               THEN datetime('now') ELSE started_at END, \
             completed_at = CASE WHEN ?1 IN ('done','submitted') \
               THEN datetime('now') ELSE completed_at END, \
             tokens = tokens + ?2 \
             WHERE id = ?3",
            rusqlite::params![status, tokens, task_id],
        )
        .map_err(|e| ApiError::internal(format!("update failed: {e}")))?;

    if changed == 0 {
        return Err(ApiError::bad_request(format!("task {task_id} not found")));
    }

    // Store notes in description if provided
    if !notes.is_empty() {
        conn.execute(
            "UPDATE tasks SET description = \
             CASE WHEN description IS NULL OR description = '' \
               THEN ?1 ELSE description || char(10) || ?1 END \
             WHERE id = ?2",
            rusqlite::params![notes, task_id],
        )
        .map_err(|e| ApiError::internal(format!("notes update failed: {e}")))?;
    }

    Ok(Json(json!({
        "ok": true,
        "task_id": task_id,
        "status": status,
        "rows_changed": changed,
    })))
}

/// POST /api/plan-db/agent/start — register agent activity
/// Body: {agent_id, agent_type, description?, task_db_id?, plan_id?, model?, host?}
async fn handle_agent_start(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let agent_id = body
        .get("agent_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing agent_id"))?;
    let agent_type = body
        .get("agent_type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let description = body
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let task_db_id = body.get("task_db_id").and_then(Value::as_i64);
    let plan_id = body.get("plan_id").and_then(Value::as_i64);
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let host = body.get("host").and_then(Value::as_str).unwrap_or("local");

    let conn = state.get_conn()?;
    conn.execute(
        "INSERT OR REPLACE INTO agent_activity \
         (agent_id, agent_type, description, task_db_id, plan_id, model, host, \
          status, started_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'running', datetime('now'))",
        rusqlite::params![
            agent_id,
            agent_type,
            description,
            task_db_id,
            plan_id,
            model,
            host
        ],
    )
    .map_err(|e| ApiError::internal(format!("agent start failed: {e}")))?;
    Ok(Json(json!({"ok": true, "agent_id": agent_id})))
}

/// POST /api/plan-db/agent/complete — mark agent done
/// Body: {agent_id, tokens_in?, tokens_out?, cost_usd?, status?}
async fn handle_agent_complete(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let agent_id = body
        .get("agent_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing agent_id"))?;
    let status = body
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("completed");
    let tokens_in = body.get("tokens_in").and_then(Value::as_i64).unwrap_or(0);
    let tokens_out = body.get("tokens_out").and_then(Value::as_i64).unwrap_or(0);
    let cost = body.get("cost_usd").and_then(Value::as_f64).unwrap_or(0.0);

    let conn = state.get_conn()?;
    let changed = conn
        .execute(
            "UPDATE agent_activity SET status = ?1, tokens_in = ?2, tokens_out = ?3, \
         tokens_total = ?2 + ?3, cost_usd = ?4, completed_at = datetime('now'), \
         duration_s = ROUND((julianday('now') - julianday(started_at)) * 86400, 1) \
         WHERE agent_id = ?5",
            rusqlite::params![status, tokens_in, tokens_out, cost, agent_id],
        )
        .map_err(|e| ApiError::internal(format!("agent complete failed: {e}")))?;
    Ok(Json(
        json!({"ok": true, "agent_id": agent_id, "rows_changed": changed}),
    ))
}
