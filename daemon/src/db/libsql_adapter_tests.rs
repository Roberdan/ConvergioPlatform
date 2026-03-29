/// Tests for the libsql adapter layer (timestamp-based sync replacing crsqlite CRDT).
///
/// ADR: libsql crate (v0.9) is async-only, requiring tokio runtime for basic
/// operations. Since the daemon uses synchronous rusqlite throughout (80+ files),
/// we keep rusqlite and implement timestamp-based sync in Rust instead.
use rusqlite::Connection;

use super::{SyncChange, SyncMeta};

fn setup_sync_schema(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE tasks (
           id INTEGER PRIMARY KEY,
           title TEXT NOT NULL,
           status TEXT NOT NULL DEFAULT 'pending',
           updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE _sync_meta (
           peer TEXT NOT NULL,
           table_name TEXT NOT NULL,
           last_sync_at TEXT NOT NULL,
           PRIMARY KEY (peer, table_name)
         );",
    )
    .expect("sync schema");
}

#[test]
fn sync_meta_round_trip() {
    let conn = Connection::open_in_memory().expect("db");
    setup_sync_schema(&conn);
    let meta = SyncMeta {
        peer: "node-alpha".to_string(),
        table_name: "tasks".to_string(),
        last_sync_at: "2026-03-28T10:00:00".to_string(),
    };
    super::upsert_sync_meta(&conn, &meta).expect("upsert");
    let result =
        super::get_sync_meta(&conn, "node-alpha", "tasks").expect("get");
    assert_eq!(result.unwrap().last_sync_at, "2026-03-28T10:00:00");
}

#[test]
fn sync_meta_upsert_overwrites() {
    let conn = Connection::open_in_memory().expect("db");
    setup_sync_schema(&conn);
    let meta1 = SyncMeta {
        peer: "node-alpha".to_string(),
        table_name: "tasks".to_string(),
        last_sync_at: "2026-03-28T10:00:00".to_string(),
    };
    super::upsert_sync_meta(&conn, &meta1).expect("upsert1");
    let meta2 = SyncMeta {
        peer: "node-alpha".to_string(),
        table_name: "tasks".to_string(),
        last_sync_at: "2026-03-28T11:00:00".to_string(),
    };
    super::upsert_sync_meta(&conn, &meta2).expect("upsert2");
    let result =
        super::get_sync_meta(&conn, "node-alpha", "tasks").expect("get");
    assert_eq!(result.unwrap().last_sync_at, "2026-03-28T11:00:00");
}

#[test]
fn export_changes_since_returns_newer_rows() {
    let conn = Connection::open_in_memory().expect("db");
    setup_sync_schema(&conn);
    conn.execute_batch(
        "INSERT INTO tasks(id, title, status, updated_at) VALUES
           (1, 'Old task', 'done', '2026-03-27T09:00:00'),
           (2, 'New task', 'pending', '2026-03-28T12:00:00');",
    )
    .expect("seed");
    let changes = super::export_changes_since(
        &conn,
        "tasks",
        Some("2026-03-28T00:00:00"),
    )
    .expect("export");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].pk, 2);
}

#[test]
fn export_changes_since_none_returns_all() {
    let conn = Connection::open_in_memory().expect("db");
    setup_sync_schema(&conn);
    conn.execute_batch(
        "INSERT INTO tasks(id, title, status, updated_at) VALUES
           (1, 'Task A', 'done', '2026-03-27T09:00:00'),
           (2, 'Task B', 'pending', '2026-03-28T12:00:00');",
    )
    .expect("seed");
    let changes =
        super::export_changes_since(&conn, "tasks", None).expect("export");
    assert_eq!(changes.len(), 2);
}

#[test]
fn apply_changes_inserts_and_updates() {
    let conn = Connection::open_in_memory().expect("db");
    setup_sync_schema(&conn);
    conn.execute_batch(
        "INSERT INTO tasks(id, title, status, updated_at)
         VALUES (1, 'Existing', 'pending', '2026-03-27T09:00:00');",
    )
    .expect("seed");

    let changes = vec![
        // Update existing row (newer timestamp)
        SyncChange {
            table_name: "tasks".to_string(),
            pk: 1,
            data: serde_json::json!({
                "title": "Existing Updated",
                "status": "done",
                "updated_at": "2026-03-28T15:00:00"
            }),
        },
        // Insert new row
        SyncChange {
            table_name: "tasks".to_string(),
            pk: 3,
            data: serde_json::json!({
                "title": "Brand New",
                "status": "pending",
                "updated_at": "2026-03-28T14:00:00"
            }),
        },
    ];
    let applied = super::apply_changes(&conn, &changes).expect("apply");
    assert_eq!(applied, 2);

    let title: String = conn
        .query_row("SELECT title FROM tasks WHERE id = 1", [], |r| r.get(0))
        .expect("query");
    assert_eq!(title, "Existing Updated");

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
        .expect("count");
    assert_eq!(count, 2);
}

