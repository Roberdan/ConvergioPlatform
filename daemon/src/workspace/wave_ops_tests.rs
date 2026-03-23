// Tests for workspace::wave_ops — DB operations for wave workspace lifecycle.
// Uses in-memory SQLite + tempdir for disk-path checks; real git for worktree tests.

use super::*;
use crate::workspace::core::WorkspaceManager;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::process::Command;
use tempfile::tempdir;

/// In-memory pool with workspaces + waves tables (minimal schema for tests).
pub(crate) fn make_wave_pool() -> ConnPool {
    let pool = Pool::builder()
        .max_size(4)
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
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                deleted_at TEXT
            );
            CREATE TABLE IF NOT EXISTS waves (
                id INTEGER PRIMARY KEY NOT NULL,
                wave_id TEXT NOT NULL DEFAULT '',
                plan_id INTEGER DEFAULT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                worktree_path TEXT DEFAULT NULL,
                branch_name TEXT DEFAULT NULL
            );",
        )
        .unwrap();
    pool
}

/// Insert a wave row and return its db id.
fn insert_wave(pool: &ConnPool, wave_id: &str, plan_id: i64) -> i64 {
    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO waves (wave_id, plan_id) VALUES (?1, ?2)",
        params![wave_id, plan_id],
    )
    .unwrap();
    conn.last_insert_rowid()
}

/// Insert an active workspace row linked to a wave.
fn insert_workspace(pool: &ConnPool, workspace_id: &str, wave_db_id: i64, path: &str) {
    pool.get()
        .unwrap()
        .execute(
            "INSERT INTO workspaces (workspace_id, wave_db_id, path, branch, status)
             VALUES (?1, ?2, ?3, 'plan/test-W1', 'active')",
            params![workspace_id, wave_db_id, path],
        )
        .unwrap();
}

// --- create_wave_workspace ---

#[test]
fn create_wave_workspace_wave_not_found() {
    let pool = make_wave_pool();
    let tmp = tempdir().unwrap();
    let mgr = WorkspaceManager::new(pool.clone(), tmp.path().to_path_buf());
    let result = create_wave_workspace(&mgr, 698, 9999, &pool);
    assert!(result.is_err(), "should error for unknown wave_db_id");
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[test]
fn create_wave_workspace_stores_branch_in_waves() {
    // Needs a real git repo so worktree add works
    let tmp = tempdir().unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    let pool = make_wave_pool();
    let wave_db_id = insert_wave(&pool, "W1", 698);
    let mgr = WorkspaceManager::new(pool.clone(), tmp.path().to_path_buf());

    let result = create_wave_workspace(&mgr, 698, wave_db_id, &pool);
    assert!(result.is_ok(), "create failed: {:?}", result.err());
    let info = result.unwrap();

    assert_eq!(info.wave_id, "W1");
    assert_eq!(info.plan_id, 698);
    assert_eq!(info.branch, "plan/698-W1");

    // Verify waves row updated with path and branch
    let (wt_path, bn): (Option<String>, Option<String>) = pool
        .get()
        .unwrap()
        .query_row(
            "SELECT worktree_path, branch_name FROM waves WHERE id = ?1",
            params![wave_db_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(wt_path.is_some(), "worktree_path should be set in waves");
    assert_eq!(bn.as_deref(), Some("plan/698-W1"));

    // cleanup to not leave orphan worktree
    let _ = mgr.delete_workspace(&info.workspace_id);
}

// --- cleanup_wave_workspace ---

#[test]
fn cleanup_wave_workspace_no_active_workspace() {
    let pool = make_wave_pool();
    let wave_db_id = insert_wave(&pool, "W1", 1);
    let tmp = tempdir().unwrap();
    let mgr = WorkspaceManager::new(pool.clone(), tmp.path().to_path_buf());
    let result = cleanup_wave_workspace(&mgr, wave_db_id, &pool);
    assert!(result.is_err(), "should err when no active workspace");
}

#[test]
fn cleanup_wave_workspace_clears_wave_fields() {
    let pool = make_wave_pool();
    let wave_db_id = insert_wave(&pool, "W2", 10);

    // Manually set worktree fields in waves row
    pool.get()
        .unwrap()
        .execute(
            "UPDATE waves SET worktree_path='/tmp/fake', branch_name='plan/10-W2' WHERE id=?1",
            params![wave_db_id],
        )
        .unwrap();

    // Insert active workspace row (path does not need to exist; git calls use .ok())
    insert_workspace(&pool, "ws-cleanup-0001", wave_db_id, "/nonexistent/path");

    let tmp = tempdir().unwrap();
    let mgr = WorkspaceManager::new(pool.clone(), tmp.path().to_path_buf());

    // delete_workspace will fail to remove worktree (no git repo) but DB must still update
    let _ = cleanup_wave_workspace(&mgr, wave_db_id, &pool);

    let (wt, bn): (Option<String>, Option<String>) = pool
        .get()
        .unwrap()
        .query_row(
            "SELECT worktree_path, branch_name FROM waves WHERE id = ?1",
            params![wave_db_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(wt.is_none(), "worktree_path should be cleared");
    assert!(bn.is_none(), "branch_name should be cleared");
}

// --- wave_workspace_status ---

#[test]
fn wave_workspace_status_none_when_no_workspace() {
    let pool = make_wave_pool();
    let wave_db_id = insert_wave(&pool, "W3", 20);
    let result = wave_workspace_status(wave_db_id, &pool).unwrap();
    assert!(result.is_none(), "should return None with no workspace");
}

#[test]
fn wave_workspace_status_none_when_path_missing() {
    let pool = make_wave_pool();
    let wave_db_id = insert_wave(&pool, "W4", 30);
    insert_workspace(
        &pool,
        "ws-status-missing",
        wave_db_id,
        "/nonexistent/ws-path",
    );
    let result = wave_workspace_status(wave_db_id, &pool).unwrap();
    assert!(
        result.is_none(),
        "should return None when path does not exist on disk"
    );
}

#[test]
fn wave_workspace_status_some_when_path_exists() {
    let pool = make_wave_pool();
    let tmp = tempdir().unwrap();
    let path_str = tmp.path().to_string_lossy().to_string();
    let wave_db_id = insert_wave(&pool, "W5", 40);
    insert_workspace(&pool, "ws-status-exists", wave_db_id, &path_str);
    let result = wave_workspace_status(wave_db_id, &pool).unwrap();
    assert!(result.is_some(), "should return Some when path exists");
    let info = result.unwrap();
    assert_eq!(info.workspace_id, "ws-status-exists");
    assert_eq!(info.wave_id, "W5");
    assert_eq!(info.plan_id, 40);
    assert_eq!(info.path, path_str);
}
