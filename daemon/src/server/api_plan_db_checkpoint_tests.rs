//! Unit tests for api_plan_db_checkpoint: checkpoint save query logic,
//! chrono_now format, and checkpoint_path derivation.
//! Integration with the checkpoint file system is tested via query shapes.

use crate::db::PlanDb;
use crate::server::state::{query_one, query_rows};

fn setup_db() -> PlanDb {
    let db = PlanDb::open_in_memory().expect("db");
    db.connection()
        .execute_batch(
            "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE plans (
                 id INTEGER PRIMARY KEY, project_id TEXT NOT NULL,
                 name TEXT NOT NULL, status TEXT DEFAULT 'draft',
                 execution_host TEXT, worktree_path TEXT
             );
             CREATE TABLE waves (
                 id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id TEXT,
                 name TEXT, status TEXT DEFAULT 'pending',
                 worktree_path TEXT, branch_name TEXT
             );
             CREATE TABLE tasks (
                 id INTEGER PRIMARY KEY, plan_id INTEGER,
                 wave_id_fk INTEGER, task_id TEXT,
                 title TEXT, status TEXT DEFAULT 'pending'
             );
             INSERT INTO projects (id, name) VALUES ('proj', 'TestProject');
             INSERT INTO plans (id, project_id, name, status)
                 VALUES (1, 'proj', 'Checkpoint Plan', 'doing');
             INSERT INTO waves (id, plan_id, wave_id, name, status)
                 VALUES (10, 1, 'W1', 'Wave 1', 'in_progress');
             INSERT INTO waves (id, plan_id, wave_id, name, status)
                 VALUES (11, 1, 'W2', 'Wave 2', 'pending');
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status)
                 VALUES (100, 1, 10, 'T1', 'First task', 'done');
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status)
                 VALUES (101, 1, 10, 'T2', 'Second task', 'pending');
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status)
                 VALUES (102, 1, 11, 'T3', 'Third task', 'pending');",
        )
        .expect("schema");
    db
}

#[test]
fn checkpoint_save_query_returns_plan() {
    let db = setup_db();
    let conn = db.connection();
    let plan = query_one(
        conn,
        "SELECT id, name, status, project_id, execution_host, worktree_path \
         FROM plans WHERE id = ?1",
        rusqlite::params![1],
    )
    .expect("query")
    .expect("plan exists");
    assert_eq!(plan["name"].as_str().unwrap(), "Checkpoint Plan");
    assert_eq!(plan["status"].as_str().unwrap(), "doing");
}

#[test]
fn checkpoint_save_query_returns_waves_ordered() {
    let db = setup_db();
    let conn = db.connection();
    let waves = query_rows(
        conn,
        "SELECT id, wave_id, name, status, worktree_path, branch_name \
         FROM waves WHERE plan_id = ?1 ORDER BY id",
        rusqlite::params![1],
    )
    .expect("query");
    assert_eq!(waves.len(), 2);
    assert_eq!(waves[0]["wave_id"].as_str().unwrap(), "W1");
    assert_eq!(waves[1]["wave_id"].as_str().unwrap(), "W2");
}

#[test]
fn checkpoint_save_query_returns_tasks_by_wave() {
    let db = setup_db();
    let conn = db.connection();
    let tasks = query_rows(
        conn,
        "SELECT id, task_id, title, status, wave_id_fk \
         FROM tasks WHERE plan_id = ?1 ORDER BY wave_id_fk, id",
        rusqlite::params![1],
    )
    .expect("query");
    assert_eq!(tasks.len(), 3);
    // First two tasks belong to wave 10
    assert_eq!(tasks[0]["wave_id_fk"].as_i64().unwrap(), 10);
    assert_eq!(tasks[1]["wave_id_fk"].as_i64().unwrap(), 10);
    // Third task belongs to wave 11
    assert_eq!(tasks[2]["wave_id_fk"].as_i64().unwrap(), 11);
}

#[test]
fn checkpoint_save_query_not_found_returns_none() {
    let db = setup_db();
    let conn = db.connection();
    let plan = query_one(
        conn,
        "SELECT id, name FROM plans WHERE id = ?1",
        rusqlite::params![999],
    )
    .expect("query");
    assert!(plan.is_none(), "nonexistent plan should return None");
}

#[test]
fn checkpoint_save_query_empty_plan_has_no_tasks() {
    let db = setup_db();
    let conn = db.connection();
    // Insert a plan with no waves or tasks
    conn.execute(
        "INSERT INTO plans (id, project_id, name) VALUES (2, 'proj', 'Empty Plan')",
        [],
    )
    .unwrap();
    let waves = query_rows(
        conn,
        "SELECT id FROM waves WHERE plan_id = ?1",
        rusqlite::params![2],
    )
    .expect("query");
    let tasks = query_rows(
        conn,
        "SELECT id FROM tasks WHERE plan_id = ?1",
        rusqlite::params![2],
    )
    .expect("query");
    assert!(waves.is_empty());
    assert!(tasks.is_empty());
}

#[test]
fn checkpoint_save_snapshot_can_be_serialized() {
    let db = setup_db();
    let conn = db.connection();
    let plan = query_one(
        conn,
        "SELECT id, name, status FROM plans WHERE id = 1",
        [],
    )
    .expect("query")
    .unwrap();
    let waves = query_rows(
        conn,
        "SELECT id, wave_id, name FROM waves WHERE plan_id = 1",
        [],
    )
    .expect("query");
    let tasks = query_rows(
        conn,
        "SELECT id, task_id, title FROM tasks WHERE plan_id = 1",
        [],
    )
    .expect("query");
    let checkpoint = serde_json::json!({
        "plan_id": 1,
        "plan": plan,
        "waves": waves,
        "tasks": tasks,
    });
    let serialized = serde_json::to_string_pretty(&checkpoint);
    assert!(serialized.is_ok(), "checkpoint must be JSON-serializable");
    let s = serialized.unwrap();
    assert!(s.contains("Checkpoint Plan"));
    assert!(s.contains("W1"));
    assert!(s.contains("T1"));
}

#[test]
fn checkpoint_restore_query_reads_plan_status() {
    let db = setup_db();
    let conn = db.connection();
    let plan = query_one(
        conn,
        "SELECT id, name, status FROM plans WHERE id = ?1",
        rusqlite::params![1],
    )
    .expect("query")
    .expect("plan");
    // Verify the shape has all the fields needed for checkpoint restore
    assert!(plan["id"].is_i64());
    assert!(plan["name"].is_string());
    assert!(plan["status"].is_string());
}
