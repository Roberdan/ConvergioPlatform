use super::*;
use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(
        "CREATE TABLE plans (
            id INTEGER PRIMARY KEY, project_id TEXT NOT NULL,
            name TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'draft',
            tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0,
            source_file TEXT, description TEXT,
            created_at TEXT, started_at TEXT, completed_at TEXT,
            updated_at TEXT, cancelled_at TEXT, cancelled_reason TEXT
        );
        CREATE TABLE plan_reviews (
            id INTEGER PRIMARY KEY,
            plan_id INTEGER NOT NULL DEFAULT 0,
            reviewer_agent TEXT NOT NULL DEFAULT '',
            verdict TEXT NOT NULL DEFAULT 'approved',
            reviewed_at TEXT DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE tasks (
            id INTEGER PRIMARY KEY, plan_id INTEGER,
            task_id TEXT, title TEXT, status TEXT DEFAULT 'pending'
        );",
    )
    .expect("schema");
    conn
}

// --- require_review ---

#[test]
fn require_review_rejects_when_no_review() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO plans (project_id, name, status) VALUES ('proj', 'Plan', 'draft')",
        [],
    )
    .unwrap();
    let err = require_review(1, &conn).unwrap_err();
    assert!(err.contains("REVIEW_REQUIRED"), "got: {err}");
}

#[test]
fn require_review_accepts_approved() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO plans (project_id, name, status) VALUES ('proj', 'Plan', 'draft')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict) \
         VALUES (1, 'plan-reviewer', 'approved')",
        [],
    )
    .unwrap();
    assert!(require_review(1, &conn).is_ok());
}

#[test]
fn require_review_accepts_proceed() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO plans (project_id, name, status) VALUES ('proj', 'Plan', 'draft')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict) \
         VALUES (1, 'plan-reviewer', 'proceed')",
        [],
    )
    .unwrap();
    assert!(require_review(1, &conn).is_ok());
}

#[test]
fn require_review_rejects_revise_verdict() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO plans (project_id, name, status) VALUES ('proj', 'Plan', 'draft')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict) \
         VALUES (1, 'plan-reviewer', 'revise')",
        [],
    )
    .unwrap();
    let err = require_review(1, &conn).unwrap_err();
    assert!(err.contains("REVIEW_REQUIRED"), "got: {err}");
}

// --- require_plan_exists ---

#[test]
fn require_plan_exists_returns_status() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO plans (project_id, name, status) \
         VALUES ('proj', 'Plan', 'doing')",
        [],
    )
    .unwrap();
    assert_eq!(require_plan_exists(1, &conn).unwrap(), "doing");
}

#[test]
fn require_plan_exists_rejects_missing() {
    let conn = setup_db();
    let err = require_plan_exists(999, &conn).unwrap_err();
    assert!(err.contains("PLAN_NOT_FOUND"), "got: {err}");
}

// --- require_plan_importable ---

#[test]
fn require_plan_importable_accepts_draft() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO plans (project_id, name, status) VALUES ('proj', 'P', 'draft')",
        [],
    )
    .unwrap();
    assert!(require_plan_importable(1, &conn).is_ok());
}

#[test]
fn require_plan_importable_accepts_todo() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO plans (project_id, name, status) VALUES ('proj', 'P', 'todo')",
        [],
    )
    .unwrap();
    assert!(require_plan_importable(1, &conn).is_ok());
}

#[test]
fn require_plan_importable_rejects_doing() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO plans (project_id, name, status) VALUES ('proj', 'P', 'doing')",
        [],
    )
    .unwrap();
    let err = require_plan_importable(1, &conn).unwrap_err();
    assert!(err.contains("PLAN_NOT_IMPORTABLE"), "got: {err}");
}

#[test]
fn require_plan_importable_rejects_missing() {
    let conn = setup_db();
    let err = require_plan_importable(999, &conn).unwrap_err();
    assert!(err.contains("PLAN_NOT_FOUND"), "got: {err}");
}

// --- require_plan_startable ---

#[test]
fn require_plan_startable_rejects_no_tasks() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO plans (project_id, name, status, tasks_total) \
         VALUES ('proj', 'P', 'draft', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict) \
         VALUES (1, 'reviewer', 'approved')",
        [],
    )
    .unwrap();
    let err = require_plan_startable(1, &conn).unwrap_err();
    assert!(err.contains("NO_SPEC_IMPORTED"), "got: {err}");
}

#[test]
fn require_plan_startable_rejects_no_review() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO plans (project_id, name, status, tasks_total) \
         VALUES ('proj', 'P', 'draft', 5)",
        [],
    )
    .unwrap();
    let err = require_plan_startable(1, &conn).unwrap_err();
    assert!(err.contains("REVIEW_REQUIRED"), "got: {err}");
}

#[test]
fn require_plan_startable_accepts_valid() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO plans (project_id, name, status, tasks_total) \
         VALUES ('proj', 'P', 'draft', 3)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict) \
         VALUES (1, 'reviewer', 'approved')",
        [],
    )
    .unwrap();
    assert!(require_plan_startable(1, &conn).is_ok());
}
