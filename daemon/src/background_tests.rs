use super::background::*;
use rusqlite::Connection;
use std::collections::HashSet;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch(
        "CREATE TABLE coordinator_events (
             id INTEGER PRIMARY KEY,
             event_type TEXT NOT NULL DEFAULT '',
             payload TEXT,
             source_node TEXT,
             handled_at TEXT DEFAULT (datetime('now'))
         );
         CREATE TABLE execution_runs (
             id INTEGER PRIMARY KEY,
             goal TEXT NOT NULL DEFAULT '',
             team TEXT NOT NULL DEFAULT '[]',
             status TEXT NOT NULL DEFAULT 'running'
                 CHECK(status IN ('running','completed','failed','cancelled','paused')),
             result TEXT,
             cost_usd REAL NOT NULL DEFAULT 0,
             agents_used INTEGER NOT NULL DEFAULT 0,
             plan_id INTEGER,
             started_at TEXT NOT NULL DEFAULT (datetime('now')),
             completed_at TEXT,
             duration_minutes REAL,
             context_path TEXT,
             paused_at TEXT,
             paused_context TEXT
         );",
    )
    .expect("setup schema");
    conn
}

#[test]
fn extract_plan_id_parses_json() {
    assert_eq!(extract_plan_id(Some(r#"{"plan_id": 42}"#)), Some(42));
    assert_eq!(extract_plan_id(Some(r#"{"plan_id": 0}"#)), Some(0));
    assert_eq!(extract_plan_id(None), None);
    assert_eq!(extract_plan_id(Some("")), None);
    assert_eq!(extract_plan_id(Some(r#"{"no_id": 1}"#)), None);
}

#[test]
fn pause_sets_status_and_paused_at() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO execution_runs (goal, plan_id, status) VALUES ('goal', 10, 'running')",
        [],
    )
    .unwrap();
    let affected = apply_pause(&conn, 10).unwrap();
    assert_eq!(affected, 1);
    let (status, paused_at): (String, Option<String>) = conn
        .query_row(
            "SELECT status, paused_at FROM execution_runs WHERE plan_id=10",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "paused");
    assert!(paused_at.is_some(), "paused_at must be set");
}

#[test]
fn resume_clears_paused_at() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO execution_runs (goal, plan_id, status, paused_at) \
         VALUES ('goal', 20, 'paused', datetime('now'))",
        [],
    )
    .unwrap();
    let affected = apply_resume(&conn, 20).unwrap();
    assert_eq!(affected, 1);
    let (status, paused_at): (String, Option<String>) = conn
        .query_row(
            "SELECT status, paused_at FROM execution_runs WHERE plan_id=20",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "running");
    assert!(paused_at.is_none(), "paused_at must be cleared on resume");
}

#[test]
fn pause_only_affects_running_rows() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO execution_runs (goal, plan_id, status) VALUES ('g', 30, 'completed')",
        [],
    )
    .unwrap();
    let affected = apply_pause(&conn, 30).unwrap();
    assert_eq!(affected, 0, "completed rows must not be paused");
}

#[test]
fn process_tick_skips_seen_events() {
    let conn = setup_db();
    conn.execute(
        "INSERT INTO coordinator_events (event_type, payload) \
         VALUES ('pause_run', '{\"plan_id\": 5}')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO execution_runs (goal, plan_id) VALUES ('g', 5)",
        [],
    )
    .unwrap();
    let mut seen = HashSet::new();
    process_tick(&conn, &mut seen).unwrap();
    let status: String = conn
        .query_row("SELECT status FROM execution_runs WHERE plan_id=5", [], |r| r.get(0))
        .unwrap();
    assert_eq!(status, "paused");
    conn.execute(
        "UPDATE execution_runs SET status='running', paused_at=NULL WHERE plan_id=5",
        [],
    )
    .unwrap();
    process_tick(&conn, &mut seen).unwrap();
    let status2: String = conn
        .query_row("SELECT status FROM execution_runs WHERE plan_id=5", [], |r| r.get(0))
        .unwrap();
    assert_eq!(status2, "running", "seen events must not be re-processed");
}
