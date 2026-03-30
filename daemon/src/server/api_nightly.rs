// POST /api/nightly/run — trigger autonomous nightly cleanup + evaluation job.
use crate::orchestrator::nightly;
use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};
use std::env;
use std::path::PathBuf;

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/nightly/run", post(handle_nightly_run))
}

async fn handle_nightly_run(
    State(state): State<ServerState>,
) -> Result<Json<Value>, ApiError> {
    let db_path = state.db_path.clone();
    let platform_dir = platform_dir(&db_path);

    // Run in a blocking task to avoid starving the async runtime (cargo test, git gc, etc.)
    let report = tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current()
            .block_on(nightly::run_nightly(&db_path, &platform_dir))
    })
    .await
    .map_err(|e| ApiError::internal(format!("nightly task panicked: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "date": report.date,
        "host": report.host,
        "status": report.status,
        "cleanup": {
            "worktrees_pruned": report.cleanup.worktrees_pruned,
            "zombies_killed": report.cleanup.zombies_killed,
            "stale_agents_removed": report.cleanup.stale_agents_removed,
            "evidence_cleared": report.cleanup.evidence_files_cleared,
            "git_gc_ok": report.cleanup.git_gc_ok,
            "branches_pruned": report.cleanup.branches_pruned,
        },
        "evaluation": {
            "commits_today": report.evaluation.commits_today,
            "fix_chains": report.evaluation.fix_chains.len(),
            "failed_log_entries": report.evaluation.failed_tests_in_log,
            "agents_over_limit": report.evaluation.agents_over_limit.len(),
            "stale_memory_files": report.evaluation.stale_memory_files.len(),
            "test_health_passed": report.evaluation.test_health.passed,
            "outdated_deps": report.evaluation.outdated_deps.len(),
        },
    })))
}

fn platform_dir(db_path: &std::path::Path) -> PathBuf {
    if let Ok(dir) = env::var("CONVERGIO_PLATFORM_DIR") {
        return PathBuf::from(dir);
    }
    nightly::platform_dir_from_db(db_path)
}
