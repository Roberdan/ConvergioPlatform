// Delegation progress API — peer reports progress to coordinator.
// Why: Plan 720 T2-01 — zero traceability for remote execution progress.

use super::state::{ApiError, ServerState};
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};

/// Ensure delegation_progress table exists (idempotent).
fn ensure_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS delegation_progress (
             id            INTEGER PRIMARY KEY AUTOINCREMENT,
             delegation_id TEXT NOT NULL UNIQUE,
             status        TEXT NOT NULL DEFAULT 'running'
                 CHECK(status IN ('running','blocked','done')),
             current_task  TEXT,
             output_summary TEXT,
             updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE INDEX IF NOT EXISTS idx_delegation_progress_id
             ON delegation_progress(delegation_id);",
    )
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route(
            "/api/delegation/:id/progress",
            post(handle_post_progress).get(handle_get_progress),
        )
        .route(
            "/api/delegation/by-plan/:plan_id",
            axum::routing::get(handle_get_by_plan),
        )
}

/// POST /api/delegation/:id/progress
/// Body: {status, current_task?, output_summary?}
/// Upserts a progress record for the given delegation ID.
async fn handle_post_progress(
    State(state): State<ServerState>,
    Path(delegation_id): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let status = body
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::bad_request("missing status"))?;

    if !matches!(status, "running" | "blocked" | "done") {
        return Err(ApiError::bad_request(format!(
            "invalid status '{status}': must be running|blocked|done"
        )));
    }

    let current_task = body.get("current_task").and_then(Value::as_str);
    let output_summary = body.get("output_summary").and_then(Value::as_str);

    let conn = state.get_conn()?;
    ensure_schema(&conn)
        .map_err(|e| ApiError::internal(format!("schema init failed: {e}")))?;

    conn.execute(
        "INSERT INTO delegation_progress
             (delegation_id, status, current_task, output_summary, updated_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))
         ON CONFLICT(delegation_id) DO UPDATE SET
             status         = excluded.status,
             current_task   = excluded.current_task,
             output_summary = excluded.output_summary,
             updated_at     = excluded.updated_at",
        rusqlite::params![delegation_id, status, current_task, output_summary],
    )
    .map_err(|e| ApiError::internal(format!("upsert failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "delegation_id": delegation_id,
        "status": status,
    })))
}

/// GET /api/delegation/:id/progress
/// Returns current progress for the given delegation ID.
/// Returns 404 if the ID is not known.
async fn handle_get_progress(
    State(state): State<ServerState>,
    Path(delegation_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    ensure_schema(&conn)
        .map_err(|e| ApiError::internal(format!("schema init failed: {e}")))?;

    let row: Option<(String, Option<String>, Option<String>, String)> = conn
        .query_row(
            "SELECT status, current_task, output_summary, updated_at
             FROM delegation_progress WHERE delegation_id = ?1",
            rusqlite::params![delegation_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();

    match row {
        None => Err(ApiError::not_found(format!(
            "delegation '{delegation_id}' not found"
        ))),
        Some((status, current_task, output_summary, updated_at)) => Ok(Json(json!({
            "ok": true,
            "delegation_id": delegation_id,
            "status": status,
            "current_task": current_task,
            "output_summary": output_summary,
            "updated_at": updated_at,
        }))),
    }
}

/// GET /api/delegation/by-plan/:plan_id
/// Returns all delegation progress entries whose delegation_id starts with `del-{plan_id}-`.
/// This resolves the mismatch where CLI passes plan_id but the per-delegation endpoint
/// expects a full delegation_id.
async fn handle_get_by_plan(
    State(state): State<ServerState>,
    Path(plan_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    ensure_schema(&conn)
        .map_err(|e| ApiError::internal(format!("schema init failed: {e}")))?;

    let prefix = format!("del-{plan_id}-%");
    let mut stmt = conn
        .prepare(
            "SELECT delegation_id, status, current_task, output_summary, updated_at \
             FROM delegation_progress WHERE delegation_id LIKE ?1 \
             ORDER BY updated_at DESC",
        )
        .map_err(|e| ApiError::internal(format!("query prepare failed: {e}")))?;

    let rows: Vec<Value> = stmt
        .query_map(rusqlite::params![prefix], |r| {
            Ok(json!({
                "delegation_id": r.get::<_, String>(0)?,
                "status": r.get::<_, String>(1)?,
                "current_task": r.get::<_, Option<String>>(2)?,
                "output_summary": r.get::<_, Option<String>>(3)?,
                "last_update": r.get::<_, String>(4)?,
            }))
        })
        .map_err(|e| ApiError::internal(format!("query failed: {e}")))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "delegations": rows,
    })))
}

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE delegation_progress (
                     id            INTEGER PRIMARY KEY AUTOINCREMENT,
                     delegation_id TEXT NOT NULL UNIQUE,
                     status        TEXT NOT NULL DEFAULT 'running'
                         CHECK(status IN ('running','blocked','done')),
                     current_task  TEXT,
                     output_summary TEXT,
                     updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
                 );
                 CREATE INDEX IF NOT EXISTS idx_delegation_progress_id
                     ON delegation_progress(delegation_id);",
            )
            .expect("schema");
        db
    }

    #[test]
    fn upsert_progress_replaces_existing() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO delegation_progress (delegation_id, status, current_task)
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["del-1", "running", "step-1"],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO delegation_progress
                 (delegation_id, status, current_task, output_summary, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(delegation_id) DO UPDATE SET
                 status         = excluded.status,
                 current_task   = excluded.current_task,
                 output_summary = excluded.output_summary,
                 updated_at     = excluded.updated_at",
            rusqlite::params!["del-1", "done", "step-2", "ok"],
        )
        .unwrap();

        let (status, task): (String, String) = conn
            .query_row(
                "SELECT status, current_task FROM delegation_progress WHERE delegation_id='del-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();

        assert_eq!(status, "done");
        assert_eq!(task, "step-2");
    }

    #[test]
    fn get_returns_none_for_unknown_id() {
        let db = setup_db();
        let conn = db.connection();

        let result: Option<String> = conn
            .query_row(
                "SELECT status FROM delegation_progress WHERE delegation_id = ?1",
                rusqlite::params!["nope"],
                |r| r.get(0),
            )
            .ok();

        assert!(result.is_none());
    }

    #[test]
    fn by_plan_returns_matching_delegations() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO delegation_progress (delegation_id, status, current_task) \
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["del-742-M1Pro-1234", "running", "T1-01"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO delegation_progress (delegation_id, status, current_task) \
             VALUES (?1, ?2, ?3)",
            rusqlite::params!["del-742-M5Max-5678", "done", "T2-01"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO delegation_progress (delegation_id, status) \
             VALUES (?1, ?2)",
            rusqlite::params!["del-999-other-0", "running"],
        )
        .unwrap();

        // Query by plan_id=742 should return 2 rows
        let mut stmt = conn
            .prepare(
                "SELECT delegation_id FROM delegation_progress \
                 WHERE delegation_id LIKE ?1 ORDER BY updated_at DESC",
            )
            .unwrap();
        let rows: Vec<String> = stmt
            .query_map(rusqlite::params!["del-742-%"], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.starts_with("del-742-")));
    }

    #[test]
    fn invalid_status_rejected() {
        let db = setup_db();
        let conn = db.connection();

        let err = conn
            .execute(
                "INSERT INTO delegation_progress (delegation_id, status) VALUES (?1, ?2)",
                rusqlite::params!["bad", "unknown"],
            )
            .unwrap_err();

        assert!(err.to_string().contains("CHECK"), "constraint must fire: {err}");
    }
}
