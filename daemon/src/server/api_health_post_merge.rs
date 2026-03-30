// POST /api/health/post-merge-check — trigger health-based auto-rollback check.
// Runs cargo test --lib and GET /api/health/deep for the given task + worktree.
// Rolls back and marks task blocked on regression.

use super::state::{ApiError, ServerState};
use crate::orchestrator::auto_rollback;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/health/post-merge-check", post(post_merge_check_handler))
}

#[derive(Deserialize)]
struct PostMergeCheckBody {
    task_id: i64,
    worktree_path: String,
}

async fn post_merge_check_handler(
    State(state): State<ServerState>,
    Json(body): Json<PostMergeCheckBody>,
) -> Result<Json<Value>, ApiError> {
    if body.worktree_path.trim().is_empty() {
        return Err(ApiError::bad_request("worktree_path must not be empty"));
    }

    let conn = state.get_conn()?;
    let wt_path = PathBuf::from(&body.worktree_path);

    let health = auto_rollback::check_health_after_merge(&conn, body.task_id, &wt_path)
        .map_err(|e| ApiError::internal(format!("post-merge check failed: {e}")))?;

    let rolled_back = health.to_string() != "healthy";
    Ok(Json(json!({
        "ok": true,
        "task_id": body.task_id,
        "health_status": health.to_string(),
        "rolled_back": rolled_back,
    })))
}
