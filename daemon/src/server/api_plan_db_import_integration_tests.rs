// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// HTTP integration tests for plan import endpoint (/api/plan-db/import).

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
    tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0,
    waves_total INTEGER DEFAULT 0, updated_at TEXT,
    source_file TEXT, description TEXT
);
CREATE TABLE IF NOT EXISTS waves (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_id INTEGER, project_id TEXT, wave_id TEXT,
    name TEXT, status TEXT DEFAULT 'pending',
    position INTEGER DEFAULT 0, depends_on TEXT,
    estimated_hours INTEGER DEFAULT 8,
    tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0
);
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_id INTEGER, project_id TEXT,
    wave_id_fk INTEGER, wave_id TEXT, task_id TEXT,
    title TEXT, status TEXT DEFAULT 'pending',
    priority TEXT, type TEXT, description TEXT,
    test_criteria TEXT, model TEXT, assignee TEXT,
    output_type TEXT, validator_agent TEXT,
    effort_level INTEGER DEFAULT 1, notes TEXT
);
CREATE TABLE IF NOT EXISTS plan_reviews (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plan_id INTEGER, spec_file TEXT,
    reviewer_agent TEXT NOT NULL, verdict TEXT NOT NULL,
    reviewed_at TEXT DEFAULT (datetime('now'))
);
INSERT INTO projects (id, name) VALUES ('convergio', 'Convergio');
INSERT INTO plans (id, project_id, name, status) VALUES (1, 'convergio', 'Import Plan', 'draft');
INSERT INTO plans (id, project_id, name, status) VALUES (2, 'convergio', 'Doing Plan', 'doing');
";

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-import-int-{}-{n}.db",
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

// --- POST /api/plan-db/import ---

#[tokio::test]
async fn import_json_waves_success() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/plan-db/import",
        serde_json::json!({
            "plan_id": 1,
            "waves": [
                {
                    "id": "W1",
                    "name": "Foundation",
                    "tasks": [
                        {"id": "T1-01", "title": "Set up CI pipeline", "priority": "P0", "type": "config"},
                        {"id": "T1-02", "title": "Add linting rules", "priority": "P1", "type": "chore"}
                    ]
                },
                {
                    "id": "W2",
                    "name": "Core Features",
                    "depends_on": "W1",
                    "tasks": [
                        {"id": "T2-01", "title": "Implement auth middleware", "priority": "P0", "type": "feature"}
                    ]
                }
            ]
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["plan_id"], 1);
    assert_eq!(j["waves_created"], 2);
    assert_eq!(j["tasks_created"], 3);
}

#[tokio::test]
async fn import_yaml_spec_success() {
    let r = test_router();
    let yaml = "waves:\n  - id: W1\n    name: Wave 1\n    tasks:\n      - id: T1\n        title: Implement user registration\n        type: feature\n        priority: P0\n";
    let (s, j) = post_json(
        &r,
        "/api/plan-db/import",
        serde_json::json!({"plan_id": 1, "spec": yaml}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["waves_created"], 1);
    assert_eq!(j["tasks_created"], 1);
}

#[tokio::test]
async fn import_missing_plan_id_returns_400() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/plan-db/import",
        serde_json::json!({"waves": []}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn import_doing_plan_blocked_by_guard() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/plan-db/import",
        serde_json::json!({
            "plan_id": 2,
            "waves": [{"id": "W1", "name": "Wave", "tasks": [{"id": "T1", "title": "Task"}]}]
        }),
    )
    .await;
    // Plan 2 is 'doing' — import should be blocked by require_plan_importable
    assert_eq!(s, StatusCode::CONFLICT);
}

#[tokio::test]
async fn import_nonexistent_plan_blocked_by_guard() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/plan-db/import",
        serde_json::json!({
            "plan_id": 9999,
            "waves": [{"id": "W1", "name": "Wave", "tasks": [{"id": "T1", "title": "Task"}]}]
        }),
    )
    .await;
    // require_plan_importable → require_plan_exists → not_found or bad_request
    assert!(
        s == StatusCode::BAD_REQUEST || s == StatusCode::NOT_FOUND || s == StatusCode::CONFLICT,
        "nonexistent plan should be rejected, got {s}"
    );
}

#[tokio::test]
async fn import_empty_waves_rejected() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/plan-db/import",
        serde_json::json!({"plan_id": 1, "waves": []}),
    )
    .await;
    // parse_waves rejects empty waves array
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn import_missing_waves_key_returns_400() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/plan-db/import",
        serde_json::json!({"plan_id": 1}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}
