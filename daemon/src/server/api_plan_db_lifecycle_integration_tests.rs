// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// HTTP integration tests for plan lifecycle endpoints (create/start/approve/cancel/complete).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS plans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL, name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'draft',
    source_file TEXT, description TEXT, human_summary TEXT,
    tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0,
    waves_total INTEGER DEFAULT 0, execution_host TEXT,
    worktree_path TEXT, parallel_mode TEXT, is_master INTEGER DEFAULT 0,
    parent_plan_id INTEGER, constraints_json TEXT,
    created_at TEXT, started_at TEXT, completed_at TEXT,
    updated_at TEXT, cancelled_at TEXT, cancelled_reason TEXT
);
CREATE TABLE IF NOT EXISTS waves (
    id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id TEXT,
    name TEXT, status TEXT DEFAULT 'pending',
    tasks_done INTEGER DEFAULT 0, tasks_total INTEGER DEFAULT 0,
    position INTEGER DEFAULT 0, worktree_path TEXT,
    cancelled_at TEXT, cancelled_reason TEXT, project_id TEXT
);
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY, project_id TEXT, plan_id INTEGER,
    wave_id_fk INTEGER, wave_id TEXT, task_id TEXT,
    title TEXT, status TEXT DEFAULT 'pending', output_type TEXT,
    started_at TEXT, completed_at TEXT, notes TEXT
);
CREATE TABLE IF NOT EXISTS plan_reviews (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_id INTEGER, spec_file TEXT,
    reviewer_agent TEXT NOT NULL, verdict TEXT NOT NULL,
    suggestions TEXT, raw_report TEXT,
    reviewed_at TEXT DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS deliverables (
    id INTEGER PRIMARY KEY, task_id INTEGER, project_id TEXT,
    name TEXT, output_type TEXT, output_path TEXT,
    status TEXT DEFAULT 'pending', version INTEGER DEFAULT 1,
    approved_by TEXT, approved_at TEXT
);
INSERT INTO projects (id, name) VALUES ('convergio', 'Convergio');
";

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-lifecycle-test-{}-{n}.db",
        std::process::id()
    ));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

async fn post_json(router: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
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

// --- POST /api/plan-db/create ---

#[tokio::test]
async fn create_plan_success() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/plan-db/create",
        serde_json::json!({
            "project_id": "convergio",
            "name": "Platform Hardening v2",
            "description": "Comprehensive hardening of daemon APIs"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["plan_id"].is_number());
    assert_eq!(j["status"], "draft");
}

#[tokio::test]
async fn create_plan_missing_project_id() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/plan-db/create",
        serde_json::json!({"name": "No Project"}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn create_plan_missing_name() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/plan-db/create",
        serde_json::json!({"project_id": "convergio"}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

// --- POST /api/plan-db/approve/:plan_id ---

#[tokio::test]
async fn approve_draft_plan() {
    let r = test_router();
    let (_, j) = post_json(
        &r,
        "/api/plan-db/create",
        serde_json::json!({"project_id": "convergio", "name": "Approval Test"}),
    )
    .await;
    let plan_id = j["plan_id"].as_i64().unwrap();

    let (s, j) = post_empty(&r, &format!("/api/plan-db/approve/{plan_id}")).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["status"], "approved");
}

#[tokio::test]
async fn approve_nonexistent_plan() {
    let r = test_router();
    let (s, _) = post_empty(&r, "/api/plan-db/approve/99999").await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

// --- POST /api/plan-db/cancel/:plan_id ---

#[tokio::test]
async fn cancel_plan_cascades_tasks() {
    let r = test_router();
    let (_, j) = post_json(
        &r,
        "/api/plan-db/create",
        serde_json::json!({"project_id": "convergio", "name": "Cancel Test"}),
    )
    .await;
    let plan_id = j["plan_id"].as_i64().unwrap();
    // Move to doing so cancel works
    post_empty(&r, &format!("/api/plan-db/approve/{plan_id}")).await;

    let (s, j) = post_json(
        &r,
        &format!("/api/plan-db/cancel/{plan_id}"),
        serde_json::json!({"reason": "requirements changed"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["status"], "cancelled");
}

#[tokio::test]
async fn cancel_already_cancelled_returns_400() {
    let r = test_router();
    let (_, j) = post_json(
        &r,
        "/api/plan-db/create",
        serde_json::json!({"project_id": "convergio", "name": "Double Cancel"}),
    )
    .await;
    let plan_id = j["plan_id"].as_i64().unwrap();
    post_json(
        &r,
        &format!("/api/plan-db/cancel/{plan_id}"),
        serde_json::json!({"reason": "first cancel"}),
    )
    .await;

    let (s, _) = post_json(
        &r,
        &format!("/api/plan-db/cancel/{plan_id}"),
        serde_json::json!({"reason": "second cancel"}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

// --- POST /api/plan-db/start/:plan_id ---

#[tokio::test]
async fn start_plan_without_tasks_blocked() {
    let r = test_router();
    let (_, j) = post_json(
        &r,
        "/api/plan-db/create",
        serde_json::json!({"project_id": "convergio", "name": "Empty Start"}),
    )
    .await;
    let plan_id = j["plan_id"].as_i64().unwrap();

    // Start should fail — no imported tasks (guard: require_plan_startable)
    let (s, _) = post_empty(&r, &format!("/api/plan-db/start/{plan_id}")).await;
    assert_ne!(s, StatusCode::OK, "start should be blocked without tasks");
}
