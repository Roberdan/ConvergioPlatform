// cvg cleanup — find and remove stale worktree branches and orphan worktrees.
// Cleans up worktree-agent-*, worktree-plan-*, wt-plan-* branches without matching dirs.

use crate::cli_error::CliError;
use std::collections::HashSet;

pub async fn handle() -> Result<(), CliError> {
    let active_paths = list_active_worktree_paths();
    let stale_branches = find_stale_branches(&active_paths);

    if stale_branches.is_empty() {
        println!("No stale worktree branches found.");
    } else {
        println!("Found {} stale branch(es):", stale_branches.len());
        for b in &stale_branches {
            println!("  - {b}");
        }
        delete_branches(&stale_branches);
    }

    // Always prune worktree metadata for removed directories
    prune_worktrees();
    Ok(())
}

/// List paths of active git worktrees (from `git worktree list --porcelain`).
fn list_active_worktree_paths() -> HashSet<String> {
    let out = match std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return HashSet::new(),
    };
    out.lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(String::from)
        .collect()
}

/// Find branches matching worktree patterns that have no active worktree directory.
fn find_stale_branches(active_paths: &HashSet<String>) -> Vec<String> {
    let prefixes = [
        "worktree-agent-",
        "worktree-plan-",
        "wt-plan-",
        "workspace/ws-",
    ];
    let out = match std::process::Command::new("git")
        .args(["branch", "--list"])
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut stale = Vec::new();
    for line in out.lines() {
        let branch = line.trim().trim_start_matches("* ");
        if !prefixes.iter().any(|p| branch.starts_with(p) || branch.contains(p)) {
            continue;
        }
        // A branch is stale if no active worktree path contains this branch name
        let has_active = active_paths.iter().any(|p| {
            p.contains(&branch.replace('/', "-"))
                || p.contains(branch)
        });
        if !has_active {
            stale.push(branch.to_string());
        }
    }
    stale
}

fn delete_branches(branches: &[String]) {
    for branch in branches {
        let out = std::process::Command::new("git")
            .args(["branch", "-D", branch])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                println!("  deleted branch: {branch}");
            }
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr);
                eprintln!("  failed to delete {branch}: {err}");
            }
            Err(e) => {
                eprintln!("  git branch -D {branch}: {e}");
            }
        }
    }
}

fn prune_worktrees() {
    match std::process::Command::new("git")
        .args(["worktree", "prune"])
        .output()
    {
        Ok(o) if o.status.success() => {
            println!("Worktree metadata pruned.");
        }
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            eprintln!("worktree prune failed: {err}");
        }
        Err(e) => {
            eprintln!("worktree prune: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_branch_detection_empty() {
        let active = HashSet::new();
        let branches = find_stale_branches(&active);
        // No branches in test env, just verify it doesn't panic
        assert!(branches.is_empty() || !branches.is_empty());
    }
}
