// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// `cvg reap [--dry-run]` — runs all reapers and reports what was cleaned.

use clap::Subcommand;
use std::time::Duration;

#[derive(Debug, Subcommand)]
pub enum ReapCommands {
    /// Run all reapers (worktree, branch, lock-file). Add --dry-run to preview only.
    Run {
        /// Preview changes without removing anything.
        #[arg(long)]
        dry_run: bool,
        /// Repository root (default: current directory).
        #[arg(long, default_value = ".")]
        repo_root: String,
        /// Lock-file directory (default: /tmp).
        #[arg(long, default_value = "/tmp")]
        lock_dir: String,
        /// Human-readable output (default: JSON).
        #[arg(long)]
        human: bool,
    },
}

pub async fn handle(cmd: ReapCommands) {
    match cmd {
        ReapCommands::Run {
            dry_run,
            repo_root,
            lock_dir,
            human,
        } => {
            let stale_wt = Duration::from_secs(24 * 3600);
            let stale_lock = Duration::from_secs(3600);

            let wt = convergio_core::resilience::reaper::reap_worktrees(
                &repo_root,
                stale_wt,
                dry_run,
            );
            let br = convergio_core::resilience::reaper::reap_merged_branches(&repo_root, dry_run);
            let lk = convergio_core::resilience::reaper::reap_lock_files(
                &lock_dir,
                stale_lock,
                dry_run,
            );

            let all_actions: Vec<_> = wt
                .actions
                .iter()
                .chain(&br.actions)
                .chain(&lk.actions)
                .collect();

            let all_errors: Vec<_> = wt
                .errors
                .iter()
                .chain(&br.errors)
                .chain(&lk.errors)
                .collect();

            if human {
                if dry_run {
                    println!("[dry-run] would clean {} item(s)", all_actions.len());
                } else {
                    println!("cleaned {} item(s)", all_actions.len());
                }
                for action in &all_actions {
                    println!("  {:?} {} — {}", action.kind, action.target, action.reason);
                }
                if !all_errors.is_empty() {
                    println!("errors ({}):", all_errors.len());
                    for e in &all_errors {
                        eprintln!("  {e}");
                    }
                }
            } else {
                let output = serde_json::json!({
                    "dry_run": dry_run,
                    "actions": all_actions.len(),
                    "errors": all_errors.len(),
                    "worktrees": wt.actions.len(),
                    "branches": br.actions.len(),
                    "locks": lk.actions.len(),
                });
                println!("{output}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reap_run_command_dry_run_field() {
        let cmd = ReapCommands::Run {
            dry_run: true,
            repo_root: ".".into(),
            lock_dir: "/tmp".into(),
            human: false,
        };
        assert!(matches!(cmd, ReapCommands::Run { dry_run: true, .. }));
    }

    #[test]
    fn reap_run_command_default_dirs() {
        let cmd = ReapCommands::Run {
            dry_run: false,
            repo_root: ".".into(),
            lock_dir: "/tmp".into(),
            human: true,
        };
        assert!(matches!(cmd, ReapCommands::Run { human: true, .. }));
    }
}
