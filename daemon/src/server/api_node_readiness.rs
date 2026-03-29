// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
//! GET /api/node/readiness — node-level readiness checks for all swarm nodes.
// Check functions live in api_node_readiness_checks.rs to stay ≤250 lines.

#[path = "api_node_readiness_checks.rs"]
mod checks;

use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/node/readiness", get(handle_node_readiness))
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

impl Check {
    pub fn pass(name: &str, detail: impl Into<String>) -> Self {
        Self { name: name.into(), passed: true, detail: detail.into() }
    }
    pub fn fail(name: &str, detail: impl Into<String>) -> Self {
        Self { name: name.into(), passed: false, detail: detail.into() }
    }
}

#[derive(Debug, Serialize)]
pub struct NodeReadinessResponse {
    pub ok: bool,
    pub node: String,
    pub role: String,
    pub checks: Vec<Check>,
}

fn default_db_path() -> std::path::PathBuf {
    std::env::var("DASHBOARD_DB").map(std::path::PathBuf::from).unwrap_or_else(|_| {
        std::path::PathBuf::from(format!("{}/.claude/data/dashboard.db", checks::home()))
    })
}

fn build_checks(db: &std::path::Path) -> Vec<Check> {
    vec![
        checks::check_mlx_lm(),
        checks::check_python_venv(),
        checks::check_db_path(db),
        checks::check_db_symlink(db),
        checks::check_telegram_token(),
        checks::check_disk_space(&checks::home()),
        checks::check_models_downloaded(),
        checks::check_daemon_version(),
        checks::check_node_role(),
        checks::check_role_capabilities(),
    ]
}

/// Run all checks — exposed for unit tests (no live ServerState required).
pub fn run_checks() -> Vec<Check> {
    let db = default_db_path();
    build_checks(&db)
}

/// GET /api/node/readiness
    #[tracing::instrument(skip_all)]
async fn handle_node_readiness(
    State(state): State<ServerState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let node = checks::gethostname();
    let (role, _) = checks::parse_peers_conf();
    let check_results = build_checks(&state.db_path);
    let ok = check_results.iter().all(|c| c.passed);
    Ok(Json(serde_json::json!({ "ok": ok, "node": node, "role": role, "checks": check_results })))
}

#[cfg(test)]
#[path = "api_node_readiness_tests.rs"]
mod tests;
