// Sync subcommand routing test extracted from libsql_adapter_tests.rs.
// Why: keep libsql_adapter_tests.rs ≤250 lines per CONSTITUTION Article V.

use rusqlite::{params, Connection};
use serde_json::json;

use crate::db::libsql_adapter::SyncChange;
use crate::db::libsql_adapter_apply::apply_changes;

fn setup_conflict_schema(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE tasks (
            id INTEGER PRIMARY KEY,
            title TEXT,
            status TEXT DEFAULT 'pending',
            updated_at TEXT
         );
         CREATE TABLE _sync_conflicts (
            id INTEGER PRIMARY KEY,
            table_name TEXT NOT NULL,
            pk INTEGER,
            local_data TEXT,
            remote_data TEXT,
            source_node TEXT DEFAULT '',
            resolved INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now'))
         );",
    )
    .expect("conflict schema");
}

#[test]
fn test_apply_changes_conflict_logged_when_remote_is_newer() {
    let conn = Connection::open_in_memory().expect("db");
    setup_conflict_schema(&conn);

    // Insert a local row with an older timestamp.
    conn.execute(
        "INSERT INTO tasks (id, title, status, updated_at) VALUES (1, 'old title', 'pending', '2026-03-01 10:00:00')",
        [],
    )
    .unwrap();

    // Remote change has a newer updated_at and different data.
    let changes = vec![SyncChange {
        table_name: "tasks".to_string(),
        pk: 1,
        data: json!({
            "id": 1,
            "title": "new title",
            "status": "in_progress",
            "updated_at": "2026-03-02 10:00:00"
        }),
    }];

    let applied = apply_changes(&conn, &changes).unwrap();
    assert_eq!(applied, 1, "remote-newer row must be applied");

    let conflict_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _sync_conflicts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(conflict_count, 1, "_sync_conflicts must have exactly 1 row");
}

#[test]
fn test_apply_changes_no_conflict_when_same_updated_at() {
    let conn = Connection::open_in_memory().expect("db");
    setup_conflict_schema(&conn);

    let ts = "2026-03-02 10:00:00";
    conn.execute(
        "INSERT INTO tasks (id, title, status, updated_at) VALUES (2, 'same title', 'pending', ?1)",
        params![ts],
    )
    .unwrap();

    // Remote change has the SAME updated_at — should be skipped (local is same age).
    let changes = vec![SyncChange {
        table_name: "tasks".to_string(),
        pk: 2,
        data: json!({
            "id": 2,
            "title": "same title",
            "status": "pending",
            "updated_at": ts
        }),
    }];

    let applied = apply_changes(&conn, &changes).unwrap();
    assert_eq!(applied, 0, "same-timestamp row must be skipped");

    let conflict_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _sync_conflicts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(conflict_count, 0, "no conflict row expected for same timestamp");
}

#[test]
fn cli_sync_commands_point_to_timestamp_adapter() {
    // After crsqlite removal, sync CLI commands should direct users
    // to the timestamp-based sync (libsql_adapter).
    let db = crate::db::PlanDb::open_in_memory().expect("db");
    // Seed minimal schema for subcommand dispatch
    db.connection()
        .execute_batch(
            "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT);
             CREATE TABLE plans (id INTEGER PRIMARY KEY, project_id TEXT, name TEXT, status TEXT, tasks_done INTEGER DEFAULT 0, tasks_total INTEGER DEFAULT 0);
             CREATE TABLE waves (id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id TEXT, name TEXT, status TEXT, tasks_done INTEGER DEFAULT 0, tasks_total INTEGER DEFAULT 0, position INTEGER DEFAULT 0);
             CREATE TABLE tasks (id INTEGER PRIMARY KEY, project_id TEXT, plan_id INTEGER, wave_id_fk INTEGER, wave_id TEXT, task_id TEXT, title TEXT, status TEXT, started_at TEXT, completed_at TEXT, notes TEXT, tokens INTEGER, output_data TEXT, executor_host TEXT, validated_at TEXT, validated_by TEXT, validation_report TEXT);",
        )
        .expect("schema");
    for cmd in &["export-changes", "apply-changes", "sync"] {
        let err = db
            .run_subcommand(&[cmd.to_string()])
            .expect_err(&format!("{cmd} should return error"));
        let msg = err.to_string();
        assert!(
            msg.contains("timestamp-based sync") || msg.contains("libsql_adapter"),
            "{cmd} error should mention timestamp sync, got: {msg}"
        );
    }
}
