// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Individual evidence check implementations for the kernel verify gate.

use crate::kernel::monitor::KernelCheckResult;
use crate::kernel::verify::EvidenceCheck;
use std::process::Command;

pub(crate) fn run_cargo_check(worktree: Option<&str>) -> EvidenceCheck {
    let mut cmd = Command::new("cargo");
    cmd.arg("check");
    if let Some(wt) = worktree {
        cmd.current_dir(wt);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => EvidenceCheck::pass("cargo_check", "exit 0"),
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

pub(crate) fn run_cargo_test(worktree: Option<&str>) -> EvidenceCheck {
    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    if let Some(wt) = worktree {
        cmd.current_dir(wt);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => EvidenceCheck::pass("cargo_test", "exit 0"),
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

pub(crate) fn run_git_clean(worktree: Option<&str>) -> EvidenceCheck {
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

pub(crate) fn build_situation_string(checks: &[EvidenceCheck]) -> String {
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
