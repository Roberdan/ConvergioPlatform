// Feature workspace creation — migrates worktree-create.sh logic into daemon.
// Why: ad-hoc branch workspaces need branch resolution (local→remote→new)
//      that doesn't fit the plan-bound create_workspace flow (Plan 698).

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::workspace::core::{run_git, symlink_env_files, Result, WorkspaceError, WorkspaceInfo};

/// Generate a feature workspace ID with branch name embedded.
/// Format: ws-feat-{branch_name}-{4hex}
pub(crate) fn generate_feature_workspace_id(branch_name: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("ws-feat-{branch_name}-{:04x}", (ts ^ (ts >> 16)) & 0xffff)
}

/// Run git command and return stdout (used for branch existence checks).
pub(crate) fn run_git_output(repo_root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| WorkspaceError::Git(e.to_string()))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(WorkspaceError::Git(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ))
    }
}

/// Create a feature workspace: git worktree for ad-hoc branch work (not plan-bound).
/// Branch resolution order: local → remote → new branch from base_ref/HEAD.
pub(crate) fn create_feature_workspace(
    db_pool: &Pool<SqliteConnectionManager>,
    repo_root: &Path,
    branch_name: &str,
    base_ref: Option<&str>,
) -> Result<WorkspaceInfo> {
    let workspace_id = generate_feature_workspace_id(branch_name);
    let parent = repo_root.parent().unwrap_or(repo_root).to_path_buf();
    let worktree_path = parent.join(format!("ws-{workspace_id}"));
    let path_str = worktree_path.to_string_lossy().to_string();

    // Check if branch already exists locally
    let local_out = run_git_output(repo_root, &["branch", "--list", branch_name])?;
    let branch_exists_locally = !local_out.trim().is_empty();

    if branch_exists_locally {
        // Use existing local branch as-is
        run_git(repo_root, &["worktree", "add", &path_str, branch_name])?;
    } else {
        // Check if branch exists on remote — treat ls-remote failure (no remote) as "not found"
        let remote_out =
            run_git_output(repo_root, &["ls-remote", "--heads", "origin", branch_name])
                .unwrap_or_default();
        let branch_exists_remotely = !remote_out.trim().is_empty();

        if branch_exists_remotely {
            // Track remote branch
            run_git(
                repo_root,
                &[
                    "worktree",
                    "add",
                    "--track",
                    "-b",
                    branch_name,
                    &path_str,
                    &format!("origin/{branch_name}"),
                ],
            )?;
        } else {
            // Create new branch from base_ref or HEAD
            let start_point = base_ref.unwrap_or("HEAD");
            run_git(
                repo_root,
                &["worktree", "add", "-b", branch_name, &path_str, start_point],
            )?;
        }
    }

    symlink_env_files(repo_root, &worktree_path);

    let conn = db_pool.get()?;
    // plan_id=NULL: feature workspaces are not bound to a plan
    conn.execute(
        "INSERT INTO workspaces (plan_id, wave_db_id, workspace_id, path, branch, status)
         VALUES (NULL, NULL, ?1, ?2, ?3, 'active')",
        params![workspace_id, path_str, branch_name],
    )?;
    let created_at: String = conn.query_row(
        "SELECT created_at FROM workspaces WHERE workspace_id = ?1",
        params![workspace_id],
        |row| row.get(0),
    )?;
    Ok(WorkspaceInfo {
        workspace_id,
        path: path_str,
        branch: Some(branch_name.to_string()),
        plan_id: None,
        wave_db_id: None,
        status: "active".to_string(),
        created_at,
    })
}
