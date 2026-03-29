use super::handoff_locking::{acquire_lock, merge_plan_status, release_lock};
use rusqlite::Connection;
use tempfile::TempDir;

fn setup_dbs(tmp: &TempDir) -> (std::path::PathBuf, std::path::PathBuf) {
    let local_path = tmp.path().join("local.db");
    let remote_path = tmp.path().join("remote.db");

    for path in [&local_path, &remote_path] {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS plans (
                id INTEGER PRIMARY KEY,
                tasks_done INTEGER DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS waves (
                id INTEGER PRIMARY KEY,
                plan_id INTEGER,
                tasks_done INTEGER DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY,
                plan_id INTEGER,
                wave_id_fk INTEGER,
                status TEXT DEFAULT 'pending',
                completed_at TEXT,
                validated_at TEXT,
                validated_by TEXT
            );",
        )
        .unwrap();
    }

    (local_path, remote_path)
}

#[test]
fn acquire_and_release_lock_succeeds() {
    let tmp = TempDir::new().unwrap();
    let lock_dir = tmp.path().join("locks");
    acquire_lock(&lock_dir, 42, "node-a", 300).unwrap();
    assert!(lock_dir.join("delegate-42.lock").exists());
    release_lock(&lock_dir, 42).unwrap();
    assert!(!lock_dir.join("delegate-42.lock").exists());
}

#[test]
fn acquire_lock_fails_when_held() {
    let tmp = TempDir::new().unwrap();
    let lock_dir = tmp.path().join("locks");
    acquire_lock(&lock_dir, 99, "node-a", 300).unwrap();
    let result = acquire_lock(&lock_dir, 99, "node-b", 300);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("locked by node-a"));
}

#[test]
fn acquire_lock_succeeds_after_ttl_expired() {
    let tmp = TempDir::new().unwrap();
    let lock_dir = tmp.path().join("locks");
    // Write a lock with ts=0 (way in the past)
    std::fs::create_dir_all(&lock_dir).unwrap();
    let lock_file = lock_dir.join("delegate-77.lock");
    std::fs::write(
        &lock_file,
        r#"{"peer":"old-node","ts":0,"pid":1234}"#,
    )
    .unwrap();
    // TTL=1 means any lock older than 1s is stale
    acquire_lock(&lock_dir, 77, "new-node", 1).unwrap();
}

#[test]
fn release_lock_noop_when_not_exists() {
    let tmp = TempDir::new().unwrap();
    // Should not error
    release_lock(tmp.path(), 999).unwrap();
}

#[test]
fn merge_plan_status_promotes_tasks() {
    let tmp = TempDir::new().unwrap();
    let (local_path, remote_path) = setup_dbs(&tmp);

    // Seed local with pending task
    let local = Connection::open(&local_path).unwrap();
    local.execute("INSERT INTO plans (id, tasks_done) VALUES (1, 0)", []).unwrap();
    local.execute("INSERT INTO waves (id, plan_id, tasks_done) VALUES (1, 1, 0)", []).unwrap();
    local
        .execute(
            "INSERT INTO tasks (id, plan_id, wave_id_fk, status) VALUES (100, 1, 1, 'pending')",
            [],
        )
        .unwrap();
    drop(local);

    // Seed remote with done task
    let remote = Connection::open(&remote_path).unwrap();
    remote.execute("INSERT INTO plans (id, tasks_done) VALUES (1, 1)", []).unwrap();
    remote.execute("INSERT INTO waves (id, plan_id, tasks_done) VALUES (1, 1, 1)", []).unwrap();
    remote
        .execute(
            "INSERT INTO tasks (id, plan_id, wave_id_fk, status, completed_at) \
             VALUES (100, 1, 1, 'done', '2026-03-28T10:00:00Z')",
            [],
        )
        .unwrap();
    drop(remote);

    let updates = merge_plan_status(1, &local_path, &remote_path).unwrap();
    assert_eq!(updates, 1);

    // Verify local was updated
    let local = Connection::open(&local_path).unwrap();
    let status: String = local
        .query_row("SELECT status FROM tasks WHERE id=100", [], |r| r.get(0))
        .unwrap();
    assert_eq!(status, "done");
}

#[test]
fn merge_plan_status_does_not_demote() {
    let tmp = TempDir::new().unwrap();
    let (local_path, remote_path) = setup_dbs(&tmp);

    // Local has "done", remote has "pending" — should NOT demote
    let local = Connection::open(&local_path).unwrap();
    local.execute("INSERT INTO plans (id) VALUES (2)", []).unwrap();
    local
        .execute(
            "INSERT INTO tasks (id, plan_id, status) VALUES (200, 2, 'done')",
            [],
        )
        .unwrap();
    drop(local);

    let remote = Connection::open(&remote_path).unwrap();
    remote.execute("INSERT INTO plans (id) VALUES (2)", []).unwrap();
    remote
        .execute(
            "INSERT INTO tasks (id, plan_id, status) VALUES (200, 2, 'pending')",
            [],
        )
        .unwrap();
    drop(remote);

    let updates = merge_plan_status(2, &local_path, &remote_path).unwrap();
    assert_eq!(updates, 0);

    let local = Connection::open(&local_path).unwrap();
    let status: String = local
        .query_row("SELECT status FROM tasks WHERE id=200", [], |r| r.get(0))
        .unwrap();
    assert_eq!(status, "done");
}

#[test]
fn merge_plan_status_no_tasks_returns_zero() {
    let tmp = TempDir::new().unwrap();
    let (local_path, remote_path) = setup_dbs(&tmp);

    let local = Connection::open(&local_path).unwrap();
    local.execute("INSERT INTO plans (id) VALUES (3)", []).unwrap();
    drop(local);
    let remote = Connection::open(&remote_path).unwrap();
    remote.execute("INSERT INTO plans (id) VALUES (3)", []).unwrap();
    drop(remote);

    let updates = merge_plan_status(3, &local_path, &remote_path).unwrap();
    assert_eq!(updates, 0);
}
