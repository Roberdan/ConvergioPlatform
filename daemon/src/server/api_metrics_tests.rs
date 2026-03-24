// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for metrics API endpoints.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp =
        std::env::temp_dir().join(format!("claude-metrics-test-{}-{n}.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    conn.execute_batch(SEED_DATA).expect("seed data");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS plans (
  id INTEGER PRIMARY KEY, name TEXT, status TEXT, project_id TEXT
);
CREATE TABLE IF NOT EXISTS execution_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  goal TEXT NOT NULL,
  status TEXT DEFAULT 'running',
  cost_usd REAL DEFAULT 0,
  agents_used INTEGER DEFAULT 0,
  plan_id INTEGER,
  started_at TEXT DEFAULT (datetime('now')),
  completed_at TEXT
);
CREATE TABLE IF NOT EXISTS delegation_log (
  id INTEGER PRIMARY KEY,
  task_db_id INTEGER,
  plan_id INTEGER,
  project_id TEXT,
  model TEXT,
  cost_estimate REAL,
  created_at DATETIME DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS tasks (
  id INTEGER PRIMARY KEY,
  plan_id INTEGER,
  status TEXT DEFAULT 'pending',
  executor_agent TEXT
);
";

const SEED_DATA: &str = "
INSERT INTO plans(id, name, status, project_id)
  VALUES(1, 'Plan Alpha', 'doing', 'proj-1');
INSERT INTO execution_runs(id, goal, status, plan_id, cost_usd, agents_used, started_at)
  VALUES(1, 'Deploy feature X', 'completed', 1, 1.25, 3,
         datetime('now', '-30 minutes'));
INSERT INTO execution_runs(id, goal, status, plan_id, started_at)
  VALUES(2, 'Run tests', 'running', 1, datetime('now', '-5 minutes'));
INSERT INTO delegation_log(id, plan_id, model, cost_estimate, created_at)
  VALUES(1, 1, 'claude-sonnet', 0.50, datetime('now', '-20 minutes'));
INSERT INTO delegation_log(id, plan_id, model, cost_estimate, created_at)
  VALUES(2, 1, 'claude-haiku', 0.10, datetime('now', '-15 minutes'));
INSERT INTO delegation_log(id, plan_id, model, cost_estimate, created_at)
  VALUES(3, 1, 'claude-sonnet', 0.30, datetime('now', '-10 minutes'));
INSERT INTO tasks(id, plan_id, status, executor_agent)
  VALUES(10, 1, 'done', 'agent-a');
INSERT INTO tasks(id, plan_id, status, executor_agent)
  VALUES(11, 1, 'done', 'agent-b');
INSERT INTO tasks(id, plan_id, status, executor_agent)
  VALUES(12, 1, 'pending', NULL);
";

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 65536).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// --- GET /api/metrics/run/:id -----------------------------------------------

#[tokio::test]
async fn run_metrics_returns_response_shape() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/metrics/run/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert!(json["run"].is_object());
    assert!(json["duration_secs"].is_number());
    assert!(json["cost_usd"].is_number());
    assert!(json["agents_used"].is_number());
    assert!(json["tasks_done"].is_number());
    assert!(json["tasks_total"].is_number());
}

#[tokio::test]
async fn run_metrics_cost_from_delegation_log() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/metrics/run/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    // delegation_log has 0.50 + 0.10 + 0.30 = 0.90 for plan_id=1
    let cost = json["cost_usd"].as_f64().unwrap();
    assert!((cost - 0.90).abs() < 0.01, "expected ~0.90, got {cost}");
}

#[tokio::test]
async fn run_metrics_agents_used_count() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/metrics/run/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    // 2 distinct executor_agents (agent-a, agent-b)
    assert_eq!(json["agents_used"], 2);
}

#[tokio::test]
async fn run_metrics_task_counts() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/metrics/run/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["tasks_done"], 2);
    assert_eq!(json["tasks_total"], 3);
}

#[tokio::test]
async fn run_metrics_not_found_returns_bad_request() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/metrics/run/9999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// --- GET /api/metrics/summary -----------------------------------------------

#[tokio::test]
async fn summary_response_shape() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/metrics/summary")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["run_count"], 2);
    assert!(json["status_distribution"].is_array());
    assert!(json["top_agents"].is_array());
}

// --- GET /api/metrics/cost --------------------------------------------------

#[tokio::test]
async fn cost_breakdown_response_shape() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/metrics/cost")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["days"], 7); // default
    assert!(json["by_model"].is_array());
    assert!(json["by_project"].is_array());
    assert!(json["by_date"].is_array());
}

#[tokio::test]
async fn cost_breakdown_with_days_param() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/metrics/cost?days=30")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["days"], 30);
}
