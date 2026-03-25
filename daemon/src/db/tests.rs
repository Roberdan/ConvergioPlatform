use super::{PlanDb, TaskStatus, UpdateTaskArgs};

#[path = "tests_crud.rs"]
mod tests_crud;

// BUG 4 — SQLite retry with exponential backoff
// These tests call the public retry API and verify correct attempt counts.
#[test]
fn db_retry_succeeds_on_first_attempt() {
    let mut attempts = 0u32;
    let result = super::with_retry(3, || {
        attempts += 1;
        Ok::<i32, rusqlite::Error>(42)
    });
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
    assert_eq!(attempts, 1, "should succeed on first attempt");
}

#[test]
fn db_retry_retries_on_busy_error() {
    let mut attempts = 0u32;
    let result = super::with_retry(3, || {
        attempts += 1;
        if attempts < 3 {
            // Simulate SQLITE_BUSY by returning an error
            Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ffi::ErrorCode::DatabaseBusy,
                    extended_code: 5, // SQLITE_BUSY
                },
                Some("database is locked".to_string()),
            ))
        } else {
            Ok::<i32, rusqlite::Error>(99)
        }
    });
    assert!(result.is_ok(), "should succeed after retries");
    assert_eq!(result.unwrap(), 99);
    assert_eq!(attempts, 3, "should have taken 3 attempts");
}

#[test]
fn db_retry_gives_up_after_max_attempts() {
    let mut attempts = 0u32;
    let result = super::with_retry(3, || {
        attempts += 1;
        Err::<i32, rusqlite::Error>(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ffi::ErrorCode::DatabaseBusy,
                extended_code: 5,
            },
            Some("database is locked".to_string()),
        ))
    });
    assert!(result.is_err(), "should fail after exhausting retries");
    assert_eq!(attempts, 3, "should have tried exactly 3 times");
}

#[test]
fn db_retry_does_not_retry_non_busy_errors() {
    let mut attempts = 0u32;
    let result = super::with_retry(3, || {
        attempts += 1;
        Err::<i32, rusqlite::Error>(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error {
                code: rusqlite::ffi::ErrorCode::ConstraintViolation,
                extended_code: 19,
            },
            Some("UNIQUE constraint failed".to_string()),
        ))
    });
    assert!(result.is_err(), "non-busy error should propagate immediately");
    assert_eq!(attempts, 1, "should not retry on non-BUSY error");
}

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

#[test]
fn db_status_filters_by_project() {
    let db = PlanDb::open_in_memory().expect("db");
    seed_schema(&db);

    db.connection()
        .execute(
            "INSERT INTO projects(id,name) VALUES('p1','P1'),('p2','P2')",
            [],
        )
        .expect("projects");
    db.connection()
        .execute(
            "INSERT INTO plans(id,project_id,name,status,tasks_done,tasks_total)
             VALUES(1,'p1','Plan A','doing',1,3),(2,'p2','Plan B','doing',0,2)",
            [],
        )
        .expect("plans");
    db.connection()
        .execute(
            "INSERT INTO waves(id,plan_id,wave_id,name,status,tasks_done,tasks_total,position)
             VALUES(10,1,'W1','Wave 1','in_progress',1,2,1),(20,2,'W1','Wave 1','in_progress',0,1,1)",
            [],
        )
        .expect("waves");
    db.connection()
        .execute(
            "INSERT INTO tasks(id,project_id,plan_id,wave_id_fk,wave_id,task_id,title,status)
             VALUES(100,'p1',1,10,'W1','T1','Task 1','in_progress'),(200,'p2',2,20,'W1','T2','Task 2','in_progress')",
            [],
        )
        .expect("tasks");

    let status = db.status(Some("p1")).expect("status");
    assert_eq!(status.active_plans.len(), 1);
    assert_eq!(status.active_plans[0].project_id, "p1");
    assert_eq!(status.in_progress_tasks.len(), 1);
    assert_eq!(status.in_progress_tasks[0].project_id, "p1");
}

#[test]
fn db_update_task_is_injection_safe() {
    let db = PlanDb::open_in_memory().expect("db");
    seed_schema(&db);
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
            "INSERT INTO tasks(id,project_id,plan_id,wave_id_fk,wave_id,task_id,title,status) VALUES(100,'p1',1,10,'W1','T1','Task 1','pending')",
            [],
        )
        .expect("tasks");

    let args = UpdateTaskArgs {
        notes: Some("x'; DROP TABLE tasks; --".to_string()),
        ..UpdateTaskArgs::default()
    };
    db.update_task(100, TaskStatus::InProgress, &args)
        .expect("update-task");
    let count: i64 = db
        .connection()
        .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
        .expect("tasks still exists");
    assert_eq!(count, 1);
}
