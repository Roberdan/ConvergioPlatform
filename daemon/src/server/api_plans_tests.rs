//! Integration tests for /api/plan/* endpoints.
//! Covers: cancel, reset, move, validate.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp =
        std::env::temp_dir().join(format!("claude-plans-test-{}-{n}.db", std::process::id()));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(SEED).expect("seed");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

const SEED: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS plans (
    id INTEGER PRIMARY KEY, name TEXT, status TEXT DEFAULT 'todo',
    project_id TEXT, tasks_done INTEGER DEFAULT 0, tasks_total INTEGER DEFAULT 0,
    execution_host TEXT
);
CREATE TABLE IF NOT EXISTS waves (
    id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id TEXT,
    status TEXT DEFAULT 'pending', tasks_done INTEGER DEFAULT 0,
    tasks_total INTEGER DEFAULT 0, cancelled_at TEXT, completed_at TEXT,
    started_at TEXT
);
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id_fk INTEGER,
    status TEXT DEFAULT 'pending', executor_agent TEXT,
    executor_host TEXT, validated_at TEXT, validated_by TEXT,
    started_at TEXT, completed_at TEXT
);

INSERT INTO plans (id, name, status, tasks_total) VALUES (1, 'Alpha', 'doing', 3);
INSERT INTO waves (id, plan_id, wave_id, status, tasks_total) VALUES (10, 1, 'W1', 'in_progress', 3);
INSERT INTO tasks (id, plan_id, wave_id_fk, status) VALUES (100, 1, 10, 'submitted');
INSERT INTO tasks (id, plan_id, wave_id_fk, status) VALUES (101, 1, 10, 'submitted');
INSERT INTO tasks (id, plan_id, wave_id_fk, status) VALUES (102, 1, 10, 'done');
";

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 65536).await.expect("bytes");
    serde_json::from_slice(&bytes).expect("json")
}

// --- POST /api/plan/cancel ---

#[tokio::test]
async fn plan_cancel_sets_cancelled_status() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::post("/api/plan/cancel?plan_id=1")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["action"], "cancelled");
}

#[tokio::test]
async fn plan_cancel_missing_plan_id_returns_400() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::post("/api/plan/cancel")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn plan_cancel_invalid_plan_id_returns_400() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::post("/api/plan/cancel?plan_id=abc")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// --- POST /api/plan/reset ---

#[tokio::test]
async fn plan_reset_returns_ok() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::post("/api/plan/reset?plan_id=1")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["action"], "reset");
    assert_eq!(json["plan_id"], 1);
}

#[tokio::test]
async fn plan_reset_missing_plan_id_returns_400() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::post("/api/plan/reset")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// --- GET /api/plan/move ---

#[tokio::test]
async fn plan_move_returns_ok_with_target() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::get("/api/plan/move?plan_id=1&target=worker-3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["target"], "worker-3");
}

#[tokio::test]
async fn plan_move_missing_target_returns_400() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::get("/api/plan/move?plan_id=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn plan_move_empty_target_returns_400() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::get("/api/plan/move?plan_id=1&target=")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// --- POST /api/plans/:plan_id/validate ---

#[tokio::test]
async fn plan_validate_marks_submitted_as_done() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::post("/api/plans/1/validate")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["plan_id"], 1);
    // 2 submitted tasks should be validated
    assert_eq!(json["validated"], 2);
}

#[tokio::test]
async fn plan_validate_no_submitted_tasks_returns_zero() {
    let app = test_router();
    // Plan 99 doesn't exist — no tasks to validate but SQL runs fine
    let resp = app
        .oneshot(
            Request::post("/api/plans/99/validate")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["validated"], 0);
}
