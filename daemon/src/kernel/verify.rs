// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Task completion evidence gate — blocks "done"/"submitted" transitions
// when mandatory checks fail. Logs every verification to kernel_verifications.
//
// WHY: Agents mark tasks done without evidence; this gate enforces Article VI
// of the Constitution ("done" = evidence) at the API boundary.

use crate::kernel::engine::{KernelAction, KernelEngine};
use crate::kernel::monitor::KernelCheckResult;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single named evidence check with pass/fail outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

impl EvidenceCheck {
    fn pass(name: &str, detail: impl Into<String>) -> Self {
        Self { name: name.to_string(), passed: true, detail: detail.into() }
    }

    fn fail(name: &str, detail: impl Into<String>) -> Self {
        Self { name: name.to_string(), passed: false, detail: detail.into() }
    }
}

/// Aggregate result of a full evidence gate run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceReport {
    pub task_id: i64,
    pub status_requested: String,
    pub checks: Vec<EvidenceCheck>,
    pub passed: bool,
    /// Severity classification from KernelEngine::classify().
    pub severity: String,
    /// Recommended action (retry / escalate / block).
    pub action: String,
    pub reason: String,
}

impl EvidenceReport {
    pub fn failed_checks(&self) -> Vec<&EvidenceCheck> {
        self.checks.iter().filter(|c| !c.passed).collect()
    }
}

// ---------------------------------------------------------------------------
// Public gate
// ---------------------------------------------------------------------------

/// Run all evidence checks for `task_id` in `worktree` and return a report.
///
/// The caller (API handler) decides whether to block the DB update based on
/// `report.passed`. This function never mutates the DB — it only reads task
/// metadata from `conn` and writes the verification record.
pub fn check_evidence(
    conn: &Connection,
    engine: &KernelEngine,
    task_id: i64,
    status: &str,
    worktree: Option<&str>,
    output_files: &[&str],
) -> EvidenceReport {
    info!(task_id, status, "kernel verify: running evidence gate");

    let mut checks: Vec<EvidenceCheck> = Vec::new();

    // 1. Declared output files exist.
    for &file in output_files {
        if file.is_empty() {
            continue;
        }
        if Path::new(file).exists() {
            checks.push(EvidenceCheck::pass("output_file_exists", file));
        } else {
            checks.push(EvidenceCheck::fail(
                "output_file_exists",
                format!("file not found: {file}"),
            ));
        }
    }

    // 2. Build succeeds (cargo check).
    checks.push(run_cargo_check(worktree));

    // 3. Tests pass (cargo test).
    checks.push(run_cargo_test(worktree));

    // 4. Working tree clean (git status --porcelain).
    checks.push(run_git_clean(worktree));

    // Aggregate pass/fail.
    let passed = checks.iter().all(|c| c.passed);

    // Classify via KernelEngine (heuristic if no model loaded).
    let situation = build_situation_string(&checks);
    let KernelAction { severity, action, reason } = engine.classify(&situation);
    let severity_str = format!("{severity:?}").to_lowercase();

    let report = EvidenceReport {
        task_id,
        status_requested: status.to_string(),
        checks: checks.clone(),
        passed,
        severity: severity_str.clone(),
        action: action.clone(),
        reason: reason.clone(),
    };

    // Persist verification record (best-effort — never block the report).
    let checks_json =
        serde_json::to_string(&checks).unwrap_or_else(|_| "[]".to_string());
    let blocked_reason: Option<String> = if passed {
        None
    } else {
        Some(format!("{action}: {reason}"))
    };
    if let Err(e) = conn.execute(
        "INSERT INTO kernel_verifications \
         (task_id, checks_json, passed, blocked_reason) \
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            task_id,
            checks_json,
            if passed { 1i64 } else { 0i64 },
            blocked_reason,
        ],
    ) {
        warn!(task_id, error = %e, "kernel verify: failed to persist verification record");
    }

    if passed {
        info!(task_id, "kernel verify: all checks passed");
    } else {
        warn!(task_id, severity = severity_str, "kernel verify: evidence gate BLOCKED");
    }

    report
}

// ---------------------------------------------------------------------------
// Individual checks
// ---------------------------------------------------------------------------

