// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Task completion evidence gate — blocks "done"/"submitted" transitions
// when mandatory checks fail. Logs every verification to kernel_verifications.
//
// WHY: Agents mark tasks done without evidence; this gate enforces Article VI
// of the Constitution ("done" = evidence) at the API boundary.

use crate::kernel::engine::{KernelAction, KernelEngine};
use crate::kernel::verify_checks::{build_situation_string, run_cargo_check, run_cargo_test, run_git_clean};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;
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
    pub(crate) fn pass(name: &str, detail: impl Into<String>) -> Self {
        Self { name: name.to_string(), passed: true, detail: detail.into() }
    }

    pub(crate) fn fail(name: &str, detail: impl Into<String>) -> Self {
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

