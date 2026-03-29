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
    #[tracing::instrument(skip_all)]
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
    #[tracing::instrument(skip_all)]
async fn handle_get_progress(
    State(state): State<ServerState>,
    Path(delegation_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    ensure_schema(&conn)
        .map_err(|e| ApiError::internal(format!("schema init failed: {e}")))?;

    let row: Option<(String, Option<String>, Option<String>, String)> = match conn.query_row(
        "SELECT status, current_task, output_summary, updated_at
         FROM delegation_progress WHERE delegation_id = ?1",
        rusqlite::params![delegation_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ) {
        Ok(v) => Some(v),
        Err(e) => { tracing::debug!("delegation progress query for {delegation_id}: {e}"); None }
    };

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
    #[tracing::instrument(skip_all)]
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
        .filter_map(|r| match r {
            Ok(v) => Some(v),
            Err(e) => { tracing::warn!("delegation row decode: {e}"); None }
        })
        .collect();

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "delegations": rows,
    })))
}

#[cfg(test)]
#[path = "api_delegation_unit_tests.rs"]
mod tests;