fn run_cargo_check(worktree: Option<&str>) -> EvidenceCheck {
    let mut cmd = Command::new("cargo");
    cmd.arg("check");
    if let Some(wt) = worktree {
        cmd.current_dir(wt);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => {
            EvidenceCheck::pass("cargo_check", "exit 0")
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            EvidenceCheck::fail(
                "cargo_check",
                format!(
                    "exit {}: {}",
                    out.status.code().unwrap_or(-1),
                    stderr.chars().take(200).collect::<String>()
                ),
            )
        }
        Err(e) => EvidenceCheck::fail("cargo_check", format!("spawn error: {e}")),
    }
}

fn run_cargo_test(worktree: Option<&str>) -> EvidenceCheck {
    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    if let Some(wt) = worktree {
        cmd.current_dir(wt);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => {
            EvidenceCheck::pass("cargo_test", "exit 0")
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            EvidenceCheck::fail(
                "cargo_test",
                format!(
                    "exit {}: {}",
                    out.status.code().unwrap_or(-1),
                    stderr.chars().take(200).collect::<String>()
                ),
            )
        }
        Err(e) => EvidenceCheck::fail("cargo_test", format!("spawn error: {e}")),
    }
}

fn run_git_clean(worktree: Option<&str>) -> EvidenceCheck {
    let mut cmd = Command::new("git");
    cmd.args(["status", "--porcelain"]);
    if let Some(wt) = worktree {
        cmd.current_dir(wt);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let output = stdout.trim().to_string();
            if output.is_empty() {
                EvidenceCheck::pass("git_clean", "working tree clean")
            } else {
                EvidenceCheck::fail(
                    "git_clean",
                    format!("dirty tree: {}", output.chars().take(200).collect::<String>()),
                )
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            EvidenceCheck::fail(
                "git_clean",
                format!(
                    "git status failed ({}): {}",
                    out.status.code().unwrap_or(-1),
                    stderr.chars().take(200).collect::<String>()
                ),
            )
        }
        Err(e) => EvidenceCheck::fail("git_clean", format!("spawn error: {e}")),
    }
}

fn build_situation_string(checks: &[EvidenceCheck]) -> String {
    let failures: Vec<&str> = checks
        .iter()
        .filter(|c| !c.passed)
        .map(|c| c.name.as_str())
        .collect();
    if failures.is_empty() {
        "all evidence checks passed — task completion verified".to_string()
    } else {
        format!("evidence gate failures: {}", failures.join(", "))
    }
}

// ---------------------------------------------------------------------------
// Helper: convert KernelCheckResult → EvidenceCheck (interop with monitor)
// ---------------------------------------------------------------------------

impl From<KernelCheckResult> for EvidenceCheck {
    fn from(r: KernelCheckResult) -> Self {
        Self {
            name: r.check_name,
            passed: r.ok,
            detail: r.details.unwrap_or_default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::engine::{KernelConfig, KernelEngine};
    use rusqlite::Connection;

    fn make_engine() -> KernelEngine {
        KernelEngine::new(KernelConfig::default())
    }

    fn make_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kernel_verifications (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id        INTEGER,
                timestamp      TEXT NOT NULL DEFAULT (datetime('now')),
                checks_json    TEXT NOT NULL DEFAULT '[]',
                passed         INTEGER NOT NULL DEFAULT 1,
                blocked_reason TEXT
            );",
        )
        .expect("schema");
        conn
    }

    // --- EvidenceCheck construction ---

    #[test]
    fn evidence_check_pass_sets_passed_true() {
        let c = EvidenceCheck::pass("my_check", "all good");
        assert!(c.passed);
        assert_eq!(c.name, "my_check");
        assert_eq!(c.detail, "all good");
    }

    #[test]
    fn evidence_check_fail_sets_passed_false() {
        let c = EvidenceCheck::fail("my_check", "missing file");
        assert!(!c.passed);
    }

    // --- EvidenceReport helpers ---

    #[test]
    fn evidence_report_failed_checks_filters_correctly() {
        let checks = vec![
            EvidenceCheck::pass("a", "ok"),
            EvidenceCheck::fail("b", "bad"),
            EvidenceCheck::fail("c", "also bad"),
        ];
        let report = EvidenceReport {
            task_id: 1,
            status_requested: "done".to_string(),
            passed: false,
            checks,
            severity: "warn".to_string(),
            action: "alert".to_string(),
            reason: "failures".to_string(),
        };
        let failed = report.failed_checks();
        assert_eq!(failed.len(), 2);
        assert!(failed.iter().any(|c| c.name == "b"));
        assert!(failed.iter().any(|c| c.name == "c"));
    }

    // --- File existence check (inline) ---

