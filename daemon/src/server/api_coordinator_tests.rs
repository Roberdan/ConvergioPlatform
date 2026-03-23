// Integration tests for coordinator API endpoints.
// Pattern: build_router_with_db with temp DB, send requests via oneshot.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS coordinator_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL DEFAULT '',
    payload TEXT,
    source_node TEXT,
    handled_at TEXT DEFAULT (datetime('now'))
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
        .join(format!("claude-coord-test-{}-{n}.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open tmp db");
    conn.execute_batch(SCHEMA).expect("schema");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(
        std::path::PathBuf::from("/tmp"),
        tmp,
        None,
    )
}

fn test_router_seeded() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(100);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir()
        .join(format!("claude-coord-test-{}-{n}.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open tmp db");
    conn.execute_batch(SCHEMA).expect("schema");
    conn.execute_batch(
        "INSERT INTO coordinator_events (event_type, payload, source_node) \
         VALUES ('plan_started', '{\"plan_id\": 1}', 'mac-worker');
         INSERT INTO coordinator_events (event_type, payload, source_node) \
         VALUES ('task_done', '{\"task_id\": 42}', 'linux-worker');",
    )
    .expect("seed");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(
        std::path::PathBuf::from("/tmp"),
        tmp,
        None,
    )
}

async fn get(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn post_json(
    router: &axum::Router,
    uri: &str,
    payload: Value,
) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(uri)
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&payload).unwrap()))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

async fn post_empty(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(uri)
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn coordinator_events_empty() {
    let r = test_router();
    let (s, j) = get(&r, "/api/coordinator/events").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    let events = j["events"].as_array().expect("events array");
    assert!(events.is_empty(), "fresh DB should have no events");
    assert_eq!(j["count"], 0);
}

#[tokio::test]
async fn coordinator_events_seeded() {
    let r = test_router_seeded();
    let (s, j) = get(&r, "/api/coordinator/events").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    let events = j["events"].as_array().expect("events array");
    assert_eq!(events.len(), 2);
    assert_eq!(j["count"], 2);
}

#[tokio::test]
async fn coordinator_emit_event() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/coordinator/emit",
        serde_json::json!({
            "event_type": "plan_started",
            "payload": {"plan_id": 42},
            "source_node": "test-node"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["event_id"].is_number(), "should return event_id");
    assert_eq!(j["event_type"], "plan_started");

    // Verify event appears in list
    let (_, j) = get(&r, "/api/coordinator/events").await;
    let events = j["events"].as_array().expect("events");
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn coordinator_emit_missing_event_type() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/coordinator/emit",
        serde_json::json!({"payload": {}}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn coordinator_emit_defaults_source_node() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/coordinator/emit",
        serde_json::json!({"event_type": "agent_started"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["event_type"], "agent_started");
}

#[tokio::test]
async fn coordinator_process_events() {
    let r = test_router_seeded();
    let (s, j) = post_empty(&r, "/api/coordinator/process").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["events_found"].is_number());
    assert!(j["processed"].is_number());
    assert!(j["actions"].is_array());
}

#[tokio::test]
async fn coordinator_process_empty() {
    let r = test_router();
    let (s, j) = post_empty(&r, "/api/coordinator/process").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["events_found"], 0);
    assert_eq!(j["processed"], 0);
    let actions = j["actions"].as_array().expect("actions");
    assert!(actions.is_empty());
}
