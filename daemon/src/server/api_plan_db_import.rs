use super::api_plan_db_import_defaults::apply_defaults;
use super::api_plan_db_import_parsers::parse_waves;
use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/plan-db/import", post(handle_import))
}

/// POST /api/plan-db/import — bulk import waves+tasks from JSON/YAML spec
async fn handle_import(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let plan_id = body
        .get("plan_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing plan_id"))?;

    // Parse waves from spec (support raw YAML string or JSON object)
    let mut waves = parse_waves(&body)?;

    let conn = state.get_conn()?;
    let conn = &conn;

    // Verify plan exists
    let project_id: String = conn
        .query_row(
            "SELECT project_id FROM plans WHERE id = ?1",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .map_err(|_| ApiError::bad_request(format!("plan {plan_id} not found")))?;

    let mut waves_created = 0usize;
    let mut tasks_created = 0usize;
    let mut tasks_total = 0i64;

    for (pos, wave) in waves.iter_mut().enumerate() {
        conn.execute(
            "INSERT INTO waves (plan_id, project_id, wave_id, name, status, \
             position, depends_on, estimated_hours) \
             VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7)",
            rusqlite::params![
                plan_id,
                project_id,
                wave.id,
                wave.name,
                pos as i64,
                wave.depends_on,
                wave.estimated_hours,
            ],
        )
        .map_err(|e| ApiError::internal(format!("wave insert failed: {e}")))?;

        let wave_db_id: i64 = conn
            .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
            .map_err(|e| ApiError::internal(format!("rowid failed: {e}")))?;

        waves_created += 1;
        let wave_task_count = wave.tasks.len() as i64;

        for task in &mut wave.tasks {
            // Apply smart defaults before insert (model, validator, verify, effort)
            apply_defaults(task);

            let criteria_json = task
                .test_criteria
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_default();

            // Serialize verify commands as newline-separated string for storage
            let verify_str = if task.verify.is_empty() {
                None
            } else {
                Some(task.verify.join("\n"))
            };

            conn.execute(
                "INSERT INTO tasks (plan_id, project_id, wave_id_fk, wave_id, \
                 task_id, title, status, priority, type, description, \
                 test_criteria, model, assignee, output_type, validator_agent, \
                 effort_level, notes) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?8, ?9, ?10, ?11, ?12, \
                 ?13, ?14, ?15, ?16)",
                rusqlite::params![
                    plan_id,
                    project_id,
                    wave_db_id,
                    wave.id,
                    task.id,
                    task.title,
                    task.priority,
                    task.task_type,
                    task.description,
                    criteria_json,
                    task.model,
                    task.assignee,
                    task.output_type,
                    task.validator_agent,
                    task.effort_level,
                    verify_str,
                ],
            )
            .map_err(|e| ApiError::internal(format!("task insert failed: {e}")))?;

            tasks_created += 1;
        }
        tasks_total += wave_task_count;

        // Update wave task count
        conn.execute(
            "UPDATE waves SET tasks_total = ?1 WHERE id = ?2",
            rusqlite::params![wave_task_count, wave_db_id],
        )
        .map_err(|e| ApiError::internal(format!("wave count update failed: {e}")))?;
    }

    // Update plan task total
    conn.execute(
        "UPDATE plans SET tasks_total = tasks_total + ?1, updated_at = datetime('now') \
         WHERE id = ?2",
        rusqlite::params![tasks_total, plan_id],
    )
    .map_err(|e| ApiError::internal(format!("plan count update failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "waves_created": waves_created,
        "tasks_created": tasks_created,
    })))
}

#[cfg(test)]
mod tests {
    use super::super::api_plan_db_import_parsers::parse_waves;
    use crate::db::PlanDb;
    use serde_json::json;

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
                 CREATE TABLE plans (
                     id INTEGER PRIMARY KEY, project_id TEXT NOT NULL,
                     name TEXT NOT NULL, status TEXT DEFAULT 'draft',
                     tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0,
                     updated_at TEXT
                 );
                 CREATE TABLE waves (
                     id INTEGER PRIMARY KEY, plan_id INTEGER, project_id TEXT,
                     wave_id TEXT, name TEXT, status TEXT DEFAULT 'pending',
                     position INTEGER DEFAULT 0, depends_on TEXT,
                     estimated_hours INTEGER DEFAULT 8,
                     tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0
                 );
                 CREATE TABLE tasks (
                     id INTEGER PRIMARY KEY, plan_id INTEGER, project_id TEXT,
                     wave_id_fk INTEGER, wave_id TEXT, task_id TEXT,
                     title TEXT, status TEXT DEFAULT 'pending',
                     priority TEXT, type TEXT, description TEXT,
                     test_criteria TEXT, model TEXT, assignee TEXT,
                     output_type TEXT, validator_agent TEXT,
                     effort_level INTEGER DEFAULT 1, notes TEXT
                 );
                 INSERT INTO projects (id, name) VALUES ('test', 'Test');
                 INSERT INTO plans (id, project_id, name) VALUES (1, 'test', 'Plan A');",
            )
            .expect("schema");
        db
    }

    #[test]
    fn plan_db_import_json_waves() {
        let body = json!({
            "plan_id": 1,
            "waves": [
                {
                    "id": "W1",
                    "name": "Wave 1",
                    "tasks": [
                        {"id": "T1-01", "title": "Task 1", "priority": "P0"},
                        {"id": "T1-02", "title": "Task 2"}
                    ]
                },
                {
                    "id": "W2",
                    "name": "Wave 2",
                    "depends_on": "W1",
                    "tasks": [
                        {"id": "T2-01", "title": "Task 3"}
                    ]
                }
            ]
        });

        let waves = parse_waves(&body).expect("parse");
        assert_eq!(waves.len(), 2);
        assert_eq!(waves[0].tasks.len(), 2);
        assert_eq!(waves[1].tasks.len(), 1);
        assert_eq!(waves[1].depends_on.as_deref(), Some("W1"));
    }

    #[test]
    fn plan_db_import_yaml_spec() {
        let yaml = "waves:\n  - id: W1\n    name: Wave 1\n    tasks:\n      - id: T1\n        title: First task\n";
        let body = json!({
            "plan_id": 1,
            "spec": yaml,
        });

        let waves = parse_waves(&body).expect("parse yaml");
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0].tasks[0].title, "First task");
    }

    #[test]
    fn plan_db_import_creates_rows() {
        let db = setup_db();
        let conn = db.connection();

        // Simulate import logic
        conn.execute(
            "INSERT INTO waves (plan_id, project_id, wave_id, name, status, position, tasks_total) \
             VALUES (1, 'test', 'W1', 'Wave 1', 'pending', 0, 2)",
            [],
        )
        .unwrap();
        let wave_id: i64 = conn
            .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
            .unwrap();

        conn.execute(
            "INSERT INTO tasks (plan_id, project_id, wave_id_fk, wave_id, task_id, title, priority, type) \
             VALUES (1, 'test', ?1, 'W1', 'T1', 'Task 1', 'P0', 'feature'), \
                    (1, 'test', ?1, 'W1', 'T2', 'Task 2', 'P1', 'feature')",
            rusqlite::params![wave_id],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks WHERE plan_id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 2);
    }
}