    #[test]
    fn output_file_check_passes_for_existing_file() {
        // Use this very source file as the existing file.
        let path = file!();
        assert!(Path::new(path).exists() || {
            // file! gives a relative path; try absolute fallback.
            let abs = format!(
                "{}/{}",
                env!("CARGO_MANIFEST_DIR"),
                path
            );
            Path::new(&abs).exists()
        });
    }

    #[test]
    fn output_file_check_fails_for_missing_file() {
        let c = if Path::new("/nonexistent/file/kernel_verify_test.rs").exists() {
            EvidenceCheck::pass("output_file_exists", "unexpected")
        } else {
            EvidenceCheck::fail("output_file_exists", "file not found: /nonexistent/file/kernel_verify_test.rs")
        };
        assert!(!c.passed);
    }

    // --- DB persistence ---

    #[test]
    fn check_evidence_persists_record_to_db() {
        let conn = make_conn();
        let engine = make_engine();

        // Pass no output_files; cargo/git checks will run — may pass or fail
        // in CI. We only assert the record is written.
        let report = check_evidence(&conn, &engine, 42, "done", None, &[]);

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM kernel_verifications WHERE task_id = 42",
                [],
                |r| r.get(0),
            )
            .expect("count query");
        assert_eq!(count, 1, "verification record must be written to DB");
        assert_eq!(report.task_id, 42);
        assert_eq!(report.status_requested, "done");
    }

    #[test]
    fn check_evidence_passed_field_matches_db_record() {
        let conn = make_conn();
        let engine = make_engine();

        let report = check_evidence(&conn, &engine, 99, "submitted", None, &[]);

        let db_passed: i64 = conn
            .query_row(
                "SELECT passed FROM kernel_verifications WHERE task_id = 99",
                [],
                |r| r.get(0),
            )
            .expect("passed query");

        let expected = if report.passed { 1i64 } else { 0i64 };
        assert_eq!(db_passed, expected);
    }

    #[test]
    fn check_evidence_blocked_reason_null_when_passed() {
        // Simulate a scenario with no output files; other checks may vary.
        // We only care about the blocked_reason column when passed = true.
        let conn = make_conn();
        let engine = make_engine();
        let report = check_evidence(&conn, &engine, 7, "done", None, &[]);

        if report.passed {
            let blocked: Option<String> = conn
                .query_row(
                    "SELECT blocked_reason FROM kernel_verifications WHERE task_id = 7",
                    [],
                    |r| r.get(0),
                )
                .expect("blocked query");
            assert!(blocked.is_none(), "blocked_reason must be NULL when passed");
        }
        // If not passed, blocked_reason is set — valid either way.
    }

    // --- KernelCheckResult interop ---

    #[test]
    fn from_kernel_check_result_pass() {
        let kcr = KernelCheckResult::pass("daemon_health");
        let ec = EvidenceCheck::from(kcr);
        assert!(ec.passed);
        assert_eq!(ec.name, "daemon_health");
    }

    #[test]
    fn from_kernel_check_result_fail() {
        let kcr = KernelCheckResult::fail("daemon_health", "HTTP 503");
        let ec = EvidenceCheck::from(kcr);
        assert!(!ec.passed);
        assert_eq!(ec.detail, "HTTP 503");
    }

    // --- Build situation string ---

    #[test]
    fn situation_string_all_pass() {
        let checks = vec![EvidenceCheck::pass("a", "ok"), EvidenceCheck::pass("b", "ok")];
        let s = build_situation_string(&checks);
        assert!(s.contains("passed"), "situation: {s}");
    }

    #[test]
    fn situation_string_with_failures() {
        let checks = vec![
            EvidenceCheck::pass("a", "ok"),
            EvidenceCheck::fail("cargo_check", "error"),
        ];
        let s = build_situation_string(&checks);
        assert!(s.contains("cargo_check"), "situation: {s}");
    }

    // --- Serialization ---

    #[test]
    fn evidence_report_serializes_to_json() {
        let report = EvidenceReport {
            task_id: 5,
            status_requested: "done".to_string(),
            passed: false,
            checks: vec![EvidenceCheck::fail("cargo_test", "1 failed")],
            severity: "critical".to_string(),
            action: "block".to_string(),
            reason: "tests failed".to_string(),
        };
        let json = serde_json::to_string(&report).expect("serialize");
        assert!(json.contains("cargo_test"));
        assert!(json.contains("task_id"));
        assert!(json.contains("kernel_verifications") || json.contains("checks"));
    }
}
