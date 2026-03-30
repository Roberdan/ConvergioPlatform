// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Individual evidence check implementations for the kernel verify gate.

use crate::kernel::monitor::KernelCheckResult;
use crate::kernel::verify::EvidenceCheck;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command;

// Re-export hardening primitives so callers use a single import path.
pub use crate::kernel::verify_hardening::{
    evidence_cache_key, git_head_sha, reap_build_processes, EvidenceCache, EVIDENCE_CACHE,
    EVIDENCE_MUTEX,
};

pub(crate) fn run_cargo_check(worktree: Option<&str>) -> EvidenceCheck {
    run_command_with_timeout("cargo_check", "cargo", &["check"], worktree, 60)
}

/// Shared helper: run a command with timeout and process-group cleanup.
fn run_command_with_timeout(
    check_name: &str,
    program: &str,
    args: &[&str],
    worktree: Option<&str>,
    timeout_secs: u64,
) -> EvidenceCheck {
    use std::time::Duration;
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(wt) = worktree {
        cmd.current_dir(wt);
    }
    cmd.stderr(std::process::Stdio::piped());
    #[cfg(unix)]
    unsafe { cmd.pre_exec(|| { libc::setpgid(0, 0); Ok(()) }); }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return EvidenceCheck::fail(check_name, format!("spawn error: {e}")),
    };
    let timeout = Duration::from_secs(timeout_secs);
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                return EvidenceCheck::pass(check_name, "exit 0");
            }
            Ok(Some(status)) => {
                let stderr = child.stderr.take().map(|mut s| {
                    let mut buf = String::new();
                    use std::io::Read;
                    if let Err(e) = s.read_to_string(&mut buf) {
                        tracing::warn!("verify_checks: read stderr: {e}");
                    }
                    buf
                }).unwrap_or_default();
                return EvidenceCheck::fail(
                    check_name,
                    format!("exit {}: {}",
                        status.code().unwrap_or(-1),
                        stderr.chars().take(200).collect::<String>()),
                );
            }
            Ok(None) if start.elapsed() > timeout => {
                #[cfg(unix)]
                unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL); }
                if let Err(e) = child.kill() {
                    tracing::warn!("verify_checks: kill timed-out child: {e}");
                }
                if let Err(e) = child.wait() {
                    tracing::warn!("verify_checks: wait after kill: {e}");
                }
                return EvidenceCheck::fail(
                    check_name,
                    format!("timeout after {timeout_secs}s — killed"),
                );
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(500)),
            Err(e) => return EvidenceCheck::fail(check_name, format!("wait error: {e}")),
        }
    }
}

pub(crate) fn run_cargo_test(worktree: Option<&str>) -> EvidenceCheck {
    // --lib only: unit tests are fast (~20s). Full test (bins+integration) takes 45s+
    // and spawns 400+ threads, risking resource exhaustion.
    run_command_with_timeout("cargo_test", "cargo", &["test", "--lib"], worktree, 180)
}

pub(crate) fn run_npm_check(worktree: Option<&str>) -> EvidenceCheck {
    let mut cmd = Command::new("npx");
    cmd.args(["tsc", "--noEmit"]);
    if let Some(wt) = worktree {
        cmd.current_dir(wt);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => EvidenceCheck::pass("npm_check", "tsc --noEmit exit 0"),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            let msg = if stderr.is_empty() { stdout } else { stderr };
            EvidenceCheck::fail(
                "npm_check",
                format!("exit {}: {}", out.status.code().unwrap_or(-1), msg.chars().take(200).collect::<String>()),
            )
        }
        Err(e) => EvidenceCheck::fail("npm_check", format!("spawn error: {e}")),
    }
}

pub(crate) fn run_npm_test(worktree: Option<&str>) -> EvidenceCheck {
    let mut cmd = Command::new("npx");
    cmd.args(["vitest", "run"]);
    if let Some(wt) = worktree {
        cmd.current_dir(wt);
    }
    match cmd.output() {
        Ok(out) if out.status.success() => EvidenceCheck::pass("npm_test", "vitest run exit 0"),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let stdout = String::from_utf8_lossy(&out.stdout);
            let msg = if stderr.is_empty() { stdout } else { stderr };
            EvidenceCheck::fail(
                "npm_test",
                format!("exit {}: {}", out.status.code().unwrap_or(-1), msg.chars().take(200).collect::<String>()),
            )
        }
        Err(e) => EvidenceCheck::fail("npm_test", format!("spawn error: {e}")),
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
