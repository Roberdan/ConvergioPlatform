//! Integration tests for the execution runs API endpoints.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-runs-test-{}-{n}.db",
        std::process::id()
    ));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(CORE_SCHEMA).expect("core schema");
    conn.execute_batch(SEED_DATA).expect("seed data");
    drop(conn);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

pub(super) const CORE_SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS plans (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  status TEXT DEFAULT 'todo',
  project_id TEXT
);
CREATE TABLE IF NOT EXISTS execution_runs (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  goal             TEXT    NOT NULL,
  team             TEXT    DEFAULT '[]',
  status           TEXT    DEFAULT 'running'
                           CHECK(status IN (
                             'running','completed','failed','cancelled','paused'
                           )),
  result           TEXT,
  cost_usd         REAL    DEFAULT 0,
  agents_used      INTEGER DEFAULT 0,
  plan_id          INTEGER,
  started_at       TEXT    DEFAULT (datetime('now')),
  completed_at     TEXT,
  duration_minutes REAL,
  context_path     TEXT,
  paused_at        TEXT,
  paused_context   TEXT
);
CREATE TABLE IF NOT EXISTS delegation_log (
  id INTEGER PRIMARY KEY NOT NULL,
  task_db_id INTEGER,
  plan_id INTEGER,
  project_id TEXT,
  provider TEXT,
  model TEXT,
  prompt_tokens INTEGER,
  response_tokens INTEGER,
  duration_ms INTEGER,
  exit_code INTEGER,
  thor_result TEXT,
  cost_estimate REAL,
  privacy_level TEXT,
  created_at DATETIME DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY,
  plan_id INTEGER,
  status TEXT DEFAULT 'pending',
  executor_agent TEXT
);
";

pub(super) const SEED_DATA: &str = "
INSERT INTO plans(id,name,status) VALUES(10,'Deploy Alpha','doing');

INSERT INTO execution_runs(id,goal,status,plan_id,cost_usd,agents_used,started_at)
  VALUES(1,'Fix login bug','running',10,0.5,2,datetime('now','-10 minutes'));
INSERT INTO execution_runs(id,goal,status,plan_id)
  VALUES(2,'Add metrics endpoint','completed',NULL);

INSERT INTO delegation_log(id,task_db_id,plan_id,cost_estimate,created_at)
  VALUES(1,101,10,0.25,datetime('now','-5 minutes'));
INSERT INTO delegation_log(id,task_db_id,plan_id,cost_estimate,created_at)
  VALUES(2,102,10,0.15,datetime('now','-2 minutes'));
";

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 65536).await.expect("body bytes");
    serde_json::from_slice(&bytes).expect("json body")
}

// --- GET /api/runs -----------------------------------------------------------

#[tokio::test]
async fn list_runs_returns_array() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/runs")
                .header("x-agent-token", "test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert!(json.is_array(), "expected array");
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
}

#[tokio::test]
async fn list_runs_filter_by_status() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/runs?status=running")
                .header("x-agent-token", "test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["status"], "running");
}

#[tokio::test]
async fn list_runs_includes_plan_name() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/runs?status=running")
                .header("x-agent-token", "test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    let arr = json.as_array().unwrap();
    assert_eq!(arr[0]["plan_name"], "Deploy Alpha");
}

// --- GET /api/runs/:id -------------------------------------------------------

#[tokio::test]
async fn get_run_returns_single_run() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/runs/1")
                .header("x-agent-token", "test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["id"], 1);
    assert_eq!(json["goal"], "Fix login bug");
}

#[tokio::test]
async fn get_run_includes_cost_from_delegation_log() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/runs/1")
                .header("x-agent-token", "test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    // delegation_log has 0.25 + 0.15 = 0.40 for plan_id=10
    let delegation_cost = json["delegation_cost"].as_f64().unwrap_or(0.0);
    assert!(
        (delegation_cost - 0.40).abs() < 0.01,
        "expected ~0.40 got {delegation_cost}"
    );
}

#[tokio::test]
async fn get_run_not_found_returns_400() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/runs/9999")
                .header("x-agent-token", "test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// --- POST /api/runs ----------------------------------------------------------

#[tokio::test]
async fn create_run_returns_id() {
    let app = test_router();
    let body = serde_json::json!({"goal": "Migrate database schema"});
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/runs")
                .header("x-agent-token", "test")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert!(json["id"].as_i64().unwrap() > 0);
}


