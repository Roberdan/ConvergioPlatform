use super::state::{ApiError, ServerState};
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/digest/ci", get(handle_ci_digest))
        .route("/api/digest/pr", get(handle_pr_digest))
}

#[derive(Deserialize)]
struct DigestQuery {
    cwd: Option<String>,
    mode: Option<String>,
    pr: Option<i64>,
    run_id: Option<i64>,
    compact: Option<bool>,
    no_cache: Option<bool>,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")))
        .to_path_buf()
}

fn run_digest(script: &str, query: &DigestQuery) -> Result<Value, ApiError> {
    let script_path = repo_root().join("claude-config/scripts").join(script);
    let mut cmd = Command::new("bash");
    cmd.arg(script_path).env("DIGEST_API_BYPASS", "1");
    if let Some(cwd) = &query.cwd {
        cmd.current_dir(cwd);
    }
    if query.no_cache.unwrap_or(false) {
        cmd.arg("--no-cache");
    }
    if query.compact.unwrap_or(false) {
        cmd.arg("--compact");
    }
    match script {
        "ci-digest.sh" => {
            if matches!(query.mode.as_deref(), Some("checks")) {
                cmd.arg("checks");
                if let Some(pr) = query.pr {
                    cmd.arg(pr.to_string());
                }
            } else if let Some(run_id) = query.run_id {
                cmd.arg(run_id.to_string());
            }
        }
        "pr-digest.sh" => {
            if let Some(pr) = query.pr {
                cmd.arg(pr.to_string());
            }
        }
        _ => {}
    }
    let output = cmd
        .output()
        .map_err(|e| ApiError::internal(format!("digest exec failed: {e}")))?;
    if !output.status.success() {
        return Err(ApiError::internal(format!(
            "digest exited with {:?}",
            output.status.code()
        )));
    }
    serde_json::from_slice::<Value>(&output.stdout)
        .map_err(|e| ApiError::internal(format!("digest JSON parse failed: {e}")))
}

async fn handle_ci_digest(
    State(_state): State<ServerState>,
    Query(query): Query<DigestQuery>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(run_digest("ci-digest.sh", &query)?))
}

async fn handle_pr_digest(
    State(_state): State<ServerState>,
    Query(query): Query<DigestQuery>,
) -> Result<Json<Value>, ApiError> {
    Ok(Json(run_digest("pr-digest.sh", &query)?))
}
