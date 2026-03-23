// Workspace lifecycle management — create/delete/list/get git worktrees with DB tracking.
// Why: isolate agent workspaces from main repo; track via workspaces table (Plan 698).

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("pool error: {0}")]
    Pool(#[from] r2d2::Error),
    #[error("git command failed: {0}")]
    Git(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("workspace not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, WorkspaceError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceInfo {
    pub workspace_id: String,
    pub path: String,
    pub branch: Option<String>,
    pub plan_id: Option<i64>,
    pub wave_db_id: Option<i64>,
    pub status: String,
    pub created_at: String,
}

pub struct WorkspaceManager {
    db_pool: Pool<SqliteConnectionManager>,
    repo_root: PathBuf,
}

impl WorkspaceManager {
    pub fn new(db_pool: Pool<SqliteConnectionManager>, repo_root: PathBuf) -> Self {
        Self { db_pool, repo_root }
    }

    /// Create a new workspace: git worktree + DB record.
    pub fn create_workspace(
        &self,
        plan_id: Option<i64>,
        wave_db_id: Option<i64>,
    ) -> Result<WorkspaceInfo> {
        let workspace_id = generate_workspace_id();
        let branch = format!("workspace/{workspace_id}");
        let parent = self
            .repo_root
            .parent()
            .unwrap_or(&self.repo_root)
            .to_path_buf();
        let worktree_path = parent.join(format!("ws-{workspace_id}"));

        run_git(
            &self.repo_root,
            &[
                "worktree",
                "add",
                "-b",
                &branch,
                worktree_path.to_str().unwrap_or(""),
            ],
        )?;
        symlink_env_files(&self.repo_root, &worktree_path);

        let conn = self.db_pool.get()?;
        let path_str = worktree_path.to_string_lossy().to_string();
        conn.execute(
            "INSERT INTO workspaces (plan_id, wave_db_id, workspace_id, path, branch, status)
             VALUES (?1, ?2, ?3, ?4, ?5, 'active')",
            params![plan_id, wave_db_id, workspace_id, path_str, branch],
        )?;
        let created_at: String = conn.query_row(
            "SELECT created_at FROM workspaces WHERE workspace_id = ?1",
            params![workspace_id],
            |row| row.get(0),
        )?;
        Ok(WorkspaceInfo {
            workspace_id,
            path: path_str,
            branch: Some(branch),
            plan_id,
            wave_db_id,
            status: "active".to_string(),
            created_at,
        })
    }

    /// Delete workspace: remove git worktree, delete branch, mark DB record deleted.
    pub fn delete_workspace(&self, workspace_id: &str) -> Result<()> {
        let conn = self.db_pool.get()?;
        let (path, branch): (String, Option<String>) = conn
            .query_row(
                "SELECT path, branch FROM workspaces WHERE workspace_id=?1 AND status='active'",
                params![workspace_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| WorkspaceError::NotFound(workspace_id.to_string()))?;

        run_git(&self.repo_root, &["worktree", "remove", &path, "--force"]).ok();
        if let Some(ref b) = branch {
            run_git(&self.repo_root, &["branch", "-D", b]).ok();
        }
        conn.execute(
            "UPDATE workspaces SET status='deleted', deleted_at=datetime('now') WHERE workspace_id=?1",
            params![workspace_id],
        )?;
        Ok(())
    }

    /// List active workspaces, optionally filtered by plan_id.
    pub fn list_workspaces(&self, plan_id: Option<i64>) -> Result<Vec<WorkspaceInfo>> {
        let conn = self.db_pool.get()?;
        let sql = if plan_id.is_some() {
            "SELECT workspace_id, path, branch, plan_id, wave_db_id, status, created_at
             FROM workspaces WHERE status='active' AND plan_id=?1"
        } else {
            "SELECT workspace_id, path, branch, plan_id, wave_db_id, status, created_at
             FROM workspaces WHERE status='active' ORDER BY created_at DESC"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = if let Some(pid) = plan_id {
            stmt.query_map(params![pid], row_to_workspace)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        } else {
            stmt.query_map([], row_to_workspace)?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        Ok(rows)
    }

    /// Create a feature workspace: git worktree for ad-hoc branch work (not plan-bound).
    /// Branch resolution order: local → remote → new branch from base_ref/HEAD.
    /// Delegates to feature_workspace module to keep core.rs under 250 lines.
    pub fn create_feature_workspace(
        &self,
        branch_name: &str,
        base_ref: Option<&str>,
    ) -> Result<WorkspaceInfo> {
        crate::workspace::feature_workspace::create_feature_workspace(
            &self.db_pool,
            &self.repo_root,
            branch_name,
            base_ref,
        )
    }

    /// Get a single workspace by ID.
    pub fn get_workspace(&self, workspace_id: &str) -> Result<Option<WorkspaceInfo>> {
        let conn = self.db_pool.get()?;
        let result = conn.query_row(
            "SELECT workspace_id, path, branch, plan_id, wave_db_id, status, created_at
             FROM workspaces WHERE workspace_id=?1",
            params![workspace_id],
            row_to_workspace,
        );
        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(WorkspaceError::Db(e)),
        }
    }
}

pub(crate) fn row_to_workspace(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceInfo> {
    Ok(WorkspaceInfo {
        workspace_id: row.get(0)?,
        path: row.get(1)?,
        branch: row.get(2)?,
        plan_id: row.get(3)?,
        wave_db_id: row.get(4)?,
        status: row.get(5)?,
        created_at: row.get(6)?,
    })
}

pub(crate) fn generate_workspace_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("ws-{ts}-{:04x}", (ts ^ (ts >> 16)) & 0xffff)
}

pub(crate) fn run_git(repo_root: &Path, args: &[&str]) -> Result<()> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| WorkspaceError::Git(e.to_string()))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(WorkspaceError::Git(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ))
    }
}

/// Symlink .env* files from repo root into worktree so env vars are available.
pub(crate) fn symlink_env_files(repo_root: &Path, worktree_path: &Path) {
    let Ok(entries) = std::fs::read_dir(repo_root) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with(".env") {
            let dst = worktree_path.join(&*name_str);
            if !dst.exists() {
                let _ = symlink(entry.path(), &dst);
            }
        }
    }
}

#[cfg(test)]
#[path = "core_tests.rs"]
mod tests;
