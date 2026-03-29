// LWW (last-write-wins) and HTTP plan replication tests.
// Split from api_sync_replication_tests.rs to stay under 250 lines.
use rusqlite::Connection;

use crate::db::libsql_adapter::{apply_changes, export_changes_since};

fn setup_sync_db() -> (Connection, std::path::PathBuf) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(90_000);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "claude-repl-lww-{}-{n}.db",
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

/// Proves: status update on node-A replicates to node-B (LWW newer wins).
#[test]
fn plan_status_update_replicates_via_lww() {
    let (conn_a, path_a) = setup_sync_db();
    let (conn_b, path_b) = setup_sync_db();

    let old_ts = "2026-03-28T10:00:00";
    conn_a
        .execute(
            "INSERT INTO plans (id, project_id, name, status, \
             created_at, updated_at) \
             VALUES (100, 'proj-4', 'LWW Plan', 'draft', ?1, ?1)",
            rusqlite::params![old_ts],
        )
        .expect("seed a");
    conn_b
        .execute(
            "INSERT INTO plans (id, project_id, name, status, \
             created_at, updated_at) \
             VALUES (100, 'proj-4', 'LWW Plan', 'draft', ?1, ?1)",
            rusqlite::params![old_ts],
        )
        .expect("seed b");

    let new_ts = "2026-03-29T15:00:00";
    conn_a
        .execute(
            "UPDATE plans SET status = 'doing', updated_at = ?1 \
             WHERE id = 100",
            rusqlite::params![new_ts],
        )
        .expect("update a");

    let changes = export_changes_since(&conn_a, "plans", Some(old_ts))
        .expect("export delta");
    assert!(!changes.is_empty(), "delta must contain the update");

    let applied = apply_changes(&conn_b, &changes).expect("apply");
    assert_eq!(applied, 1, "newer update must be applied");

    let status: String = conn_b
        .query_row(
            "SELECT status FROM plans WHERE id = 100",
            [],
            |r| r.get(0),
        )
        .expect("query status on b");
    assert_eq!(
        status, "doing",
        "node-B must reflect node-A's newer status"
    );

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}

/// Proves: stale update from node-A is rejected by node-B (LWW).
#[test]
fn stale_plan_update_rejected_by_lww() {
    let (conn_a, path_a) = setup_sync_db();
    let (conn_b, path_b) = setup_sync_db();

    conn_b
        .execute(
            "INSERT INTO plans (id, project_id, name, status, \
             created_at, updated_at) \
             VALUES (200, 'proj-5', 'LWW Reject', 'doing', \
             '2026-03-29T12:00:00', '2026-03-29T12:00:00')",
            [],
        )
        .expect("seed b newer");

    conn_a
        .execute(
            "INSERT INTO plans (id, project_id, name, status, \
             created_at, updated_at) \
             VALUES (200, 'proj-5', 'LWW Reject', 'draft', \
             '2026-03-28T10:00:00', '2026-03-28T10:00:00')",
            [],
        )
        .expect("seed a older");

    let changes =
        export_changes_since(&conn_a, "plans", None).expect("export stale");
    let applied = apply_changes(&conn_b, &changes).expect("apply stale");
    assert_eq!(applied, 0, "stale update must be rejected by LWW");

    let status: String = conn_b
        .query_row(
            "SELECT status FROM plans WHERE id = 200",
            [],
            |r| r.get(0),
        )
        .expect("query");
    assert_eq!(status, "doing", "node-B must keep its newer status");

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}

/// Proves: HTTP sync endpoints export plans correctly (integration).
#[tokio::test]
async fn http_sync_export_returns_plans() {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use tower::ServiceExt;

    let (_, path) = setup_sync_db();
    let conn = Connection::open(&path).expect("open");
    conn.execute(
        "INSERT INTO plans (project_id, name, status, \
         created_at, updated_at) \
         VALUES ('proj-6', 'HTTP Export Plan', 'draft', \
         datetime('now'), datetime('now'))",
        [],
    )
    .expect("seed plan");
    drop(conn);

    super::super::middleware::set_dev_mode(true);
    let router = super::super::routes::build_router_with_db(
        std::path::PathBuf::from("/tmp"),
        path.clone(),
        None,
    );

    let resp = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/sync/export?table=plans")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), 4_194_304)
        .await
        .unwrap();
    let json: serde_json::Value =
        serde_json::from_slice(&body).expect("json");
    let count = json["count"].as_u64().unwrap_or(0);
    assert!(count >= 1, "HTTP export must return the seeded plan");

    let _ = std::fs::remove_file(&path);
}
