// Tests for workspace::core — DB operations and struct construction.
// Uses in-memory SQLite; git commands are not invoked (no real repo needed).

use super::*;
use crate::workspace::feature_workspace::generate_feature_workspace_id;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tempfile::tempdir;

pub(crate) fn make_workspace_pool() -> Pool<SqliteConnectionManager> {
    let pool = Pool::builder()
        .max_size(1)
        .build(SqliteConnectionManager::memory())
        .unwrap();
    pool.get()
        .unwrap()
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS workspaces (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            plan_id INTEGER, wave_db_id INTEGER,
            workspace_id TEXT UNIQUE NOT NULL, path TEXT NOT NULL,
            branch TEXT, status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL DEFAULT (datetime('now')), deleted_at TEXT
        );",
        )
        .unwrap();
    pool
}

#[test]
fn workspace_id_format() {
    let id = generate_workspace_id();
    assert!(id.starts_with("ws-"), "id should start with ws-: {id}");
    let parts: Vec<&str> = id.splitn(3, '-').collect();
    assert_eq!(
        parts.len(),
        3,
        "id should have 3 dash-separated parts: {id}"
    );
    assert!(!parts[1].is_empty(), "timestamp part should be non-empty");
    assert_eq!(parts[2].len(), 4, "hex suffix should be 4 chars: {id}");
}

#[test]
fn list_workspaces_empty() {
    let mgr = WorkspaceManager::new(make_workspace_pool(), PathBuf::from("/tmp"));
    assert!(
        mgr.list_workspaces(None).unwrap().is_empty(),
        "fresh DB should return empty list"
    );
}

#[test]
fn get_workspace_not_found() {
    let mgr = WorkspaceManager::new(make_workspace_pool(), PathBuf::from("/tmp"));
    assert!(mgr.get_workspace("ws-nonexistent-0000").unwrap().is_none());
}

#[test]
fn insert_and_get_workspace_directly() {
    let pool = make_workspace_pool();
    let mgr = WorkspaceManager::new(pool.clone(), PathBuf::from("/tmp"));
    pool.get()
        .unwrap()
        .execute(
            "INSERT INTO workspaces (plan_id, wave_db_id, workspace_id, path, branch, status)
         VALUES (42, 7, 'ws-111-aaaa', '/tmp/ws-111-aaaa', 'workspace/ws-111-aaaa', 'active')",
            [],
        )
        .unwrap();
    let info = mgr
        .get_workspace("ws-111-aaaa")
        .unwrap()
        .expect("should find workspace");
    assert_eq!(info.plan_id, Some(42));
    assert_eq!(info.wave_db_id, Some(7));
    assert_eq!(info.branch, Some("workspace/ws-111-aaaa".to_string()));
    assert_eq!(info.status, "active");
}

#[test]
fn list_workspaces_filtered_by_plan_id() {
    let pool = make_workspace_pool();
    let mgr = WorkspaceManager::new(pool.clone(), PathBuf::from("/tmp"));
    pool.get().unwrap().execute_batch(
        "INSERT INTO workspaces (plan_id, workspace_id, path, status) VALUES (10, 'ws-10-aaaa', '/tmp/ws-10', 'active');
         INSERT INTO workspaces (plan_id, workspace_id, path, status) VALUES (20, 'ws-20-bbbb', '/tmp/ws-20', 'active');",
    ).unwrap();
    assert_eq!(mgr.list_workspaces(Some(10)).unwrap().len(), 1);
    assert_eq!(
        mgr.list_workspaces(Some(10)).unwrap()[0].workspace_id,
        "ws-10-aaaa"
    );
    assert_eq!(mgr.list_workspaces(None).unwrap().len(), 2);
}

