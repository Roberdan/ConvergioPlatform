// Nightly report: write markdown file + store in nightly_reports DB table.
use super::cleanup::CleanupResult;
use super::evaluation::EvaluationResult;
use chrono::Local;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct NightlyReport {
    pub date: String,
    pub host: String,
    pub cleanup: CleanupSummary,
    pub evaluation: EvaluationResult,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupSummary {
    pub worktrees_pruned: usize,
    pub zombies_killed: usize,
    pub stale_agents_removed: usize,
    pub evidence_files_cleared: usize,
    pub git_gc_ok: bool,
    pub branches_pruned: usize,
}

impl From<CleanupResult> for CleanupSummary {
    fn from(c: CleanupResult) -> Self {
        Self {
            worktrees_pruned: c.worktrees_pruned,
            zombies_killed: c.zombies_killed,
            stale_agents_removed: c.stale_agents_removed,
            evidence_files_cleared: c.evidence_files_cleared,
            git_gc_ok: c.git_gc_ok,
            branches_pruned: c.branches_pruned,
        }
    }
}

pub fn build_markdown(report: &NightlyReport) -> String {
    let mut md = String::new();
    md.push_str(&format!("# Nightly Report — {}\n\n", report.date));
    md.push_str(&format!("**Host:** {}  \n", report.host));
    md.push_str(&format!("**Status:** {}  \n\n", report.status));

    md.push_str("## Cleanup\n\n");
    md.push_str(&format!(
        "- Worktrees pruned: {}\n",
        report.cleanup.worktrees_pruned
    ));
    md.push_str(&format!(
        "- Zombies killed: {}\n",
        report.cleanup.zombies_killed
    ));
    md.push_str(&format!(
        "- Stale agents removed: {}\n",
        report.cleanup.stale_agents_removed
    ));
    md.push_str(&format!(
        "- Evidence cache cleared: {}\n",
        report.cleanup.evidence_files_cleared
    ));
    md.push_str(&format!(
        "- Git GC: {}\n",
        if report.cleanup.git_gc_ok { "ok" } else { "failed" }
    ));
    md.push_str(&format!(
        "- Remote branches pruned: {}\n\n",
        report.cleanup.branches_pruned
    ));

    md.push_str("## Evaluation\n\n");
    let ev = &report.evaluation;
    md.push_str(&format!("- Commits today: {}\n", ev.commits_today));
    md.push_str(&format!(
        "- Failed log entries: {}\n",
        ev.failed_tests_in_log
    ));
    md.push_str(&format!("- Test suite: {}\n", if ev.test_health.passed { "✅ passed" } else { "❌ failed" }));

    if !ev.fix_chains.is_empty() {
        md.push_str("\n### Fix chains\n\n");
        for f in &ev.fix_chains {
            md.push_str(&format!("- {f}\n"));
        }
    }

    if !ev.agents_over_limit.is_empty() {
        md.push_str("\n### Agents over 200 lines\n\n");
        for a in &ev.agents_over_limit {
            md.push_str(&format!("- {} ({} lines)\n", a.path, a.lines));
        }
    }

    if !ev.stale_memory_files.is_empty() {
        md.push_str("\n### Stale memory files (>30 days)\n\n");
        for f in &ev.stale_memory_files {
            md.push_str(&format!("- {f}\n"));
        }
    }

    if !ev.outdated_deps.is_empty() {
        md.push_str("\n### Outdated dependencies\n\n");
        for d in &ev.outdated_deps {
            md.push_str(&format!("- {} {} → {}\n", d.name, d.current, d.latest));
        }
    }

    md
}

/// Write the markdown report to a file. Returns the path written.
pub fn write_report_file(report: &NightlyReport) -> Option<PathBuf> {
    let filename = format!("/tmp/nightly-report-{}.md", report.date);
    let path = PathBuf::from(&filename);
    let content = build_markdown(report);
    match std::fs::write(&path, content) {
        Ok(()) => {
            info!("nightly: report written to {filename}");
            Some(path)
        }
        Err(e) => {
            warn!("nightly: failed to write report file: {e}");
            None
        }
    }
}

/// Ensure nightly_reports table exists and insert the report.
pub fn store_in_db(db: &Connection, report: &NightlyReport) {
    let _ = db.execute_batch(
        "CREATE TABLE IF NOT EXISTS nightly_reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            host TEXT,
            status TEXT,
            worktrees_pruned INTEGER DEFAULT 0,
            zombies_killed INTEGER DEFAULT 0,
            stale_agents_removed INTEGER DEFAULT 0,
            evidence_cleared INTEGER DEFAULT 0,
            git_gc_ok INTEGER DEFAULT 0,
            branches_pruned INTEGER DEFAULT 0,
            commits_today INTEGER DEFAULT 0,
            failed_log_entries INTEGER DEFAULT 0,
            test_health_passed INTEGER DEFAULT 0,
            agents_over_limit INTEGER DEFAULT 0,
            stale_memory_files INTEGER DEFAULT 0,
            outdated_deps INTEGER DEFAULT 0,
            report_json TEXT,
            created_at TEXT DEFAULT (datetime('now'))
        );",
    );

    let report_json = serde_json::to_string(report).unwrap_or_default();
    let now = Local::now().format("%Y-%m-%d").to_string();
    let ev = &report.evaluation;
    let cl = &report.cleanup;

    let result = db.execute(
        "INSERT INTO nightly_reports (
            date, host, status,
            worktrees_pruned, zombies_killed, stale_agents_removed, evidence_cleared,
            git_gc_ok, branches_pruned,
            commits_today, failed_log_entries, test_health_passed,
            agents_over_limit, stale_memory_files, outdated_deps,
            report_json
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        rusqlite::params![
            now,
            report.host,
            report.status,
            cl.worktrees_pruned as i64,
            cl.zombies_killed as i64,
            cl.stale_agents_removed as i64,
            cl.evidence_files_cleared as i64,
            cl.git_gc_ok as i64,
            cl.branches_pruned as i64,
            ev.commits_today as i64,
            ev.failed_tests_in_log as i64,
            ev.test_health.passed as i64,
            ev.agents_over_limit.len() as i64,
            ev.stale_memory_files.len() as i64,
            ev.outdated_deps.len() as i64,
            report_json,
        ],
    );

    match result {
        Ok(_) => info!("nightly: report stored in DB for {}", report.date),
        Err(e) => warn!("nightly: failed to store report: {e}"),
    }
}
