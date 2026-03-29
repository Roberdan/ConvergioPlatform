use rusqlite::Connection;

use super::SyncChange;

fn setup_sync_schema(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE tasks (
           id INTEGER PRIMARY KEY,
           title TEXT NOT NULL,
           status TEXT NOT NULL DEFAULT 'pending',
           executor_status TEXT NOT NULL DEFAULT 'idle'
             CHECK(executor_status IN ('idle', 'running', 'paused', 'completed', 'failed')),
           validated_by TEXT,
           updated_at TEXT NOT NULL
         );
         CREATE TRIGGER enforce_thor_done
         BEFORE UPDATE OF status ON tasks
         WHEN NEW.status = 'done' AND OLD.status <> 'done'
         BEGIN
           SELECT RAISE(ABORT, 'BLOCKED: Only Thor can set status=done. validated_by must be thor/thor-quality-assurance-guardian/thor-per-wave/forced-admin.')
           WHERE OLD.status <> 'submitted'
             OR NEW.validated_by IS NULL
             OR NEW.validated_by NOT IN ('thor', 'thor-quality-assurance-guardian', 'thor-per-wave', 'forced-admin');
         END;",
    )
    .expect("sync schema");
}

#[test]
fn apply_changes_normalises_done_task_for_thor_guard() {
    let conn = Connection::open_in_memory().expect("db");
    setup_sync_schema(&conn);
    conn.execute_batch(
        "INSERT INTO tasks(id, title, status, validated_by, updated_at)
         VALUES (41, 'Existing task', 'pending', NULL, '2026-03-29T20:00:00Z');",
    )
    .expect("seed");

    let changes = vec![SyncChange {
        table_name: "tasks".to_string(),
        pk: 41,
        data: serde_json::json!({
            "title": "Existing task",
            "status": "done",
            "updated_at": "2026-03-29T20:01:00Z"
        }),
    }];

    let applied = super::apply_changes(&conn, &changes).expect("apply");
    assert_eq!(applied, 1);

    let (status, validated_by): (String, String) = conn
        .query_row(
            "SELECT status, validated_by FROM tasks WHERE id = 41",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("query");
    assert_eq!(status, "done");
    assert_eq!(validated_by, "forced-admin");
}

#[test]
fn apply_changes_continues_after_done_task_and_inserts_probe_task() {
    let conn = Connection::open_in_memory().expect("db");
    setup_sync_schema(&conn);
    conn.execute_batch(
        "INSERT INTO tasks(id, title, status, validated_by, updated_at)
         VALUES (41, 'Existing task', 'pending', NULL, '2026-03-29T20:00:00Z');",
    )
    .expect("seed");

    let changes = vec![
        SyncChange {
            table_name: "tasks".to_string(),
            pk: 41,
            data: serde_json::json!({
                "title": "Existing task",
                "status": "done",
                "updated_at": "2026-03-29T20:01:00Z"
            }),
        },
        SyncChange {
            table_name: "tasks".to_string(),
            pk: 55,
            data: serde_json::json!({
                "title": "Probe task",
                "status": "pending",
                "executor_status": null,
                "validated_by": null,
                "updated_at": "2026-03-29T20:02:00Z"
            }),
        },
    ];

    let applied = super::apply_changes(&conn, &changes).expect("apply");
    assert_eq!(applied, 2);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, 2);

    let executor_status: String = conn
        .query_row(
            "SELECT executor_status FROM tasks WHERE id = 55",
            [],
            |row| row.get(0),
        )
        .expect("probe task status");
    assert_eq!(executor_status, "idle");
}