#[test]
fn delete_workspace_marks_deleted() {
    let pool = make_workspace_pool();
    pool.get()
        .unwrap()
        .execute_batch(
            "INSERT INTO workspaces (plan_id, workspace_id, path, branch, status)
         VALUES (1, 'ws-del-0001', '/nonexistent/path', 'workspace/ws-del-0001', 'active');",
        )
        .unwrap();
    let tmp = tempdir().unwrap();
    let mgr = WorkspaceManager::new(pool.clone(), tmp.path().to_path_buf());
    // git calls fail on non-repo but use .ok() — DB update must still happen
    let _ = mgr.delete_workspace("ws-del-0001");
    let status: String = pool
        .get()
        .unwrap()
        .query_row(
            "SELECT status FROM workspaces WHERE workspace_id='ws-del-0001'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "deleted");
}

#[test]
fn symlink_env_files_skips_nonexistent_dir() {
    // Should not panic on invalid paths
    symlink_env_files(
        Path::new("/nonexistent/path"),
        Path::new("/also/nonexistent"),
    );
}

// --- feature workspace tests ---

#[test]
fn feature_workspace_id_format() {
    // ws-feat-{branch_name}-{4hex}
    let id = generate_feature_workspace_id("my-branch");
    assert!(
        id.starts_with("ws-feat-my-branch-"),
        "expected ws-feat-<branch>-<hex>: {id}"
    );
    let hex_suffix = id.rsplit('-').next().unwrap();
    assert_eq!(hex_suffix.len(), 4, "hex suffix should be 4 chars: {id}");
}

#[test]
fn feature_workspace_inserts_null_plan_id() {
    // create_feature_workspace stores plan_id=NULL (feature branch, not plan-bound)
    let tmp_repo = tempdir().unwrap();
    // Init bare-ish git repo so worktree commands work
    StdCommand::new("git")
        .args(["init"])
        .current_dir(tmp_repo.path())
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(tmp_repo.path())
        .output()
        .unwrap();
    StdCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(tmp_repo.path())
        .output()
        .unwrap();
    // Need at least one commit for branch to work
    StdCommand::new("git")
        .args(["commit", "--allow-empty", "-m", "init"])
        .current_dir(tmp_repo.path())
        .output()
        .unwrap();

    let pool = make_workspace_pool();
    let mgr = WorkspaceManager::new(pool.clone(), tmp_repo.path().to_path_buf());
    let info = mgr
        .create_feature_workspace("feat-test-branch", None)
        .unwrap();
    assert_eq!(
        info.plan_id, None,
        "feature workspace must have null plan_id"
    );
    assert_eq!(
        info.wave_db_id, None,
        "feature workspace must have null wave_db_id"
    );
    assert!(
        info.workspace_id.starts_with("ws-feat-feat-test-branch-"),
        "id format: {}",
        info.workspace_id
    );
    assert_eq!(info.status, "active");
    // cleanup: delete the worktree we just made
    let _ = mgr.delete_workspace(&info.workspace_id);
}

#[test]
fn feature_workspace_cleanup_removes_entry() {
    // After delete_workspace, the record should be status=deleted
    let pool = make_workspace_pool();
    pool.get().unwrap().execute_batch(
        "INSERT INTO workspaces (plan_id, workspace_id, path, branch, status)
         VALUES (NULL, 'ws-feat-cleanup-0001', '/nonexistent/feat', 'feat-cleanup-0001', 'active');",
    ).unwrap();
    let tmp = tempdir().unwrap();
    let mgr = WorkspaceManager::new(pool.clone(), tmp.path().to_path_buf());
    let _ = mgr.delete_workspace("ws-feat-cleanup-0001"); // git calls fail but DB must update
    let status: String = pool
        .get()
        .unwrap()
        .query_row(
            "SELECT status FROM workspaces WHERE workspace_id='ws-feat-cleanup-0001'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "deleted");
}

#[test]
fn row_to_workspace_info_fields() {
    let pool = make_workspace_pool();
    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO workspaces (plan_id, wave_db_id, workspace_id, path, branch, status)
         VALUES (5, 3, 'ws-999-ffff', '/ws/path', 'workspace/ws-999-ffff', 'active')",
        [],
    )
    .unwrap();
    let info: WorkspaceInfo = conn
        .query_row(
            "SELECT workspace_id, path, branch, plan_id, wave_db_id, status, created_at
         FROM workspaces WHERE workspace_id='ws-999-ffff'",
            [],
            row_to_workspace,
        )
        .unwrap();
    assert_eq!(info.workspace_id, "ws-999-ffff");
    assert_eq!(info.plan_id, Some(5));
    assert_eq!(info.wave_db_id, Some(3));
    assert!(!info.created_at.is_empty());
}
