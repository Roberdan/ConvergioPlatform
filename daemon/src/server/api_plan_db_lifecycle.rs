use super::state::{query_one, ApiError, ServerState};
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
/// Body: {project_id, name, source_file?, description?}
async fn handle_create(
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

    let conn = state.get_conn()?;
    let conn = &conn;

    conn.execute(
        "INSERT INTO plans (project_id, name, status, source_file, description, \
         created_at, updated_at) \
         VALUES (?1, ?2, 'draft', ?3, ?4, datetime('now'), datetime('now'))",
        rusqlite::params![project_id, name, source_file, description],
    )
    .map_err(|e| ApiError::internal(format!("create failed: {e}")))?;

    let plan_id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(|e| ApiError::internal(format!("rowid query failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "status": "draft",
    })))
}

/// POST /api/plan-db/start/:plan_id — set status=doing, started_at=now
async fn handle_start(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

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

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "status": "doing",
    })))
}

/// POST /api/plan-db/complete/:plan_id — set status=completed
/// Guard: all tasks must be done/cancelled/skipped
async fn handle_complete(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    // Check for incomplete tasks
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

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "status": "completed",
    })))
}

/// POST /api/plan-db/cancel/:plan_id — cancel plan + all pending tasks
/// Body: {reason?}
async fn handle_cancel(
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
async fn handle_approve(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let changed = conn
        .execute(
            "UPDATE plans SET status = 'approved', \
             updated_at = datetime('now') \
             WHERE id = ?1 AND status IN ('draft', 'todo')",
            rusqlite::params![plan_id],
        )
        .map_err(|e| ApiError::internal(format!("approve failed: {e}")))?;

    if changed == 0 {
        return Err(ApiError::bad_request(format!(
            "plan {plan_id} not found or not in approvable state"
        )));
    }

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "status": "approved",
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::PlanDb;

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
                 CREATE TABLE plans (
                     id INTEGER PRIMARY KEY, project_id TEXT NOT NULL,
                     name TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'draft',
                     source_file TEXT, description TEXT, human_summary TEXT,
                     tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0,
                     execution_host TEXT, worktree_path TEXT, parallel_mode TEXT,
                     created_at TEXT, started_at TEXT, completed_at TEXT,
                     updated_at TEXT, cancelled_at TEXT, cancelled_reason TEXT,
                     constraints_json TEXT
                 );
                 CREATE TABLE waves (
                     id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id TEXT,
                     name TEXT, status TEXT DEFAULT 'pending',
                     tasks_done INTEGER DEFAULT 0, tasks_total INTEGER DEFAULT 0,
                     position INTEGER DEFAULT 0, worktree_path TEXT,
                     cancelled_at TEXT, cancelled_reason TEXT, project_id TEXT
                 );
                 CREATE TABLE tasks (
                     id INTEGER PRIMARY KEY, project_id TEXT, plan_id INTEGER,
                     wave_id_fk INTEGER, wave_id TEXT, task_id TEXT,
                     title TEXT, status TEXT DEFAULT 'pending',
                     started_at TEXT, completed_at TEXT, notes TEXT,
                     tokens INTEGER DEFAULT 0, description TEXT,
                     type TEXT, priority TEXT, assignee TEXT,
                     test_criteria TEXT, output_data TEXT, executor_host TEXT,
                     validated_at TEXT, validated_by TEXT, validation_report TEXT
                 );
                 INSERT INTO projects (id, name) VALUES ('test', 'Test');",
            )
            .expect("schema");
        db
    }

    #[test]
    fn plan_db_lifecycle_create_start_complete() {
        let db = setup_db();
        let conn = db.connection();

        // Create plan
        conn.execute(
            "INSERT INTO plans (project_id, name, status) VALUES ('test', 'Plan A', 'draft')",
            [],
        )
        .expect("create");
        let plan_id: i64 = conn
            .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
            .unwrap();

        // Start
        let changed = conn
            .execute(
                "UPDATE plans SET status = 'doing', \
                 started_at = datetime('now') \
                 WHERE id = ?1 AND status IN ('draft', 'approved', 'todo')",
                rusqlite::params![plan_id],
            )
            .unwrap();
        assert_eq!(changed, 1);

        // Complete (no tasks, should succeed)
        let pending: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE plan_id = ?1 \
                 AND status NOT IN ('done', 'cancelled', 'skipped')",
                rusqlite::params![plan_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending, 0);

        let changed = conn
            .execute(
                "UPDATE plans SET status = 'completed', completed_at = datetime('now') \
                 WHERE id = ?1 AND status IN ('doing', 'approved')",
                rusqlite::params![plan_id],
            )
            .unwrap();
        assert_eq!(changed, 1);
    }

    #[test]
    fn plan_db_lifecycle_cancel_cascades() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO plans (project_id, name, status) VALUES ('test', 'Plan B', 'doing')",
            [],
        )
        .unwrap();
        let plan_id: i64 = conn
            .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
            .unwrap();

        conn.execute(
            "INSERT INTO waves (plan_id, wave_id, name, status, project_id) \
             VALUES (?1, 'W1', 'Wave 1', 'pending', 'test')",
            rusqlite::params![plan_id],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO tasks (plan_id, wave_id, task_id, title, status, project_id, wave_id_fk) \
             VALUES (?1, 'W1', 'T1', 'Task 1', 'pending', 'test', 1), \
                    (?1, 'W1', 'T2', 'Task 2', 'in_progress', 'test', 1)",
            rusqlite::params![plan_id],
        )
        .unwrap();

        // Cancel cascades to tasks
        let tasks_cancelled = conn
            .execute(
                "UPDATE tasks SET status = 'cancelled' \
                 WHERE plan_id = ?1 AND status IN ('pending', 'in_progress')",
                rusqlite::params![plan_id],
            )
            .unwrap();
        assert_eq!(tasks_cancelled, 2);

        conn.execute(
            "UPDATE plans SET status = 'cancelled' WHERE id = ?1",
            rusqlite::params![plan_id],
        )
        .unwrap();

        let status: String = conn
            .query_row(
                "SELECT status FROM plans WHERE id = ?1",
                rusqlite::params![plan_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "cancelled");
    }
}
