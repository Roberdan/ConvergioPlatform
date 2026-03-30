// Two-node plan replication test via sync export/import path.
// Why: proves plan created on node-A replicates to node-B within SLA.
use rusqlite::Connection;
use std::time::Instant;

use crate::db::libsql_adapter::{apply_changes, export_changes_since};

fn setup_sync_db() -> (Connection, std::path::PathBuf) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(80_000);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "claude-repl-sync-{}-{n}.db",
        std::process::id()
    ));
    let conn = Connection::open(&path).expect("open db");
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA busy_timeout=5000;
         CREATE TABLE IF NOT EXISTS plans (
             id INTEGER PRIMARY KEY,
             project_id TEXT NOT NULL DEFAULT 'test',
             name TEXT NOT NULL,
             status TEXT NOT NULL DEFAULT 'draft',
             description TEXT,
             created_at TEXT DEFAULT (datetime('now')),
             updated_at TEXT DEFAULT (datetime('now'))
         );
         CREATE TABLE IF NOT EXISTS _sync_meta (
             peer TEXT NOT NULL,
             table_name TEXT NOT NULL,
             last_sync_at TEXT NOT NULL,
             PRIMARY KEY (peer, table_name)
         );",
    )
    .expect("schema");
    (conn, path)
}

/// Proves: plan created on node-A replicates to node-B via export+import.
/// Full cycle must complete under 60s SLA.
#[test]
fn plan_replicates_via_sync_export_import() {
    let (conn_a, path_a) = setup_sync_db();
    let (conn_b, path_b) = setup_sync_db();
    let start = Instant::now();

    // 1. Create plan on node-A
    conn_a
        .execute(
            "INSERT INTO plans (project_id, name, status, description, \
             created_at, updated_at) \
             VALUES ('proj-1', 'Sync SLA Test Plan', 'draft', \
             'Replication proof', datetime('now'), datetime('now'))",
            [],
        )
        .expect("insert plan on node-a");
    let plan_id: i64 = conn_a
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .unwrap();

    // 2. Export from node-A
    let changes =
        export_changes_since(&conn_a, "plans", None).expect("export");
    assert!(
        !changes.is_empty(),
        "node-A must export the created plan"
    );
    let found = changes.iter().any(|c| c.pk == plan_id);
    assert!(found, "exported changes must include plan {plan_id}");

    // 3. Import into node-B
    let applied = apply_changes(&conn_b, &changes).expect("apply");
    assert!(applied >= 1, "at least one change must apply on node-B");

    // 4. Verify plan exists on node-B
    let name: String = conn_b
        .query_row(
            "SELECT name FROM plans WHERE id = ?1",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .expect("plan must exist on node-B after sync");
    assert_eq!(name, "Sync SLA Test Plan");

    let status: String = conn_b
        .query_row(
            "SELECT status FROM plans WHERE id = ?1",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .expect("status");
    assert_eq!(status, "draft");

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 60,
        "replication must complete within 60s SLA, took {:?}",
        elapsed
    );

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}

/// Proves: sync is idempotent — importing the same plan twice is a no-op.
#[test]
fn plan_replication_idempotent() {
    let (conn_a, path_a) = setup_sync_db();
    let (conn_b, path_b) = setup_sync_db();

    conn_a
        .execute(
            "INSERT INTO plans (project_id, name, status, \
             created_at, updated_at) \
             VALUES ('proj-2', 'Idempotent Plan', 'draft', \
             datetime('now'), datetime('now'))",
            [],
        )
        .expect("insert");

    let changes =
        export_changes_since(&conn_a, "plans", None).expect("export");

    let first = apply_changes(&conn_b, &changes).expect("first apply");
    assert!(first >= 1, "first import must apply");

    let second = apply_changes(&conn_b, &changes).expect("second apply");
    assert_eq!(second, 0, "duplicate import must be no-op (LWW skip)");

    let count: i64 = conn_b
        .query_row("SELECT COUNT(*) FROM plans", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1, "must have exactly one plan after double import");

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}

/// Proves: local replication cycle completes sub-second — no bottleneck.
#[test]
fn plan_replication_sub_second() {
    let (conn_a, path_a) = setup_sync_db();
    let (conn_b, path_b) = setup_sync_db();
    let start = Instant::now();

    conn_a
        .execute(
            "INSERT INTO plans (project_id, name, status, \
             created_at, updated_at) \
             VALUES ('proj-3', 'Speed Test Plan', 'draft', \
             datetime('now'), datetime('now'))",
            [],
        )
        .expect("insert");

    let changes =
        export_changes_since(&conn_a, "plans", None).expect("export");
    apply_changes(&conn_b, &changes).expect("apply");

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 1000,
        "local replication must be sub-second, took {}ms",
        elapsed.as_millis()
    );

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}
