pub mod dispatch;

use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .merge(dispatch::router())
        .route("/api/workers", get(handle_list_workers))
        .route("/api/workers/launch", post(handle_launch_worker))
        .route("/api/workers/status", get(handle_worker_status))
}

/// GET /api/workers — list active worker processes
async fn handle_list_workers(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let workers = query_rows(
        &conn,
        "SELECT id, agent_id, agent_type, model, status, started_at, \
         host, description, plan_id, task_db_id \
         FROM agent_activity \
         WHERE status = 'running' \
         ORDER BY started_at DESC",
        [],
    )?;

    Ok(Json(json!({
        "ok": true,
        "workers": workers,
        "count": workers.len(),
    })))
}

/// POST /api/workers/launch — launch a new worker process
/// Body: {agent_type, command?, plan_id?, task_id?, model?}
async fn handle_launch_worker(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let agent_type = body
        .get("agent_type")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing agent_type"))?;
    let plan_id = body.get("plan_id").and_then(Value::as_i64);
    let task_id = body.get("task_id").and_then(Value::as_i64);
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("default");
    let description = body
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");

    let agent_id = format!(
        "{}-{}-{}",
        agent_type,
        plan_id.unwrap_or(0),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );

    let host = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let conn = state.get_conn()?;
    conn.execute(
        "INSERT OR REPLACE INTO agent_activity \
             (agent_id, agent_type, description, plan_id, task_db_id, \
              model, host, status, started_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'running', datetime('now'))",
        rusqlite::params![
            agent_id,
            agent_type,
            description,
            plan_id,
            task_id,
            model,
            host
        ],
    )
    .map_err(|e| ApiError::internal(format!("launch failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "agent_id": agent_id,
        "agent_type": agent_type,
        "status": "running",
    })))
}

/// GET /api/workers/status — summary of worker activity
async fn handle_worker_status(State(state): State<ServerState>) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let conn = &conn;

    let running = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM agent_activity WHERE status = 'running'",
        [],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    let completed_today = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM agent_activity \
         WHERE status = 'completed' AND completed_at >= date('now')",
        [],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    Ok(Json(json!({
        "ok": true,
        "running": running,
        "completed_today": completed_today,
    })))
}

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;
    use crate::server::state::query_rows;

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE plans (
                     id INTEGER PRIMARY KEY, project_id TEXT, name TEXT,
                     status TEXT, execution_host TEXT, updated_at TEXT
                 );
                 CREATE TABLE coordinator_events (
                     id INTEGER PRIMARY KEY, event_type TEXT NOT NULL DEFAULT '',
                     payload TEXT, source_node TEXT,
                     handled_at TEXT DEFAULT (datetime('now'))
                 );
                 CREATE TABLE agent_activity (
                     id INTEGER PRIMARY KEY AUTOINCREMENT,
                     agent_id TEXT NOT NULL UNIQUE, agent_type TEXT,
                     description TEXT, plan_id INTEGER, task_db_id INTEGER,
                     model TEXT, host TEXT, status TEXT DEFAULT 'running',
                     started_at TEXT DEFAULT (datetime('now')),
                     completed_at TEXT
                 );
                 INSERT INTO plans VALUES (1, 'test', 'Plan A', 'doing', NULL, NULL);",
            )
            .expect("schema");
        db
    }

    #[test]
    fn api_workers_launch_and_list() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO agent_activity (agent_id, agent_type, status, model) \
             VALUES ('w1', 'copilot', 'running', 'sonnet')",
            [],
        )
        .unwrap();

        let workers = query_rows(
            conn,
            "SELECT agent_id FROM agent_activity WHERE status = 'running'",
            [],
        )
        .unwrap();
        assert_eq!(workers.len(), 1);
    }
}
