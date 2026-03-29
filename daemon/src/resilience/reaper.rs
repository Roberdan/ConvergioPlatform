// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
//! Zero-zombie enforcement: scans and removes stale worktrees, branches, processes,
//! and DB connections (Article XI — Zero zombies).
//!
//! Run automatically every 30 min via `start_reaper_task`. CLI: `cvg reap [--dry-run]`.

use std::process::Command;
use std::time::{Duration, SystemTime};
use tracing::{info, warn};

/// A single reap action that was taken (or would be taken in dry-run mode).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReapAction {
    pub kind: ReapKind,
    pub target: String,
    pub reason: String,
}

/// Category of resource being reaped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReapKind {
    Worktree,
    Branch,
    LockFile,
    DbConnection,
}

/// Result from a full reap cycle.
#[derive(Debug, Default)]
pub struct ReapReport {
    pub actions: Vec<ReapAction>,
    pub errors: Vec<String>,
}

impl ReapReport {
    pub fn is_clean(&self) -> bool {
        self.actions.is_empty() && self.errors.is_empty()
    }
}

/// Remove git worktrees whose branch is not `main` and whose lock file has not been
/// modified in `stale_after`. Skips worktrees that are currently locked by git.
///
/// Returns actions taken (removed or would-remove in dry-run).
pub fn reap_worktrees(repo_root: &str, stale_after: Duration, dry_run: bool) -> ReapReport {
    let mut report = ReapReport::default();

    let output = match Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_root)
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            report.errors.push(format!("git worktree list failed: {e}"));
            return report;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse porcelain format: blocks separated by blank lines
    for block in stdout.split("\n\n") {
        let mut wt_path: Option<String> = None;
        let mut branch: Option<String> = None;

        for line in block.lines() {
            if let Some(p) = line.strip_prefix("worktree ") {
                wt_path = Some(p.to_string());
            } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
                branch = Some(b.to_string());
            }
        }

        let (Some(path), Some(br)) = (wt_path, branch) else {
            continue;
        };

        // Never touch the main/master worktree
        if br == "main" || br == "master" {
            continue;
        }

        // Use the worktree metadata file to gauge last activity.
        // (The .git/gitdir file exists inside each linked worktree; we check
        // the worktree root directory mtime as a proxy for recent activity.)
        let is_stale = match std::fs::metadata(&path) {
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

        if !is_stale {
            continue;
        }

        let action = ReapAction {
            kind: ReapKind::Worktree,
            target: path.clone(),
            reason: format!("branch={br} stale>{stale_after:?}"),
        };

        if dry_run {
            info!(target="reaper", path=%path, branch=%br, "dry-run: would remove worktree");
        } else {
            let result = Command::new("git")
                .args(["worktree", "remove", "--force", &path])
                .current_dir(repo_root)
                .output();

            match result {
                Ok(out) if out.status.success() => {
                    info!(target="reaper", path=%path, "removed stale worktree");
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    warn!(target="reaper", path=%path, err=%err, "failed to remove worktree");
                    report.errors.push(format!("worktree remove {path}: {err}"));
                }
                Err(e) => {
                    report.errors.push(format!("worktree remove {path}: {e}"));
                }
            }
        }

        report.actions.push(action);
    }

    report
}

// Branch cleanup, lock files, and periodic scan task → reaper_scan.rs
pub use super::reaper_scan::{reap_lock_files, reap_merged_branches, start_reaper_task};

#[cfg(test)]
mod tests;
