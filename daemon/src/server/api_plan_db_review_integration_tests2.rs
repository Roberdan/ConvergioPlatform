// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// HTTP integration tests for plan review endpoints — reset and link-by-spec.
// Register and check tests are in api_plan_db_review_integration_tests.rs.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS plans (
    id INTEGER PRIMARY KEY, project_id TEXT, name TEXT, status TEXT
);
CREATE TABLE IF NOT EXISTS plan_reviews (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_id INTEGER, spec_file TEXT,
    reviewer_agent TEXT NOT NULL, verdict TEXT NOT NULL,
    suggestions TEXT, raw_report TEXT,
    reviewed_at TEXT DEFAULT (datetime('now'))
);
INSERT INTO projects (id, name) VALUES ('convergio', 'Convergio');
INSERT INTO plans (id, project_id, name, status) VALUES (1, 'convergio', 'Plan Alpha', 'draft');
INSERT INTO plans (id, project_id, name, status) VALUES (2, 'convergio', 'Plan Beta', 'doing');
";

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-review-int2-{}-{n}.db",
        std::process::id()
    ));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
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

// --- POST /api/plan-db/review/reset ---

#[tokio::test]
async fn review_reset_deletes_reviews() {
    let r = test_router();
    post_json(
        &r,
        "/api/plan-db/review/register",
        serde_json::json!({
            "plan_id": 1,
            "reviewer_agent": "plan-reviewer",
            "verdict": "reject"
        }),
    )
    .await;

    let (s, j) = post_json(
        &r,
        "/api/plan-db/review/reset",
        serde_json::json!({"plan_id": 1}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["deleted"].as_i64().unwrap() >= 1);

    // Verify empty after reset
    let (_, j) = get(&r, "/api/plan-db/review/check?plan_id=1").await;
    assert_eq!(j["total"], 0);
}

#[tokio::test]
async fn review_reset_missing_plan_id() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/plan-db/review/reset",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

// --- POST /api/plan-db/review/link-by-spec ---

#[tokio::test]
async fn review_link_by_spec() {
    let r = test_router();
    // Register review by spec_file (no plan_id)
    post_json(
        &r,
        "/api/plan-db/review/register",
        serde_json::json!({
            "spec_file": "/workspace/plans/plan-750.yaml",
            "reviewer_agent": "plan-reviewer",
            "verdict": "proceed"
        }),
    )
    .await;

    // Link it to plan 1
    let (s, j) = post_json(
        &r,
        "/api/plan-db/review/link-by-spec",
        serde_json::json!({
            "plan_id": 1,
            "spec_file": "/workspace/plans/plan-750.yaml"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["linked"], 1);

    // Verify it appears in plan 1 check
    let (_, j) = get(&r, "/api/plan-db/review/check?plan_id=1").await;
    assert_eq!(j["total"], 1);
}

#[tokio::test]
async fn review_link_by_spec_missing_fields() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/plan-db/review/link-by-spec",
        serde_json::json!({"plan_id": 1}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}
