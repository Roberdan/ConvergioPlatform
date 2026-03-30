// Nightly cleanup tasks: worktree prune, zombie kill, stale agents, evidence cache, git gc.
use rusqlite::Connection;
use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

pub struct CleanupResult {
    pub worktrees_pruned: usize,
    pub zombies_killed: usize,
    pub stale_agents_removed: usize,
    pub evidence_files_cleared: usize,
    pub git_gc_ok: bool,
    pub branches_pruned: usize,
}

/// Prune git worktrees older than 7 days with no associated active plan.
pub fn prune_stale_worktrees(db: &Connection, platform_dir: &Path) -> usize {
    let active_worktrees: Vec<String> = db
        .prepare(
            "SELECT DISTINCT worktree_path FROM plans \
             WHERE status NOT IN ('done','cancelled') AND worktree_path IS NOT NULL",
        )
        .and_then(|mut s| {
            s.query_map([], |r| r.get::<_, String>(0))
                .map(|rows| rows.flatten().collect())
        })
        .unwrap_or_default();

    let out = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(platform_dir)
        .output();

    let Ok(out) = out else { return 0 };
    let stdout = String::from_utf8_lossy(&out.stdout);

    let mut pruned = 0usize;
    let mut current_path: Option<String> = None;

    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(path.to_string());
        } else if line == "HEAD" || line.starts_with("HEAD ") {
            // main worktree, skip
            current_path = None;
        } else if line.is_empty() {
            if let Some(ref path) = current_path {
                let p = Path::new(path);
                let is_active = active_worktrees.iter().any(|a| a == path);
                let is_old = is_older_than_days(p, 7);
                if !is_active && is_old && p != platform_dir {
                    let removed = Command::new("git")
                        .args(["worktree", "remove", "--force", path])
                        .current_dir(platform_dir)
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);
                    if removed {
                        info!("nightly: pruned worktree {path}");
                        pruned += 1;
                    } else {
                        warn!("nightly: failed to prune worktree {path}");
                    }
                }
            }
            current_path = None;
        }
    }
    pruned
}

fn is_older_than_days(path: &Path, days: u64) -> bool {
    path.metadata()
        .and_then(|m| m.modified())
        .map(|t| {
            t.elapsed()
                .map(|d| d.as_secs() > days * 86_400)
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Kill copilot/claude processes running longer than 4 hours without an active task.
pub fn kill_zombie_processes(db: &Connection) -> usize {
    let active_pids: Vec<i64> = db
        .prepare(
            "SELECT pid FROM agent_activity \
             WHERE status='running' AND pid IS NOT NULL \
             AND started_at >= datetime('now', '-4 hours')",
        )
        .and_then(|mut s| {
            s.query_map([], |r| r.get::<_, i64>(0))
                .map(|rows| rows.flatten().collect())
        })
        .unwrap_or_default();

    let out = Command::new("pgrep")
        .args(["-fl", "copilot\\|claude"])
        .output();

    let Ok(out) = out else { return 0 };
    let stdout = String::from_utf8_lossy(&out.stdout);

    let mut killed = 0usize;
    for line in stdout.lines() {
        let mut parts = line.splitn(2, ' ');
        let pid_str = parts.next().unwrap_or("");
        let Ok(pid) = pid_str.parse::<i64>() else { continue };

        // Skip if it has an active task registered in DB
        if active_pids.contains(&pid) {
            continue;
        }

        // Check runtime via ps
        let age_out = Command::new("ps")
            .args(["-o", "etimes=", "-p", pid_str])
            .output();
        let secs: u64 = age_out
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
            .unwrap_or(0);

        if secs > 4 * 3600 {
            let ok = Command::new("kill")
                .args(["-9", pid_str])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if ok {
                info!("nightly: killed zombie pid={pid}");
                killed += 1;
            }
        }
    }
    killed
}

/// Remove agents from DB not seen in 24h.
pub fn cleanup_stale_agents(db: &Connection) -> usize {
    db.execute(
        "DELETE FROM ipc_agents WHERE last_seen < datetime('now', '-24 hours')",
        [],
    )
    .unwrap_or(0)
}

/// Clear evidence cache rows older than 7 days.
pub fn clear_evidence_cache(db: &Connection) -> usize {
    db.execute(
        "DELETE FROM plan_evidence WHERE created_at < datetime('now', '-7 days')",
        [],
    )
    .unwrap_or(0)
}

/// Run git gc and prune merged remote tracking branches.
pub fn run_git_gc(platform_dir: &Path) -> (bool, usize) {
    let gc_ok = Command::new("git")
        .args(["gc", "--auto", "--quiet"])
        .current_dir(platform_dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    // Prune merged remote branches (safe: only remote-tracking refs)
    let fetch_ok = Command::new("git")
        .args(["fetch", "--prune", "--quiet"])
        .current_dir(platform_dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let pruned = if fetch_ok {
        count_pruned_remote_branches(platform_dir)
    } else {
        0
    };

    (gc_ok, pruned)
}

fn count_pruned_remote_branches(platform_dir: &Path) -> usize {
    let out = Command::new("git")
        .args(["branch", "-r", "--merged", "main"])
        .current_dir(platform_dir)
        .output();
    out.map(|o| {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| !l.contains("HEAD") && !l.trim().is_empty())
            .count()
    })
    .unwrap_or(0)
}
