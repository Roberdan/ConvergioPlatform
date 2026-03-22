// Copyright (c) 2026 Roberto D'Angelo
//! Plan checkpoint endpoints — serialize plan state to file and restore it.
//! Mirrors the plan-checkpoint.sh behaviour via the daemon API.
use super::state::{query_one, query_rows, ApiError, ServerState};
use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/plan-db/checkpoint/save", post(handle_checkpoint_save))
        .route(
            "/api/plan-db/checkpoint/restore",
            get(handle_checkpoint_restore),
        )
}

fn checkpoint_path(plan_id: i64) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".claude/data/checkpoints")
        .join(format!("plan-{plan_id}.json"))
}

/// POST /api/plan-db/checkpoint/save — serialize current plan state to JSON file
/// Body: {plan_id}
async fn handle_checkpoint_save(
    State(state): State<ServerState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let plan_id = body
        .get("plan_id")
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request("missing plan_id"))?;

    let conn = state.get_conn()?;
    let conn = &conn;

    let plan = query_one(
        conn,
        "SELECT id, name, status, project_id, execution_host, worktree_path \
         FROM plans WHERE id = ?1",
        rusqlite::params![plan_id],
    )?
    .ok_or_else(|| ApiError::bad_request(format!("plan {plan_id} not found")))?;

    let waves = query_rows(
        conn,
        "SELECT id, wave_id, name, status, worktree_path, branch_name \
         FROM waves WHERE plan_id = ?1 ORDER BY id",
        rusqlite::params![plan_id],
    )?;

    let tasks = query_rows(
        conn,
        "SELECT id, task_id, title, status, wave_id_fk \
         FROM tasks WHERE plan_id = ?1 ORDER BY wave_id_fk, id",
        rusqlite::params![plan_id],
    )?;

    let checkpoint = json!({
        "plan_id": plan_id,
        "saved_at": chrono_now(),
        "plan": plan,
        "waves": waves,
        "tasks": tasks,
    });

    let path = checkpoint_path(plan_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ApiError::internal(format!("checkpoint dir error: {e}")))?;
    }
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&checkpoint)
            .map_err(|e| ApiError::internal(format!("serialize failed: {e}")))?,
    )
    .map_err(|e| ApiError::internal(format!("checkpoint write failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "path": path.to_string_lossy(),
        "waves": waves.len(),
        "tasks": tasks.len(),
    })))
}

#[derive(Deserialize)]
struct CheckpointRestoreQuery {
    plan_id: i64,
}

/// GET /api/plan-db/checkpoint/restore?plan_id=N — read checkpoint file for context injection
async fn handle_checkpoint_restore(
    State(_state): State<ServerState>,
    Query(params): Query<CheckpointRestoreQuery>,
) -> Result<Json<Value>, ApiError> {
    let plan_id = params.plan_id;
    let path = checkpoint_path(plan_id);

    if !path.exists() {
        return Err(ApiError::bad_request(format!(
            "no checkpoint for plan {plan_id}"
        )));
    }

    let raw = std::fs::read_to_string(&path)
        .map_err(|e| ApiError::internal(format!("checkpoint read failed: {e}")))?;

    let checkpoint: Value = serde_json::from_str(&raw)
        .map_err(|e| ApiError::internal(format!("checkpoint parse failed: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "plan_id": plan_id,
        "path": path.to_string_lossy(),
        "checkpoint": checkpoint,
    })))
}

/// Minimal datetime without chrono dep — uses SQLite-compatible format
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as ISO 8601 without external deps
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    // Approximate date from epoch (good enough for a checkpoint timestamp)
    format!("epoch+{days}d {h:02}:{m:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::checkpoint_path;

    #[test]
    fn checkpoint_path_is_in_home_checkpoints_dir() {
        let path = checkpoint_path(42);
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains(".claude/data/checkpoints"),
            "expected checkpoints dir, got: {path_str}"
        );
        assert!(
            path_str.ends_with("plan-42.json"),
            "expected plan-42.json, got: {path_str}"
        );
    }

    #[test]
    fn checkpoint_path_varies_by_plan_id() {
        let p1 = checkpoint_path(1);
        let p2 = checkpoint_path(2);
        assert_ne!(p1, p2);
    }
}
