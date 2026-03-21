// api_plan_db_query_fmt: execution tree formatter handler + tests
use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

/// GET /api/plan-db/execution-tree/:plan_id — nested plan+waves+tasks
pub async fn handle_execution_tree(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let plan = query_one(
        conn,
        "SELECT id, name, status, project_id, tasks_total, tasks_done, \
         execution_host, worktree_path, description, human_summary \
         FROM plans WHERE id = ?1",
        rusqlite::params![plan_id],
    )?
    .ok_or_else(|| ApiError::bad_request(format!("plan {plan_id} not found")))?;

    let waves = query_rows(
        conn,
        "SELECT id, wave_id, name, status, tasks_done, tasks_total, \
         position, depends_on, worktree_path \
         FROM waves WHERE plan_id = ?1 ORDER BY position, id",
        rusqlite::params![plan_id],
    )?;

    let tasks = query_rows(
        conn,
        "SELECT id, task_id, title, status, priority, type, \
         wave_id_fk, wave_id, assignee, test_criteria, description, \
         started_at, completed_at, validated_by, executor_host, model \
         FROM tasks WHERE plan_id = ?1 ORDER BY wave_id_fk, id",
        rusqlite::params![plan_id],
    )?;

    // Build nested structure: waves with their tasks
    let tree: Vec<Value> = waves
        .into_iter()
        .map(|wave| {
            let wave_id = wave.get("id").and_then(Value::as_i64).unwrap_or(0);
            let wave_tasks: Vec<&Value> = tasks
                .iter()
                .filter(|t| t.get("wave_id_fk").and_then(Value::as_i64).unwrap_or(-1) == wave_id)
                .collect();
            json!({
                "wave": wave,
                "tasks": wave_tasks,
            })
        })
        .collect();

    Ok(Json(json!({
        "ok": true,
        "plan": plan,
        "tree": tree,
    })))
}

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;
    use crate::server::state::{query_one, query_rows};

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
                 CREATE TABLE plans (
                     id INTEGER PRIMARY KEY, project_id TEXT NOT NULL,
                     name TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'draft',
                     tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0,
                     execution_host TEXT, worktree_path TEXT, description TEXT,
                     human_summary TEXT, parallel_mode TEXT,
                     source_file TEXT, created_at TEXT, started_at TEXT,
                     completed_at TEXT, updated_at TEXT
                 );
                 CREATE TABLE waves (
                     id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id TEXT,
                     name TEXT, status TEXT DEFAULT 'pending',
                     tasks_done INTEGER DEFAULT 0, tasks_total INTEGER DEFAULT 0,
                     position INTEGER DEFAULT 0, depends_on TEXT,
                     worktree_path TEXT
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
                 INSERT INTO projects (id, name) VALUES ('test', 'Test');
                 INSERT INTO plans (id, project_id, name, status, tasks_total, tasks_done)
                     VALUES (1, 'test', 'Plan A', 'doing', 3, 1);
                 INSERT INTO waves (id, plan_id, wave_id, name, status, position, tasks_total, tasks_done)
                     VALUES (10, 1, 'W1', 'Wave 1', 'in_progress', 1, 3, 1);
                 INSERT INTO tasks (id, project_id, plan_id, wave_id_fk, wave_id, task_id, title, status, priority)
                     VALUES (100, 'test', 1, 10, 'W1', 'T1', 'Done task', 'done', 'P0'),
                            (101, 'test', 1, 10, 'W1', 'T2', 'Pending task', 'pending', 'P1'),
                            (102, 'test', 1, 10, 'W1', 'T3', 'In progress', 'in_progress', 'P0');",
            )
            .expect("schema");
        db
    }

    #[test]
    fn plan_db_query_list_active_plans() {
        let db = setup_db();
        let plans = query_rows(
            db.connection(),
            "SELECT id, name, status FROM plans \
             WHERE status NOT IN ('completed', 'cancelled') ORDER BY id DESC",
            [],
        )
        .expect("list");
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].get("name").unwrap().as_str().unwrap(), "Plan A");
    }

    #[test]
    fn plan_db_query_execution_tree_nests_tasks() {
        let db = setup_db();
        let conn = db.connection();
        let waves = query_rows(
            conn,
            "SELECT id, wave_id, name, status FROM waves WHERE plan_id = 1",
            [],
        )
        .expect("waves");
        let tasks = query_rows(
            conn,
            "SELECT id, task_id, title, status, wave_id_fk FROM tasks WHERE plan_id = 1",
            [],
        )
        .expect("tasks");
        assert_eq!(waves.len(), 1);
        assert_eq!(tasks.len(), 3);
        // All tasks belong to wave 10
        for t in &tasks {
            assert_eq!(t.get("wave_id_fk").unwrap().as_i64().unwrap(), 10);
        }
    }

    #[test]
    fn plan_db_query_drift_check_finds_stale() {
        let db = setup_db();
        let conn = db.connection();
        // Make task 102 stale (started 48 hours ago)
        conn.execute(
            "UPDATE tasks SET started_at = datetime('now', '-48 hours') WHERE id = 102",
            [],
        )
        .unwrap();

        let stale = query_rows(
            conn,
            "SELECT id FROM tasks WHERE plan_id = 1 AND status = 'in_progress' \
             AND started_at < datetime('now', '-24 hours')",
            [],
        )
        .expect("stale");
        assert_eq!(stale.len(), 1);
    }

    #[test]
    fn plan_db_query_validate_task_returns_info() {
        let db = setup_db();
        let conn = db.connection();
        let task = query_one(
            conn,
            "SELECT id, task_id, status, validated_by FROM tasks WHERE id = 100 AND plan_id = 1",
            [],
        )
        .expect("query")
        .expect("task exists");
        assert_eq!(task.get("status").unwrap().as_str().unwrap(), "done");
    }
}
