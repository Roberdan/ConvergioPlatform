// HTTP handlers for the rollback API.
// POST /api/rollback/:task_id          — restore latest snapshot for a task
// GET  /api/rollback/snapshots         — list snapshots (?task_id=<id>)

use super::state::{ApiError, ServerState};
use crate::orchestrator::rollback;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

pub fn router() -> Router<ServerState> {
    Router::new()
        // Static route must come before the dynamic param route so Axum
        // resolves GET /api/rollback/snapshots before trying :task_id.
        .route("/api/rollback/snapshots", get(list_snapshots_handler))
        .route("/api/rollback/:task_id", post(restore_handler))
}

#[derive(Deserialize)]
struct RestoreBody {
    worktree_path: String,
}

async fn restore_handler(
    State(state): State<ServerState>,
    Path(task_id): Path<i64>,
    Json(body): Json<RestoreBody>,
) -> Result<Json<Value>, ApiError> {
    if body.worktree_path.trim().is_empty() {
        return Err(ApiError::bad_request("worktree_path must not be empty"));
    }

    let conn = state.get_conn()?;
    let path = PathBuf::from(&body.worktree_path);

    rollback::restore_snapshot(&conn, task_id, &path)
        .map_err(|e| ApiError::internal(format!("restore failed: {e}")))?;

    Ok(Json(json!({ "ok": true, "task_id": task_id })))
}

#[derive(Deserialize)]
struct SnapshotsQuery {
    task_id: i64,
}

async fn list_snapshots_handler(
    State(state): State<ServerState>,
    Query(q): Query<SnapshotsQuery>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;

    let items = rollback::list_snapshots(&conn, q.task_id)
        .map_err(|e| ApiError::internal(format!("query failed: {e}")))?;

    Ok(Json(json!({ "items": items })))
}
