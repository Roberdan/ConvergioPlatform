// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
//! Branch, lock-file, and periodic scan functions for the zero-zombie reaper.
//! Split from reaper.rs to keep each file under 250 lines.

use std::process::Command;
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

use super::reaper::{ReapAction, ReapKind, ReapReport};

/// Remove branches fully merged into `main`.
pub fn reap_merged_branches(repo_root: &str, dry_run: bool) -> ReapReport {
    let mut report = ReapReport::default();

    let output = match Command::new("git")
        .args(["branch", "--merged", "main"])
        .current_dir(repo_root)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            report.errors.push(format!("git branch --merged failed: {e}"));
            return report;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    for raw in stdout.lines() {
        let branch = raw.trim().trim_start_matches("* ");
        // Never delete main/master/HEAD
        if branch.is_empty() || branch == "main" || branch == "master" || branch == "HEAD" {
            continue;
        }

        let action = ReapAction {
            kind: ReapKind::Branch,
            target: branch.to_string(),
            reason: "merged into main".to_string(),
        };

        if !dry_run {
            let result = Command::new("git")
                .args(["branch", "-d", branch])
                .current_dir(repo_root)
                .output();

            match result {
                Ok(out) if out.status.success() => {
                    info!(target="reaper", branch=%branch, "deleted merged branch");
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    warn!(target="reaper", branch=%branch, "failed to delete branch: {err}");
                    report.errors.push(format!("delete {branch}: {err}"));
                }
                Err(e) => {
                    report.errors.push(format!("delete {branch}: {e}"));
                }
            }
        } else {
            info!(target="reaper", branch=%branch, "dry-run: would delete merged branch");
        }

        report.actions.push(action);
    }

    report
}

/// Remove expired agent lock files from a directory. A lock file is expired when its
/// mtime is older than `stale_after`.
pub fn reap_lock_files(lock_dir: &str, stale_after: Duration, dry_run: bool) -> ReapReport {
    let mut report = ReapReport::default();

    let entries = match std::fs::read_dir(lock_dir) {
        Ok(e) => e,
        Err(e) => {
            report.errors.push(format!("read_dir {lock_dir}: {e}"));
            return report;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        if !name.ends_with(".lock") {
            continue;
        }

        let is_expired = match path.metadata() {
            Ok(m) => match m.modified() {
                Ok(modified) => {
                    SystemTime::now()
                        .duration_since(modified)
                        .unwrap_or(Duration::ZERO)
                        > stale_after
                }
                Err(_) => false,
            },
            Err(_) => false,
        };

        if !is_expired {
            continue;
        }

        let action = ReapAction {
            kind: ReapKind::LockFile,
            target: path.to_string_lossy().to_string(),
            reason: format!("lock file stale>{stale_after:?}"),
        };

        if !dry_run {
            if let Err(e) = std::fs::remove_file(&path) {
                report.errors.push(format!("remove lock {path:?}: {e}"));
            } else {
                info!(target="reaper", path=?path, "removed stale lock file");
            }
        } else {
            info!(target="reaper", path=?path, "dry-run: would remove lock file");
        }

        report.actions.push(action);
    }

    report
}

/// Spawn a background tokio task that runs all reapers every `interval`.
pub fn start_reaper_task(
    repo_root: String,
    lock_dir: String,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    use super::reaper::reap_worktrees;

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        ticker.tick().await; // first tick fires immediately; skip it
        loop {
            ticker.tick().await;
            let worktree_age = Duration::from_secs(24 * 3600);
            let lock_age = Duration::from_secs(3600);
            let wt = reap_worktrees(&repo_root, worktree_age, false);
            let br = reap_merged_branches(&repo_root, false);
            let lk = reap_lock_files(&lock_dir, lock_age, false);
            info!(
                target = "reaper",
                worktrees_reaped = wt.actions.len(),
                branches_reaped = br.actions.len(),
                locks_reaped = lk.actions.len(),
                "reap cycle complete"
            );
        }
    })
}
