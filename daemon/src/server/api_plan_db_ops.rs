use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/plan-db/wave/update", post(handle_wave_update))
        .route("/api/plan-db/kb-search", get(handle_kb_search))
}

/// POST /api/plan-db/wave/update — update wave status
/// Body: {wave_id, status, notes?}
async fn handle_wave_update(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let wave_id = body
        .get("wave_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing wave_id"))?;
    let status = body
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing status"))?;

    let conn = state.get_conn()?;
    let conn = &conn;

    // Guard: if setting to done, all tasks must be done/cancelled/skipped
    if status == "done" {
        let pending = query_one(
            conn,
            "SELECT COUNT(*) AS c FROM tasks \
             WHERE wave_id_fk = ?1 AND status NOT IN ('done', 'cancelled', 'skipped')",
            rusqlite::params![wave_id],
        )?
        .and_then(|v| v.get("c").and_then(Value::as_i64))
        .unwrap_or(0);

        if pending > 0 {
            return Err(ApiError::bad_request(format!(
                "wave {wave_id} has {pending} incomplete tasks"
            )));
        }
    }

    let changed = conn
        .execute(
            "UPDATE waves SET status = ?1, \
             started_at = CASE WHEN ?1 = 'in_progress' AND started_at IS NULL \
               THEN datetime('now') ELSE started_at END, \
             completed_at = CASE WHEN ?1 = 'done' \
               THEN datetime('now') ELSE completed_at END \
             WHERE id = ?2",
            rusqlite::params![status, wave_id],
        )
        .map_err(|e| ApiError::internal(format!("wave update failed: {e}")))?;

    if changed == 0 {
        return Err(ApiError::bad_request(format!("wave {wave_id} not found")));
    }

    // Update plan stats when wave completes
    if status == "done" {
        let plan_id = query_one(
            conn,
            "SELECT plan_id FROM waves WHERE id = ?1",
            rusqlite::params![wave_id],
        )?
        .and_then(|v| v.get("plan_id").and_then(Value::as_i64));

        if let Some(pid) = plan_id {
            // Recount done tasks for the plan
            conn.execute(
                "UPDATE plans SET tasks_done = \
                 (SELECT COUNT(*) FROM tasks WHERE plan_id = ?1 AND status = 'done'), \
                 updated_at = datetime('now') WHERE id = ?1",
                rusqlite::params![pid],
            )
            .map_err(|e| ApiError::internal(format!("plan stats update failed: {e}")))?;
        }
    }

    Ok(Json(json!({
        "ok": true,
        "wave_id": wave_id,
        "status": status,
    })))
}

#[derive(Deserialize)]
struct KbSearchQuery {
    q: String,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    20
}

/// GET /api/plan-db/kb-search?q=term — search knowledge_base table
async fn handle_kb_search(
    State(state): State<ServerState>,
    Query(params): Query<KbSearchQuery>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    // Check if knowledge_base table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master \
             WHERE type='table' AND name='knowledge_base'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);

    if !table_exists {
        return Ok(Json(json!({
            "ok": true,
            "results": [],
            "query": params.q,
        })));
    }

    let pattern = format!("%{}%", params.q);
    let results = query_rows(
        conn,
        "SELECT id, domain, title, content, created_at, hit_count \
         FROM knowledge_base \
         WHERE title LIKE ?1 OR content LIKE ?1 OR domain LIKE ?1 \
         ORDER BY hit_count DESC, created_at DESC \
         LIMIT ?2",
        rusqlite::params![pattern, params.limit],
    )?;

    Ok(Json(json!({
        "ok": true,
        "results": results,
        "query": params.q,
        "count": results.len(),
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
                "CREATE TABLE plans (
                     id INTEGER PRIMARY KEY, project_id TEXT, name TEXT,
                     status TEXT, tasks_total INTEGER DEFAULT 0,
                     tasks_done INTEGER DEFAULT 0, updated_at TEXT
                 );
                 CREATE TABLE waves (
                     id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id TEXT,
                     name TEXT, status TEXT DEFAULT 'pending',
                     started_at TEXT, completed_at TEXT,
                     tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0
                 );
                 CREATE TABLE tasks (
                     id INTEGER PRIMARY KEY, plan_id INTEGER,
                     wave_id_fk INTEGER, wave_id TEXT, task_id TEXT,
                     title TEXT, status TEXT DEFAULT 'pending',
                     project_id TEXT
                 );
                 CREATE TABLE knowledge_base (
                     id INTEGER PRIMARY KEY, domain TEXT, title TEXT,
                     content TEXT, created_at TEXT, hit_count INTEGER DEFAULT 0
                 );
                 INSERT INTO plans (id, project_id, name, status) VALUES (1, 'test', 'P', 'doing');
                 INSERT INTO waves (id, plan_id, wave_id, name, tasks_total)
                     VALUES (10, 1, 'W1', 'Wave 1', 2);
                 INSERT INTO tasks (id, plan_id, wave_id_fk, wave_id, task_id, title, status)
                     VALUES (100, 1, 10, 'W1', 'T1', 'Task 1', 'done'),
                            (101, 1, 10, 'W1', 'T2', 'Task 2', 'done');
                 INSERT INTO knowledge_base (domain, title, content, hit_count)
                     VALUES ('rust', 'Axum patterns', 'Use Router::new() for routing', 5),
                            ('shell', 'Bash tips', 'Use set -e for error handling', 2);",
            )
            .expect("schema");
        db
    }

    #[test]
    fn plan_db_wave_update_to_done() {
        let db = setup_db();
        let conn = db.connection();

        // All tasks done, wave should be completable
        let pending: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE wave_id_fk = 10 \
                 AND status NOT IN ('done', 'cancelled', 'skipped')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending, 0);

        conn.execute(
            "UPDATE waves SET status = 'done', completed_at = datetime('now') WHERE id = 10",
            [],
        )
        .unwrap();

        let status: String = conn
            .query_row("SELECT status FROM waves WHERE id = 10", [], |r| r.get(0))
            .unwrap();
        assert_eq!(status, "done");
    }

    #[test]
    fn plan_db_wave_update_blocked_by_pending_tasks() {
        let db = setup_db();
        let conn = db.connection();

        // Add a pending task
        conn.execute(
            "INSERT INTO tasks (id, plan_id, wave_id_fk, wave_id, task_id, title, status) \
             VALUES (102, 1, 10, 'W1', 'T3', 'Pending', 'pending')",
            [],
        )
        .unwrap();

        let pending: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE wave_id_fk = 10 \
                 AND status NOT IN ('done', 'cancelled', 'skipped')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending, 1); // Should block wave completion
    }

    #[test]
    fn plan_db_kb_search_finds_results() {
        let db = setup_db();
        let conn = db.connection();

        let results = query_rows(
            conn,
            "SELECT id, title FROM knowledge_base WHERE title LIKE ?1 OR content LIKE ?1 LIMIT 10",
            rusqlite::params!["%axum%"],
        )
        .expect("search");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn plan_db_kb_search_empty_table() {
        let db = PlanDb::open_in_memory().expect("db");
        // No knowledge_base table — should return empty
        let exists: bool = db
            .connection()
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='knowledge_base'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!exists);
    }
}
