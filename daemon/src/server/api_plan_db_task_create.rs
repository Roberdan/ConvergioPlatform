// POST /api/plan-db/task/create — insert a new task into an existing plan/wave.
// WHY: The import API only works on draft/todo plans; this allows task creation
// on plans in any status (e.g. adding scope mid-execution).

use super::state::{ApiError, ServerState};
use axum::{extract::State, Json};
use serde_json::{json, Value};

/// POST /api/plan-db/task/create
/// Body: {plan_id, wave_id_fk, task_id, title, priority?, type?, model?, description?}
///
/// Inserts the task and increments tasks_total on both the wave and plan.
/// No lifecycle guard — callers may add tasks to plans in any status.
#[tracing::instrument(skip_all)]
pub async fn handle_create_task(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let plan_id = body
        .get("plan_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing plan_id"))?;
    let wave_id_fk = body
        .get("wave_id_fk")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing wave_id_fk"))?;
    let task_id = body
        .get("task_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing task_id"))?;
    let title = body
        .get("title")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing title"))?;

    let priority = body.get("priority").and_then(Value::as_str).unwrap_or("P2");
    let task_type = body.get("type").and_then(Value::as_str).unwrap_or("feature");
    let model = body.get("model").and_then(Value::as_str).unwrap_or("");
    let description = body.get("description").and_then(Value::as_str).unwrap_or("");

    let conn = state.get_conn()?;
    let conn = &conn;

    // Verify the plan exists.
    let plan_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM plans WHERE id = ?1",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if plan_exists == 0 {
        return Err(ApiError::bad_request(format!("plan {plan_id} not found")));
    }

    // Verify the wave exists and belongs to the plan.
    let wave_ok: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM waves WHERE id = ?1 AND plan_id = ?2",
            rusqlite::params![wave_id_fk, plan_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if wave_ok == 0 {
        return Err(ApiError::bad_request(format!(
            "wave {wave_id_fk} not found in plan {plan_id}"
        )));
    }

    // Derive project_id from plan.
    let project_id: Option<String> = conn
        .query_row(
            "SELECT project_id FROM plans WHERE id = ?1",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .ok();

    // Insert the task.
    conn.execute(
        "INSERT INTO tasks \
         (plan_id, wave_id_fk, task_id, title, priority, type, model, description, \
          project_id, status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending')",
        rusqlite::params![
            plan_id,
            wave_id_fk,
            task_id,
            title,
            priority,
            task_type,
            model,
            description,
            project_id
        ],
    )
    .map_err(|e| ApiError::internal(format!("insert task failed: {e}")))?;

    let db_id = conn.last_insert_rowid();

    // Increment tasks_total on wave and plan.
    conn.execute(
        "UPDATE waves SET tasks_total = COALESCE(tasks_total, 0) + 1 WHERE id = ?1",
        rusqlite::params![wave_id_fk],
    )
    .map_err(|e| ApiError::internal(format!("wave counter update failed: {e}")))?;

    conn.execute(
        "UPDATE plans SET tasks_total = COALESCE(tasks_total, 0) + 1 WHERE id = ?1",
        rusqlite::params![plan_id],
    )
    .map_err(|e| ApiError::internal(format!("plan counter update failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "db_id": db_id,
        "task_id": task_id,
        "plan_id": plan_id,
        "wave_id_fk": wave_id_fk,
    })))
}

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
                 CREATE TABLE plans (
                     id INTEGER PRIMARY KEY, project_id TEXT NOT NULL,
                     name TEXT NOT NULL, status TEXT DEFAULT 'draft',
                     tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0
                 );
                 CREATE TABLE waves (
                     id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id TEXT,
                     name TEXT, status TEXT DEFAULT 'pending',
                     tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0
                 );
                 CREATE TABLE tasks (
                     id INTEGER PRIMARY KEY, project_id TEXT, plan_id INTEGER,
                     wave_id_fk INTEGER, wave_id TEXT, task_id TEXT,
                     title TEXT, status TEXT DEFAULT 'pending',
                     priority TEXT, type TEXT, assignee TEXT,
                     test_criteria TEXT, description TEXT, model TEXT,
                     started_at TEXT, completed_at TEXT,
                     validated_at TEXT, validated_by TEXT,
                     validation_report TEXT, executor_host TEXT,
                     notes TEXT, tokens INTEGER DEFAULT 0
                 );
                 INSERT INTO projects (id, name) VALUES ('proj', 'TestProject');
                 INSERT INTO plans (id, project_id, name, status, tasks_total)
                     VALUES (1, 'proj', 'Plan A', 'doing', 0);
                 INSERT INTO waves (id, plan_id, wave_id, name, status, tasks_total)
                     VALUES (10, 1, 'W1', 'Wave 1', 'in_progress', 0);",
            )
            .expect("schema");
        db
    }

    #[test]
    fn create_task_returns_db_id() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO tasks \
             (plan_id, wave_id_fk, task_id, title, priority, type, model, description, \
              project_id, status) \
             VALUES (1, 10, 'T1-01', 'Add feature', 'P1', 'feature', 'sonnet', 'Desc', 'proj', 'pending')",
            [],
        )
        .expect("insert");

        let db_id = conn.last_insert_rowid();
        assert!(db_id > 0, "last_insert_rowid must return a positive id");

        let title: String = conn
            .query_row(
                "SELECT title FROM tasks WHERE id = ?1",
                rusqlite::params![db_id],
                |r| r.get(0),
            )
            .expect("query");
        assert_eq!(title, "Add feature");
    }

    #[test]
    fn create_task_increments_plan_and_wave_counters() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO tasks \
             (plan_id, wave_id_fk, task_id, title, priority, type, model, description, \
              project_id, status) \
             VALUES (1, 10, 'T1-02', 'Second task', 'P2', 'chore', '', '', 'proj', 'pending')",
            [],
        )
        .expect("insert");

        conn.execute(
            "UPDATE waves SET tasks_total = COALESCE(tasks_total, 0) + 1 WHERE id = 10",
            [],
        )
        .expect("wave counter");
        conn.execute(
            "UPDATE plans SET tasks_total = COALESCE(tasks_total, 0) + 1 WHERE id = 1",
            [],
        )
        .expect("plan counter");

        let wave_total: i64 = conn
            .query_row(
                "SELECT tasks_total FROM waves WHERE id = 10",
                [],
                |r| r.get(0),
            )
            .expect("wave query");
        let plan_total: i64 = conn
            .query_row(
                "SELECT tasks_total FROM plans WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .expect("plan query");

        assert_eq!(wave_total, 1, "wave tasks_total must be incremented");
        assert_eq!(plan_total, 1, "plan tasks_total must be incremented");
    }
}
