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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BootReadinessSummary {
    pub blocking_failures: Vec<String>,
    pub warning_failures: Vec<String>,
}

/// Full boot readiness report — returned by `run_boot_checks`.
#[derive(Debug, Serialize)]
pub struct ReadinessReport {
    pub ok: bool,
    pub node: String,
    pub role: String,
    pub checks: Vec<Check>,
    pub summary: BootReadinessSummary,
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

/// Run all readiness checks for a specific db_path and return a full report.
/// Called at daemon boot (before binding) to enforce fail-loud startup policy.
pub fn run_boot_checks(db_path: &std::path::Path) -> ReadinessReport {
    let check_results = build_checks(db_path);
    let summary = summarize_for_boot(&check_results);
    let ok = check_results.iter().all(|c| c.passed);
    let node = checks::gethostname();
    let (role, _) = checks::parse_peers_conf();
    ReadinessReport { ok, node, role, checks: check_results, summary }
}

pub fn summarize_for_boot(checks: &[Check]) -> BootReadinessSummary {
    let mut blocking_failures = Vec::new();
    let mut warning_failures = Vec::new();
    for check in checks.iter().filter(|check| !check.passed) {
        let detail = format!("{}: {}", check.name, check.detail);
        if is_boot_blocking(check) {
            blocking_failures.push(detail);
        } else {
            warning_failures.push(detail);
        }
    }
    BootReadinessSummary { blocking_failures, warning_failures }
}

fn is_boot_blocking(check: &Check) -> bool {
    // v20 fail-loud: missing/corrupt DB and missing role are hard blockers.
    match check.name.as_str() {
        "disk_space" | "node_role" | "db_exists" => true,
        _ => false,
    }
}

/// GET /api/node/readiness — serves live checks; falls back to cached boot result.
    #[tracing::instrument(skip_all)]
async fn handle_node_readiness(
    State(state): State<ServerState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let node = checks::gethostname();
    let (role, _) = checks::parse_peers_conf();
    // Try cached boot result first; run live checks as fallback.
    let cached = state.get_conn().ok().and_then(|conn| {
        conn.query_row(
            "SELECT value FROM daemon_config WHERE key = 'boot_readiness'",
            [],
            |r| r.get::<_, String>(0),
        ).ok()
    }).and_then(|v| serde_json::from_str::<serde_json::Value>(&v).ok());
    if let Some(cached_val) = cached {
        return Ok(Json(cached_val));
    }
    let check_results = build_checks(&state.db_path);
    let ok = check_results.iter().all(|c| c.passed);
    Ok(Json(serde_json::json!({ "ok": ok, "node": node, "role": role, "checks": check_results })))
}

#[cfg(test)]
#[path = "api_node_readiness_tests.rs"]
mod tests;
