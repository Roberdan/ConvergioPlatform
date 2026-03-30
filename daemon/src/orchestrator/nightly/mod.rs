// Nightly autonomy module: cleanup + evaluation, run at 02:00 CET.
pub mod cleanup;
pub mod evaluation;
pub mod report;

use chrono::Local;
use std::path::{Path, PathBuf};
use tracing::{error, info};

pub use report::NightlyReport;

/// Run the full nightly job: cleanup → evaluate → report.
/// `db_path`: path to dashboard.db  `platform_dir`: repo root.
pub async fn run_nightly(db_path: &Path, platform_dir: &Path) -> NightlyReport {
    info!("nightly: starting autonomy job");
    let date = Local::now().format("%Y-%m-%d").to_string();
    let host = hostname();

    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("nightly: cannot open DB: {e}");
            return failed_report(date, host);
        }
    };

    // --- Cleanup phase ---
    let worktrees_pruned = cleanup::prune_stale_worktrees(&conn, platform_dir);
    let zombies_killed = cleanup::kill_zombie_processes(&conn);
    let stale_agents_removed = cleanup::cleanup_stale_agents(&conn);
    let evidence_files_cleared = cleanup::clear_evidence_cache(&conn);
    let (git_gc_ok, branches_pruned) = cleanup::run_git_gc(platform_dir);

    let cleanup_result = cleanup::CleanupResult {
        worktrees_pruned,
        zombies_killed,
        stale_agents_removed,
        evidence_files_cleared,
        git_gc_ok,
        branches_pruned,
    };

    // --- Evaluation phase ---
    let (commits_today, fix_chains, failed_tests_in_log) =
        evaluation::analyse_git_log(platform_dir);
    let agents_over_limit = evaluation::audit_agent_tokens(platform_dir);
    let stale_memory_files = evaluation::audit_stale_memory(platform_dir);
    let test_health = evaluation::run_test_health(platform_dir);
    let outdated_deps = evaluation::check_outdated_deps(platform_dir);

    let eval_result = evaluation::EvaluationResult {
        commits_today,
        fix_chains,
        failed_tests_in_log,
        agents_over_limit,
        stale_memory_files,
        test_health,
        outdated_deps,
    };

    let status = if eval_result.test_health.passed {
        "ok"
    } else {
        "test_failure"
    }
    .to_string();

    let nightly_report = NightlyReport {
        date: date.clone(),
        host: host.clone(),
        cleanup: report::CleanupSummary::from(cleanup_result),
        evaluation: eval_result,
        status,
    };

    report::write_report_file(&nightly_report);
    report::store_in_db(&conn, &nightly_report);

    info!("nightly: job complete for {date}");
    nightly_report
}

fn hostname() -> String {
    std::process::Command::new("hostname")
        .arg("-s")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn failed_report(date: String, host: String) -> NightlyReport {
    NightlyReport {
        date,
        host,
        cleanup: report::CleanupSummary {
            worktrees_pruned: 0,
            zombies_killed: 0,
            stale_agents_removed: 0,
            evidence_files_cleared: 0,
            git_gc_ok: false,
            branches_pruned: 0,
        },
        evaluation: evaluation::EvaluationResult::default(),
        status: "db_error".to_string(),
    }
}

/// Derive the platform directory from the DB path (assumes data/ is inside the repo).
pub fn platform_dir_from_db(db_path: &Path) -> PathBuf {
    db_path
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn nightly_platform_dir_from_db() {
        let db = PathBuf::from("/home/user/repo/data/dashboard.db");
        let dir = platform_dir_from_db(&db);
        assert_eq!(dir, PathBuf::from("/home/user/repo"));
    }

    #[test]
    fn nightly_hostname_not_empty() {
        let h = hostname();
        assert!(!h.is_empty());
    }

    #[tokio::test]
    async fn nightly_run_with_missing_db_returns_failed_report() {
        let report = run_nightly(
            &PathBuf::from("/nonexistent/dashboard.db"),
            &PathBuf::from("/nonexistent"),
        )
        .await;
        assert_eq!(report.status, "db_error");
    }

    #[test]
    fn nightly_report_json_roundtrip() {
        let r = NightlyReport {
            date: "2025-01-01".to_string(),
            host: "m1-pro".to_string(),
            cleanup: report::CleanupSummary {
                worktrees_pruned: 2,
                zombies_killed: 1,
                stale_agents_removed: 3,
                evidence_files_cleared: 5,
                git_gc_ok: true,
                branches_pruned: 4,
            },
            evaluation: evaluation::EvaluationResult::default(),
            status: "ok".to_string(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: NightlyReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.status, "ok");
        assert_eq!(back.cleanup.worktrees_pruned, 2);
    }
}
