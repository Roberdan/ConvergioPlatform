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

    // Verify DB row was updated
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
        result.unwrap_err().contains("authorized"),
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
        result.unwrap_err().contains("in_progress"),
        "error should mention the invalid status"
    );
}

#[test]
fn validate_task_done_without_validated_at_gets_stamp() {
    let pool = make_pool();
    insert_plan(&pool, 4, 1, 1);
    let wave_db_id = insert_wave(&pool, 40, 4, 1, "done");
    let task_db_id = insert_task(&pool, 4, wave_db_id, "done");

    // done + validated_at IS NULL is allowed (backfill scenario)
    let result = validate_task(task_db_id, "thor-per-wave", &pool);
    assert!(
        result.is_ok(),
        "expected Ok for done task missing validated_at"
    );
    let r = result.unwrap();
    assert_eq!(r.old_status, "done");
    assert_eq!(r.new_status, "done");
}

// --- validate_wave ---

#[test]
fn validate_wave_batch_promotes_submitted_to_done() {
    let pool = make_pool();
    insert_plan(&pool, 5, 0, 3);
    let wave_db_id = insert_wave(&pool, 50, 5, 1, "in_progress");
    insert_task(&pool, 5, wave_db_id, "submitted");
    insert_task(&pool, 5, wave_db_id, "submitted");
    insert_task(&pool, 5, wave_db_id, "done"); // already done — stamp validated_at
    pool.get()
        .unwrap()
        .execute(
            "UPDATE tasks SET validated_at = datetime('now'), validated_by = 'thor'
             WHERE wave_id_fk = ?1 AND status = 'done'",
            rusqlite::params![wave_db_id],
        )
        .unwrap();

    let result = validate_wave(wave_db_id, "thor-per-wave", &pool);
    assert!(result.is_ok(), "expected Ok, got: {:?}", result.err());
    let r = result.unwrap();
    assert_eq!(r.tasks_validated, 2); // 2 promoted from submitted
    assert_eq!(r.wave_status, "done");

    // All tasks in wave must now be done with validated_at
    let missing_count: i64 = pool
        .get()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE wave_id_fk = ?1 AND (status != 'done' OR validated_at IS NULL)",
            rusqlite::params![wave_db_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(missing_count, 0, "all tasks must be done with validated_at");
}

#[test]
fn validate_wave_blocks_on_unresolved_tasks() {
    let pool = make_pool();
    insert_plan(&pool, 6, 0, 2);
    let wave_db_id = insert_wave(&pool, 60, 6, 1, "in_progress");
    insert_task(&pool, 6, wave_db_id, "submitted");
    insert_task(&pool, 6, wave_db_id, "in_progress"); // unresolved

    let result = validate_wave(wave_db_id, "thor-per-wave", &pool);
    assert!(
        result.is_err(),
        "expected Err for unresolved in_progress task"
    );
    assert!(
        result.unwrap_err().contains("unresolved"),
        "error should mention unresolved tasks"
    );
}

// --- check_wave_sequential ---

#[test]
fn check_wave_sequential_allows_first_wave() {
    let pool = make_pool();
    insert_plan(&pool, 7, 0, 0);
    let wave_db_id = insert_wave(&pool, 70, 7, 1, "pending");

    let result = check_wave_sequential(7, 1, &pool);
    assert!(result.is_ok(), "first wave should always be allowed");
}

#[test]
fn check_wave_sequential_allows_when_predecessors_done() {
    let pool = make_pool();
    insert_plan(&pool, 8, 0, 0);
    insert_wave(&pool, 80, 8, 1, "done");
    insert_wave(&pool, 81, 8, 2, "pending");

    let result = check_wave_sequential(8, 2, &pool);
    assert!(result.is_ok(), "should allow wave 2 when wave 1 is done");
}

#[test]
fn check_wave_sequential_blocks_when_predecessor_not_done() {
    let pool = make_pool();
    insert_plan(&pool, 9, 0, 0);
    insert_wave(&pool, 90, 9, 1, "in_progress");
    insert_wave(&pool, 91, 9, 2, "pending");

    let result = check_wave_sequential(9, 2, &pool);
    assert!(
        result.is_err(),
        "should block wave 2 when wave 1 is in_progress"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("must be completed"),
        "error should mention completion requirement, got: {err}"
    );
}

#[test]
fn check_wave_sequential_blocks_when_predecessor_merging() {
    // 'merging' is not terminal — only 'done' unblocks the next wave
    let pool = make_pool();
    insert_plan(&pool, 10, 0, 0);
    insert_wave(&pool, 100, 10, 1, "merging");
    insert_wave(&pool, 101, 10, 2, "pending");

    let result = check_wave_sequential(10, 2, &pool);
    assert!(
        result.is_err(),
        "merging status is not terminal — should block"
    );
}
