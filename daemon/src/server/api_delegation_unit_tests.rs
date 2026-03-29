// Unit tests for delegation progress API — DB logic only (no HTTP layer).
// Extracted from api_delegation.rs to keep that file ≤250 lines.

use crate::db::PlanDb;

fn setup_db() -> PlanDb {
    let db = PlanDb::open_in_memory().expect("db");
    db.connection()
        .execute_batch(
            "CREATE TABLE delegation_progress (
                 id            INTEGER PRIMARY KEY AUTOINCREMENT,
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
fn upsert_progress_replaces_existing() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO delegation_progress (delegation_id, status, current_task)
         VALUES (?1, ?2, ?3)",
        rusqlite::params!["del-1", "running", "step-1"],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO delegation_progress
             (delegation_id, status, current_task, output_summary, updated_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))
         ON CONFLICT(delegation_id) DO UPDATE SET
             status         = excluded.status,
             current_task   = excluded.current_task,
             output_summary = excluded.output_summary,
             updated_at     = excluded.updated_at",
        rusqlite::params!["del-1", "done", "step-2", "ok"],
    )
    .unwrap();

    let (status, task): (String, String) = conn
        .query_row(
            "SELECT status, current_task FROM delegation_progress WHERE delegation_id='del-1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();

    assert_eq!(status, "done");
    assert_eq!(task, "step-2");
}

#[test]
fn get_returns_none_for_unknown_id() {
    let db = setup_db();
    let conn = db.connection();

    let result: Option<String> = conn
        .query_row(
            "SELECT status FROM delegation_progress WHERE delegation_id = ?1",
            rusqlite::params!["nope"],
            |r| r.get(0),
        )
        .ok();

    assert!(result.is_none());
}

#[test]
fn by_plan_returns_matching_delegations() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO delegation_progress (delegation_id, status, current_task) \
         VALUES (?1, ?2, ?3)",
        rusqlite::params!["del-742-M1Pro-1234", "running", "T1-01"],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO delegation_progress (delegation_id, status, current_task) \
         VALUES (?1, ?2, ?3)",
        rusqlite::params!["del-742-M5Max-5678", "done", "T2-01"],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO delegation_progress (delegation_id, status) \
         VALUES (?1, ?2)",
        rusqlite::params!["del-999-other-0", "running"],
    )
    .unwrap();

    // Query by plan_id=742 should return 2 rows
    let mut stmt = conn
        .prepare(
            "SELECT delegation_id FROM delegation_progress \
             WHERE delegation_id LIKE ?1 ORDER BY updated_at DESC",
        )
        .unwrap();
    let rows: Vec<String> = stmt
        .query_map(rusqlite::params!["del-742-%"], |r| r.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.starts_with("del-742-")));
}

#[test]
fn invalid_status_rejected() {
    let db = setup_db();
    let conn = db.connection();

    let err = conn
        .execute(
            "INSERT INTO delegation_progress (delegation_id, status) VALUES (?1, ?2)",
            rusqlite::params!["bad", "unknown"],
        )
        .unwrap_err();

    assert!(err.to_string().contains("CHECK"), "constraint must fire: {err}");
}
