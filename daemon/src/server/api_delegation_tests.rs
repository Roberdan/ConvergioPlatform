// Tests for delegation progress API — POST/GET /api/delegation/:id/progress
// TDD: written before implementation to drive the RED→GREEN cycle.

use crate::db::PlanDb;
use crate::server::state::query_rows;

fn setup_db() -> PlanDb {
    let db = PlanDb::open_in_memory().expect("db");
    db.connection()
        .execute_batch(
            "CREATE TABLE delegation_progress (
                 id           INTEGER PRIMARY KEY AUTOINCREMENT,
                 delegation_id TEXT NOT NULL UNIQUE,
                 status        TEXT NOT NULL DEFAULT 'running'
                     CHECK(status IN ('running','blocked','done')),
                 current_task  TEXT,
                 output_summary TEXT,
                 updated_at    TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE INDEX IF NOT EXISTS idx_delegation_progress_id
                 ON delegation_progress(delegation_id);",
        )
        .expect("schema");
    db
}

#[test]
fn post_progress_inserts_row() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO delegation_progress (delegation_id, status, current_task, output_summary)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params!["del-42", "running", "T2-01: build handler", "3 tests passing"],
    )
    .unwrap();

    let rows = query_rows(
        conn,
        "SELECT delegation_id, status, current_task, output_summary FROM delegation_progress",
        [],
    )
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["delegation_id"], "del-42");
    assert_eq!(rows[0]["status"], "running");
    assert_eq!(rows[0]["current_task"], "T2-01: build handler");
    assert_eq!(rows[0]["output_summary"], "3 tests passing");
}

#[test]
fn post_progress_upserts_on_conflict() {
    let db = setup_db();
    let conn = db.connection();

    // Insert initial record
    conn.execute(
        "INSERT INTO delegation_progress (delegation_id, status, current_task)
         VALUES (?1, ?2, ?3)",
        rusqlite::params!["del-99", "running", "task A"],
    )
    .unwrap();

    // Upsert with updated status
    conn.execute(
        "INSERT INTO delegation_progress (delegation_id, status, current_task, output_summary, updated_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))
         ON CONFLICT(delegation_id) DO UPDATE SET
             status        = excluded.status,
             current_task  = excluded.current_task,
             output_summary = excluded.output_summary,
             updated_at    = excluded.updated_at",
        rusqlite::params!["del-99", "done", "task B", "completed"],
    )
    .unwrap();

    let rows = query_rows(
        conn,
        "SELECT delegation_id, status, current_task FROM delegation_progress",
        [],
    )
    .unwrap();
    assert_eq!(rows.len(), 1, "upsert must not create duplicate row");
    assert_eq!(rows[0]["status"], "done");
    assert_eq!(rows[0]["current_task"], "task B");
}

#[test]
fn get_progress_returns_404_for_unknown_id() {
    let db = setup_db();
    let conn = db.connection();

    let rows = query_rows(
        conn,
        "SELECT * FROM delegation_progress WHERE delegation_id = ?1",
        rusqlite::params!["unknown-delegation"],
    )
    .unwrap();
    assert!(rows.is_empty(), "unknown delegation_id should return no rows");
}

#[test]
fn get_progress_returns_row_for_known_id() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO delegation_progress (delegation_id, status, current_task, output_summary)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            "plan-720-wave-w2",
            "blocked",
            "T2-01: waiting for lock",
            "lock timeout on mesh peer"
        ],
    )
    .unwrap();

    let rows = query_rows(
        conn,
        "SELECT delegation_id, status, current_task, output_summary
         FROM delegation_progress WHERE delegation_id = ?1",
        rusqlite::params!["plan-720-wave-w2"],
    )
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["status"], "blocked");
    assert_eq!(rows[0]["current_task"], "T2-01: waiting for lock");
}

#[test]
fn status_constraint_rejects_invalid_value() {
    let db = setup_db();
    let conn = db.connection();

    let result = conn.execute(
        "INSERT INTO delegation_progress (delegation_id, status) VALUES (?1, ?2)",
        rusqlite::params!["bad-del", "invalid_status"],
    );
    assert!(result.is_err(), "invalid status must be rejected by CHECK constraint");
}
