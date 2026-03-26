/// Tests that debug_assert! boundaries at DB module entry points
/// do NOT fire on valid/normal inputs (GREEN path only — assertions
/// are debug-only and won't panic in release builds).
///
/// F-06: Assertions at public module boundaries.
use super::{PlanDb, TaskStatus, UpdateTaskArgs};

fn build_db_with_schema() -> PlanDb {
    let db = PlanDb::open_in_memory().unwrap();
    db.connection()
        .execute_batch(
            "
            CREATE TABLE projects (
              id TEXT PRIMARY KEY, name TEXT NOT NULL
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
            INSERT INTO projects VALUES ('proj-1', 'Alpha Project');
            INSERT INTO plans VALUES (1, 'proj-1', 'Plan Alpha', 'doing', 0, 1);
            INSERT INTO waves VALUES (1, 1, 'W1', 'Wave 1', 'in_progress', 0, 1, 0);
            INSERT INTO tasks VALUES (
              1, 'proj-1', 1, 1, 'W1', 'T1-01', 'Task One',
              'pending', NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL
            );
            ",
        )
        .unwrap();
    db
}

/// Positive task_id (> 0) must not trigger the debug_assert in update_task.
#[test]
fn update_task_positive_id_no_assertion_fire() {
    let db = build_db_with_schema();
    // task_id=1 satisfies debug_assert!(task_id > 0)
    let result = db.update_task(
        1,
        TaskStatus::InProgress,
        &UpdateTaskArgs {
            notes: Some("started".into()),
            executor_host: None,
            tokens: None,
            output_data: None,
        },
    );
    assert!(result.is_ok(), "update_task with valid id should succeed: {result:?}");
    let r = result.unwrap();
    assert_eq!(r.old_status, "pending");
    assert_eq!(r.new_status, "in_progress");
}

/// plan_id > 0 must satisfy the debug_assert in handle_plan_validate-equivalent DB calls.
/// We verify the status API accepts non-negative project_id filtering without panic.
#[test]
fn status_with_non_empty_project_id_no_assertion_fire() {
    let db = build_db_with_schema();
    // Non-empty project_id satisfies debug_assert!(!project_id.is_empty())
    let result = db.status(Some("proj-1"));
    assert!(result.is_ok(), "status with valid project_id should succeed: {result:?}");
    let view = result.unwrap();
    assert!(!view.active_plans.is_empty(), "should have one active plan");
    assert_eq!(view.active_plans[0].name, "Plan Alpha");
}

/// status() with None project_id must work (no assertion on optional field).
#[test]
fn status_without_project_id_no_assertion_fire() {
    let db = build_db_with_schema();
    let result = db.status(None);
    assert!(result.is_ok(), "status with None should succeed: {result:?}");
}
