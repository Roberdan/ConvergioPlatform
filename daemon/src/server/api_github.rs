// api_github: router + commits/events/repo-create handlers
// stats handler + gh CLI helpers → api_github_handlers.rs
use super::api_github_handlers::handle_github_stats;
use super::state::{query_rows, ApiError, ServerState};
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/github/commits/:plan_id", get(handle_github_commits))
        .route("/api/github/events/:project_id", get(handle_github_events))
        .route("/api/github/stats/:plan_id", get(handle_github_stats))
        .route("/api/github/repo/create", post(handle_github_repo_create))
}

async fn handle_github_commits(
    State(state): State<ServerState>,
    Path(plan_id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let local_commits = query_rows(
        &conn,
        "SELECT commit_sha, commit_message, lines_added, lines_removed, files_changed, authored_at, created_at FROM plan_commits WHERE plan_id=?1 ORDER BY COALESCE(authored_at, created_at) DESC LIMIT 50",
        rusqlite::params![plan_id],
    )?;
    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "repo": "local/repo",
        "local_commits": local_commits,
        "remote_commits": []
    })))
}

async fn handle_github_events(
    State(state): State<ServerState>,
    Path(project_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let conn = state.get_conn()?;
    let local_events = query_rows(
        &conn,
        "SELECT ge.id, ge.plan_id, ge.event_type, ge.status, ge.created_at FROM github_events ge JOIN plans p ON p.id=ge.plan_id WHERE p.project_id=?1 ORDER BY ge.created_at DESC LIMIT 100",
        rusqlite::params![project_id],
    )
    .unwrap_or_default();
    Ok(Json(
        json!({"ok": true, "project_id": project_id, "repo": "local/repo", "local_events": local_events, "remote_events": []}),
    ))
}

async fn handle_github_repo_create(axum::Json(payload): axum::Json<Value>) -> Json<Value> {
    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if name.is_empty() {
        return Json(json!({"ok": false, "error": "missing repository name"}));
    }
    Json(
        json!({"ok": true, "repo": {"nameWithOwner": name, "url": "", "isPrivate": true}, "create_output": ""}),
    )
}
