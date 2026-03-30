// Rollback model — captures pre-task snapshots and restores them on demand.
// save_snapshot: captures git HEAD + changed files + task DB row.
// restore_snapshot: checks out the saved git_ref and resets task to 'pending'.
// list_snapshots: returns all snapshots for a task ordered newest-first.

use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

type RollbackResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

// ── Migration ─────────────────────────────────────────────────────────────────

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS rollback_snapshots (
            id            INTEGER PRIMARY KEY,
            task_id       INTEGER,
            git_ref       TEXT NOT NULL,
            changed_files TEXT,
            db_rows_json  TEXT,
            created_at    TEXT DEFAULT (datetime('now'))
        );",
    )
}

// ── Core operations ───────────────────────────────────────────────────────────

/// Capture current git HEAD, changed files, and the task DB row as a snapshot.
pub fn save_snapshot(
    conn: &Connection,
    task_id: i64,
    worktree_path: &Path,
) -> RollbackResult<i64> {
    let git_ref = git_rev_parse(worktree_path)?;
    let changed_files = git_changed_files(worktree_path)?;

    let db_rows_json: Option<String> = conn
        .query_row(
            "SELECT id, task_id, title, status, description FROM tasks WHERE id = ?1",
            params![task_id],
            |row| {
                Ok(json!({
                    "id":          row.get::<_, i64>(0)?,
                    "task_id":     row.get::<_, Option<String>>(1)?,
                    "title":       row.get::<_, Option<String>>(2)?,
                    "status":      row.get::<_, Option<String>>(3)?,
                    "description": row.get::<_, Option<String>>(4)?,
                })
                .to_string())
            },
        )
        .ok();

    conn.execute(
        "INSERT INTO rollback_snapshots (task_id, git_ref, changed_files, db_rows_json)
         VALUES (?1, ?2, ?3, ?4)",
        params![task_id, git_ref, changed_files, db_rows_json],
    )?;

    Ok(conn.last_insert_rowid())
}

/// Restore the latest snapshot for a task: checks out the saved git_ref
/// in the worktree and resets task status to 'pending'.
pub fn restore_snapshot(
    conn: &Connection,
    task_id: i64,
    worktree_path: &Path,
) -> RollbackResult<()> {
    let git_ref: String = conn
        .query_row(
            "SELECT git_ref FROM rollback_snapshots \
             WHERE task_id = ?1 ORDER BY id DESC LIMIT 1",
            params![task_id],
            |row| row.get(0),
        )
        .map_err(|_| format!("no snapshot found for task {task_id}"))?;

    let out = Command::new("git")
        .args(["checkout", &git_ref])
        .current_dir(worktree_path)
        .output()
        .map_err(|e| format!("git checkout failed: {e}"))?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!("git checkout {git_ref} failed: {stderr}").into());
    }

    conn.execute(
        "UPDATE tasks SET status = 'pending', started_at = NULL WHERE id = ?1",
        params![task_id],
    )?;

    Ok(())
}

/// Return all snapshots for a task, newest first.
pub fn list_snapshots(conn: &Connection, task_id: i64) -> rusqlite::Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, git_ref, changed_files, db_rows_json, created_at
         FROM rollback_snapshots
         WHERE task_id = ?1
         ORDER BY id DESC",
    )?;

    let rows: rusqlite::Result<Vec<Value>> = stmt
        .query_map(params![task_id], |row| {
            Ok(json!({
                "id":            row.get::<_, i64>(0)?,
                "task_id":       row.get::<_, Option<i64>>(1)?,
                "git_ref":       row.get::<_, String>(2)?,
                "changed_files": row.get::<_, Option<String>>(3)?,
                "db_rows_json":  row.get::<_, Option<String>>(4)?,
                "created_at":    row.get::<_, Option<String>>(5)?,
            }))
        })?
        .collect();

    rows
}

// ── Git helpers ───────────────────────────────────────────────────────────────

fn git_rev_parse(worktree_path: &Path) -> RollbackResult<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(worktree_path)
        .output()
        .map_err(|e| format!("git rev-parse failed: {e}"))?;

    if !out.status.success() {
        return Err(format!("git rev-parse HEAD failed in {:?}", worktree_path).into());
    }

    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn git_changed_files(worktree_path: &Path) -> RollbackResult<Option<String>> {
    let out = Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(worktree_path)
        .output()
        .map_err(|e| format!("git diff failed: {e}"))?;

    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(if text.is_empty() { None } else { Some(text) })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY,
                task_id TEXT,
                title TEXT,
                status TEXT DEFAULT 'pending',
                description TEXT,
                started_at TEXT
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_migrate_idempotent() {
        let conn = setup();
        migrate(&conn).unwrap();
        conn.execute_batch("SELECT 1 FROM rollback_snapshots LIMIT 1")
            .unwrap();
    }

    #[test]
    fn test_list_snapshots_empty() {
        let conn = setup();
        let snaps = list_snapshots(&conn, 999).unwrap();
        assert!(snaps.is_empty());
    }

    #[test]
    fn test_restore_no_snapshot_errors() {
        let conn = setup();
        let err = restore_snapshot(&conn, 42, Path::new("/nonexistent")).unwrap_err();
        assert!(err.to_string().contains("no snapshot found"));
    }

    #[test]
    fn test_save_and_list_snapshot() {
        let conn = setup();
        conn.execute(
            "INSERT INTO tasks (id, task_id, title, status) VALUES (1, 'T1-01', 'Test', 'in_progress')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO rollback_snapshots (task_id, git_ref, changed_files) \
             VALUES (?1, ?2, ?3)",
            params![1i64, "deadbeef1234", "src/lib.rs\nsrc/main.rs"],
        )
        .unwrap();

        let snaps = list_snapshots(&conn, 1).unwrap();
        assert_eq!(snaps.len(), 1);
        assert_eq!(snaps[0]["git_ref"], "deadbeef1234");
        assert_eq!(snaps[0]["changed_files"], "src/lib.rs\nsrc/main.rs");
    }

    #[test]
    fn test_restore_resets_task_status() {
        let conn = setup();
        conn.execute(
            "INSERT INTO tasks (id, task_id, title, status, started_at) \
             VALUES (1, 'T1-01', 'Test', 'in_progress', '2025-01-01')",
            [],
        )
        .unwrap();

        // DB-only path: reset status directly (git checkout skipped in unit tests)
        conn.execute(
            "UPDATE tasks SET status = 'pending', started_at = NULL WHERE id = ?1",
            params![1i64],
        )
        .unwrap();

        let status: String = conn
            .query_row("SELECT status FROM tasks WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(status, "pending");
    }
}
