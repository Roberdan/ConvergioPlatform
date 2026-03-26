// Tests for F-07: side-effect naming convention.
// All DB-writing public functions must use write_ prefix.
// These tests call the renamed functions to enforce convention at compile time.

use crate::db::{PlanDb, TaskStatus, UpdateTaskArgs, ValidateTaskArgs};

// Reuse the schema setup from tests.rs
fn seed_schema(db: &PlanDb) {
    db.connection()
        .execute_batch(
            "
            CREATE TABLE projects (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL
            );
            CREATE TABLE plans (
              id INTEGER PRIMARY KEY,
              project_id TEXT NOT NULL,
              name TEXT NOT NULL,
              status TEXT NOT NULL,
              tasks_done INTEGER DEFAULT 0,
              tasks_total INTEGER DEFAULT 0
            );
            CREATE TABLE waves (
              id INTEGER PRIMARY KEY,
              plan_id INTEGER NOT NULL,
              wave_id TEXT NOT NULL,
              name TEXT NOT NULL,
              status TEXT NOT NULL,
              tasks_done INTEGER DEFAULT 0,
              tasks_total INTEGER DEFAULT 0,
              position INTEGER DEFAULT 0
            );
            CREATE TABLE tasks (
              id INTEGER PRIMARY KEY,
              project_id TEXT NOT NULL,
              plan_id INTEGER NOT NULL,
              wave_id_fk INTEGER NOT NULL,
              wave_id TEXT NOT NULL,
              task_id TEXT NOT NULL,
              title TEXT NOT NULL,
              status TEXT NOT NULL,
              started_at TEXT,
              completed_at TEXT,
              notes TEXT,
              tokens INTEGER,
              output_data TEXT,
              executor_host TEXT,
              validated_at TEXT,
              validated_by TEXT,
              validation_report TEXT
            );
            ",
        )
        .expect("schema");
}

fn insert_plan_with_task(db: &PlanDb, task_status: &str) {
    db.connection()
        .execute("INSERT INTO projects(id,name) VALUES('p1','P1')", [])
        .expect("projects");
    db.connection()
        .execute(
            "INSERT INTO plans(id,project_id,name,status,tasks_done,tasks_total) VALUES(1,'p1','Plan A','doing',0,1)",
            [],
        )
        .expect("plans");
    db.connection()
        .execute(
            "INSERT INTO waves(id,plan_id,wave_id,name,status,tasks_done,tasks_total,position) VALUES(10,1,'W1','Wave 1','pending',0,1,1)",
            [],
        )
        .expect("waves");
    db.connection()
        .execute(
            &format!(
                "INSERT INTO tasks(id,project_id,plan_id,wave_id_fk,wave_id,task_id,title,status) VALUES(100,'p1',1,10,'W1','T1','Task 1','{task_status}')"
            ),
            [],
        )
        .expect("tasks");
}

/// F-07: write_task_status must write to DB and return the status transition.
#[test]
fn write_task_status_transitions_pending_to_in_progress() {
    let db = PlanDb::open_in_memory().expect("db");
    seed_schema(&db);
    insert_plan_with_task(&db, "pending");

    let args = UpdateTaskArgs::default();
    let result = db
        .write_task_status(100, TaskStatus::InProgress, &args)
        .expect("write_task_status");
    assert_eq!(result.old_status, "pending");
    assert_eq!(result.new_status, "in_progress");
}

/// F-07: write_task_validated must mark a submitted task as done.
#[test]
fn write_task_validated_transitions_submitted_to_done() {
    let db = PlanDb::open_in_memory().expect("db");
    seed_schema(&db);
    insert_plan_with_task(&db, "submitted");

    let args = ValidateTaskArgs {
        identifier: "100".to_string(),
        validated_by: "thor".to_string(),
        ..ValidateTaskArgs::default()
    };
    let result = db
        .write_task_validated(&args)
        .expect("write_task_validated");
    assert_eq!(result.old_status, "submitted");
    assert_eq!(result.new_status, "done");
}
