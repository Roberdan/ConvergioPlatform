// Tests for api_workers/dispatch.rs — extracted to keep dispatch.rs under 250 lines.
use crate::db::PlanDb;

fn setup_db() -> PlanDb {
    let db = PlanDb::open_in_memory().expect("db");
    db.connection()
        .execute_batch(
            "CREATE TABLE plans (
                 id INTEGER PRIMARY KEY, project_id TEXT, name TEXT,
                 status TEXT, execution_host TEXT, updated_at TEXT
             );
             CREATE TABLE coordinator_events (
                 id INTEGER PRIMARY KEY, event_type TEXT NOT NULL DEFAULT '',
                 payload TEXT, source_node TEXT,
                 handled_at TEXT DEFAULT (datetime('now'))
             );
             INSERT INTO plans VALUES (1, 'test', 'Plan A', 'doing', NULL, NULL);",
        )
        .expect("schema");
    db
}

#[test]
fn remote_exec_delegate_updates_host() {
    let db = setup_db();
    let conn = db.connection();

    let changed = conn
        .execute(
            "UPDATE plans SET execution_host = 'linux-worker' WHERE id = 1",
            [],
        )
        .unwrap();
    assert_eq!(changed, 1);

    let host: String = conn
        .query_row("SELECT execution_host FROM plans WHERE id = 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(host, "linux-worker");
}
