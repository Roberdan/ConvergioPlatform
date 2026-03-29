use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS _sync_meta (
    peer TEXT NOT NULL,
    table_name TEXT NOT NULL,
    last_sync_at TEXT NOT NULL,
    PRIMARY KEY (peer, table_name)
);
CREATE TABLE IF NOT EXISTS plans (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT DEFAULT 'todo',
    project_id TEXT
);
CREATE TABLE IF NOT EXISTS daemon_config (
    key TEXT PRIMARY KEY,
    value TEXT
);
CREATE TABLE IF NOT EXISTS agent_activity (
    id INTEGER PRIMARY KEY,
    agent_id TEXT,
    activity TEXT,
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS host_heartbeats (
    id INTEGER PRIMARY KEY,
    host_id TEXT,
    ts TEXT DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS peer_heartbeats (
    id INTEGER PRIMARY KEY,
    peer_id TEXT,
    ts TEXT DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS mesh_sync_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    peer_id TEXT NOT NULL,
    last_sync_at TEXT,
    changes_sent INTEGER DEFAULT 0,
    changes_received INTEGER DEFAULT 0,
    status TEXT DEFAULT 'idle'
);
";

fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir()
        .join(format!("claude-sync-test-{}-{n}.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open tmp db");
    conn.execute_batch(SCHEMA).expect("schema");
    drop(conn);
    super::super::middleware::set_dev_mode(true);
    super::super::routes::build_router_with_db(
        std::path::PathBuf::from("/tmp"),
        tmp,
        None,
    )
}

fn test_router_seeded() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(200);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir()
        .join(format!("claude-sync-test-{}-{n}.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open tmp db");
    conn.execute_batch(SCHEMA).expect("schema");
    conn.execute_batch(
        "INSERT INTO tasks(id, title, status, updated_at) VALUES
         (1, 'Implement sync API', 'done', '2026-03-27T09:00:00'),
         (2, 'Wire background loop', 'pending', '2026-03-28T12:00:00');",
    )
    .expect("seed");
    drop(conn);
    super::super::middleware::set_dev_mode(true);
    super::super::routes::build_router_with_db(
        std::path::PathBuf::from("/tmp"),
        tmp,
        None,
    )
}

async fn get_json(router: axum::Router, path: &str) -> (StatusCode, Value) {
    let resp = router
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_048_576)
        .await
        .unwrap();
    let json: Value =
        serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}

async fn post_json_body(
    router: axum::Router,
    path: &str,
    body: Value,
) -> (StatusCode, Value) {
    let resp = router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(path)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_048_576)
        .await
        .unwrap();
    let json: Value =
        serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn export_returns_empty_for_empty_table() {
    let router = test_router();
    let (status, body) =
        get_json(router, "/api/sync/export?table=tasks").await;
    assert_eq!(status, StatusCode::OK, "expected 200: {body}");
    let changes = body["changes"].as_array().unwrap();
    assert!(changes.is_empty(), "empty table must yield empty changes");
}

#[tokio::test]
async fn export_returns_all_when_no_since() {
    let router = test_router_seeded();
    let (status, body) =
        get_json(router, "/api/sync/export?table=tasks").await;
    assert_eq!(status, StatusCode::OK, "expected 200: {body}");
    let changes = body["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 2, "should return all rows");
}

#[tokio::test]
async fn export_filters_by_since() {
    let router = test_router_seeded();
    let (status, body) = get_json(
        router,
        "/api/sync/export?table=tasks&since=2026-03-28T00:00:00",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "expected 200: {body}");
    let changes = body["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 1, "should return only newer rows");
    assert_eq!(changes[0]["pk"], 2);
}

#[tokio::test]
async fn export_rejects_missing_table_param() {
    let router = test_router();
    let (status, _body) =
        get_json(router, "/api/sync/export").await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "missing table should 400"
    );
}

#[tokio::test]
async fn import_applies_changes() {
    let router = test_router();
    let payload = serde_json::json!({
        "changes": [
            {
                "table_name": "tasks",
                "pk": 10,
                "data": {
                    "title": "Imported Task",
                    "status": "pending",
                    "updated_at": "2026-03-28T14:00:00"
                }
            }
        ]
    });
    let (status, body) =
        post_json_body(router, "/api/sync/import", payload).await;
    assert_eq!(status, StatusCode::OK, "expected 200: {body}");
    assert_eq!(body["applied"], 1);
}

#[tokio::test]
async fn import_rejects_empty_changes() {
    let router = test_router();
    let payload = serde_json::json!({ "changes": [] });
    let (status, body) =
        post_json_body(router, "/api/sync/import", payload).await;
    assert_eq!(status, StatusCode::OK, "expected 200: {body}");
    assert_eq!(body["applied"], 0);
}