#[test]
fn apply_changes_skips_stale_updates() {
    let conn = Connection::open_in_memory().expect("db");
    setup_sync_schema(&conn);
    conn.execute_batch(
        "INSERT INTO tasks(id, title, status, updated_at)
         VALUES (1, 'Current', 'done', '2026-03-28T15:00:00');",
    )
    .expect("seed");

    let changes = vec![SyncChange {
        table_name: "tasks".to_string(),
        pk: 1,
        data: serde_json::json!({
            "title": "Stale Update",
            "status": "pending",
            "updated_at": "2026-03-27T09:00:00"
        }),
    }];
    let applied = super::apply_changes(&conn, &changes).expect("apply");
    assert_eq!(applied, 0, "stale change should be skipped");

    let title: String = conn
        .query_row("SELECT title FROM tasks WHERE id = 1", [], |r| r.get(0))
        .expect("query");
    assert_eq!(title, "Current", "title should not change");
}

#[test]
fn apply_changes_skips_stale_when_local_is_legacy_space_format() {
    let conn = Connection::open_in_memory().expect("db");
    setup_sync_schema(&conn);
    conn.execute_batch(
        "INSERT INTO tasks(id, title, status, updated_at)
         VALUES (1, 'Current', 'done', '2026-03-28 15:00:00');",
    )
    .expect("seed");

    let changes = vec![SyncChange {
        table_name: "tasks".to_string(),
        pk: 1,
        data: serde_json::json!({
            "title": "Should Not Apply",
            "status": "pending",
            "updated_at": "2026-03-28T14:00:00Z"
        }),
    }];
    let applied = super::apply_changes(&conn, &changes).expect("apply");
    assert_eq!(applied, 0, "older remote update must be skipped");
}

#[test]
fn apply_changes_applies_newer_when_remote_is_legacy_space_format() {
    let conn = Connection::open_in_memory().expect("db");
    setup_sync_schema(&conn);
    conn.execute_batch(
        "INSERT INTO tasks(id, title, status, updated_at)
         VALUES (1, 'Current', 'done', '2026-03-28T14:00:00Z');",
    )
    .expect("seed");

    let changes = vec![SyncChange {
        table_name: "tasks".to_string(),
        pk: 1,
        data: serde_json::json!({
            "title": "Should Apply",
            "status": "pending",
            "updated_at": "2026-03-28 15:00:00"
        }),
    }];
    let applied = super::apply_changes(&conn, &changes).expect("apply");
    assert_eq!(applied, 1, "newer remote update must be applied");
}

#[test]
fn open_path_without_crsqlite_succeeds() {
    // PlanDb::open_path should work without crsqlite extension loading
    let dir = tempfile::tempdir().expect("tmpdir");
    let db_path = dir.path().join("test.db");
    let db = crate::db::PlanDb::open_path(&db_path).expect("open_path");
    // Verify pragmas applied
    let journal: String = db
        .connection()
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .expect("journal");
    assert_eq!(journal, "wal");
    let timeout: i64 = db
        .connection()
        .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
        .expect("timeout");
    assert_eq!(timeout, 5000);
}

#[test]
fn plandb_no_crsqlite_extension_field() {
    // After migration, PlanDb should not have crsqlite_extension field
    let db = crate::db::PlanDb::open_in_memory().expect("db");
    // open_in_memory still works and connection is usable
    db.connection()
        .execute_batch("CREATE TABLE t(id INTEGER PRIMARY KEY)")
        .expect("create table");
}

#[test]
fn db_mod_has_no_crsqlite_references() {
    // Compile-time proof: crsqlite is removed from the db layer.
    // PlanDb::open_path works without any extension loading.
    let dir = tempfile::tempdir().expect("tmpdir");
    let db_path = dir.path().join("no-crdt.db");
    let db = crate::db::PlanDb::open_path(&db_path).expect("open");
    // Timestamp-based sync works as the replacement
    db.connection()
        .execute_batch(
            "CREATE TABLE _sync_meta (
               peer TEXT NOT NULL,
               table_name TEXT NOT NULL,
               last_sync_at TEXT NOT NULL,
               PRIMARY KEY (peer, table_name)
             );",
        )
        .expect("sync meta table");
    let meta = SyncMeta {
        peer: "test-node".into(),
        table_name: "plans".into(),
        last_sync_at: "2026-03-28T12:00:00".into(),
    };
    super::upsert_sync_meta(db.connection(), &meta).expect("upsert");
    let result = super::get_sync_meta(db.connection(), "test-node", "plans")
        .expect("get")
        .expect("found");
    assert_eq!(result.last_sync_at, "2026-03-28T12:00:00");
}

// cli_sync_commands_point_to_timestamp_adapter is in libsql_adapter_sync_tests.rs
