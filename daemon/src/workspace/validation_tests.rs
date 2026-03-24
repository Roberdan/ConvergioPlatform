// Tests for workspace::validation — task/wave validation lifecycle.
// Why: enforce TDD before implementation (Plan 698 T3-04).

use super::*;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

/// In-memory pool with minimal tasks + waves + plans schema for validation tests.
fn make_pool() -> ConnPool {
    let pool = Pool::builder()
        .max_size(4)
        .build(SqliteConnectionManager::memory())
        .unwrap();
    pool.get()
        .unwrap()
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS plans (
                id INTEGER PRIMARY KEY NOT NULL,
                tasks_done INTEGER DEFAULT 0,
                tasks_total INTEGER DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'doing'
            );
            CREATE TABLE IF NOT EXISTS waves (
                id INTEGER PRIMARY KEY NOT NULL,
                plan_id INTEGER DEFAULT NULL,
                wave_id TEXT NOT NULL DEFAULT '',
                position INTEGER DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'pending',
                tasks_done INTEGER DEFAULT 0,
                tasks_total INTEGER DEFAULT 0,
                completed_at DATETIME DEFAULT NULL
            );
            CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY NOT NULL,
                plan_id INTEGER DEFAULT NULL,
                wave_id_fk INTEGER DEFAULT NULL,
                task_id TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL DEFAULT 'pending',
                validated_at DATETIME DEFAULT NULL,
                validated_by TEXT DEFAULT NULL,
                completed_at DATETIME DEFAULT NULL
            );",
        )
        .unwrap();
    pool
}

fn insert_plan(pool: &ConnPool, plan_id: i64, tasks_done: i64, tasks_total: i64) {
    pool.get()
        .unwrap()
        .execute(
            "INSERT INTO plans (id, tasks_done, tasks_total) VALUES (?1, ?2, ?3)",
            rusqlite::params![plan_id, tasks_done, tasks_total],
        )
        .unwrap();
}

fn insert_wave(pool: &ConnPool, wave_db_id: i64, plan_id: i64, position: i64, status: &str) -> i64 {
    pool.get()
        .unwrap()
        .execute(
            "INSERT INTO waves (id, plan_id, wave_id, position, status) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![wave_db_id, plan_id, format!("W{position}"), position, status],
        )
        .unwrap();
    wave_db_id
}

fn insert_task(pool: &ConnPool, plan_id: i64, wave_id_fk: i64, status: &str) -> i64 {
    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO tasks (plan_id, wave_id_fk, task_id, status) VALUES (?1, ?2, 'T1', ?3)",
        rusqlite::params![plan_id, wave_id_fk, status],
    )
    .unwrap();
    conn.last_insert_rowid()
}

// --- validate_task ---

#[test]
fn validate_task_submitted_transitions_to_done() {
    let pool = make_pool();
    insert_plan(&pool, 1, 0, 1);
    let wave_db_id = insert_wave(&pool, 10, 1, 1, "in_progress");
    let task_db_id = insert_task(&pool, 1, wave_db_id, "submitted");

    let result = validate_task(task_db_id, "thor", &pool);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    let r = result.unwrap();
    assert_eq!(r.old_status, "submitted");
    assert_eq!(r.new_status, "done");
    assert_eq!(r.task_db_id, task_db_id);

    let conn = pool.get().unwrap();
    let (status, validated_by, validated_at): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT status, validated_by, validated_at FROM tasks WHERE id = ?1",
            rusqlite::params![task_db_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(status, "done");
    assert_eq!(validated_by.as_deref(), Some("thor"));
    assert!(validated_at.is_some(), "validated_at must be set");
}

#[test]
fn validate_task_rejects_non_thor_validator() {
    let pool = make_pool();
    insert_plan(&pool, 2, 0, 1);
    let wave_db_id = insert_wave(&pool, 20, 2, 1, "in_progress");
    let task_db_id = insert_task(&pool, 2, wave_db_id, "submitted");

    let result = validate_task(task_db_id, "agent-smith", &pool);
    assert!(result.is_err(), "expected Err for non-Thor validator");
    assert!(
        result.unwrap_err().to_string().contains("authorized"),
        "error should mention authorization"
    );
}

#[test]
fn validate_task_rejects_non_submitted_status() {
    let pool = make_pool();
    insert_plan(&pool, 3, 0, 1);
    let wave_db_id = insert_wave(&pool, 30, 3, 1, "in_progress");
    let task_db_id = insert_task(&pool, 3, wave_db_id, "in_progress");

    let result = validate_task(task_db_id, "thor", &pool);
    assert!(result.is_err(), "expected Err for in_progress status");
    assert!(
        result.unwrap_err().to_string().contains("in_progress"),
        "error should mention the invalid status"
    );
}

#[test]
fn validate_task_done_without_validated_at_gets_stamp() {
    let pool = make_pool();
    insert_plan(&pool, 4, 1, 1);
    let wave_db_id = insert_wave(&pool, 40, 4, 1, "done");
    let task_db_id = insert_task(&pool, 4, wave_db_id, "done");

    let result = validate_task(task_db_id, "thor-per-wave", &pool);
    assert!(
        result.is_ok(),
        "expected Ok for done task missing validated_at"
    );
    let r = result.unwrap();
    assert_eq!(r.old_status, "done");
    assert_eq!(r.new_status, "done");
}

#[path = "validation_tests_wave.rs"]
mod wave_tests;
