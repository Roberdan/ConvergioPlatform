// reaper_main_guard.rs — detects dirty main repo (copilot crash artifacts).
// WHY: 2026-03-31 incident — copilot left 467 dirty files on main branch.
// Runs every 5 min as part of the reaper cycle. Sends notification if dirty.

const DAEMON_BASE: &str = "http://localhost:8420";
const DIRTY_THRESHOLD: usize = 5;

/// Check if main repo working directory has uncommitted changes.
/// Sends warning notification via daemon API (Telegram/ntfy).
pub async fn check_main_repo_dirty() {
    let repo = std::env::var("CONVERGIO_REPO_ROOT").unwrap_or_default();
    if repo.is_empty() { return; }

    // Only check the main checkout, not worktrees
    let common = std::process::Command::new("git")
        .args(["-C", &repo, "rev-parse", "--git-common-dir"])
        .output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    let gitdir = std::process::Command::new("git")
        .args(["-C", &repo, "rev-parse", "--git-dir"])
        .output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    if common != gitdir { return; } // worktree — skip

    let output = match std::process::Command::new("git")
        .args(["-C", &repo, "status", "--porcelain"])
        .output() {
        Ok(o) => o,
        Err(_) => return,
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let dirty: usize = stdout.lines().count();
    if dirty <= DIRTY_THRESHOLD { return; }

    let branch = std::process::Command::new("git")
        .args(["-C", &repo, "rev-parse", "--abbrev-ref", "HEAD"])
        .output().ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let msg = format!(
        "REAPER WARNING: main repo has {dirty} dirty files on branch '{branch}'. \
         Possible copilot crash without cleanup."
    );
    tracing::warn!("{msg}");
    let client = reqwest::Client::new();
    let _ = client.post(format!("{DAEMON_BASE}/api/notify"))
        .json(&serde_json::json!({
            "title": "MainDirtyGuard",
            "message": msg,
            "severity": "warning"
        }))
        .send().await;
}
