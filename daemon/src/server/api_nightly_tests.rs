// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for nightly jobs API — list, detail, create, trigger.
// Toggle and config tests → api_nightly_tests2.rs.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

pub(super) const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS nightly_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT, job_name TEXT DEFAULT 'guardian',
    started_at TEXT DEFAULT (datetime('now')),
    finished_at TEXT, host TEXT, status TEXT DEFAULT 'running',
    sentry_unresolved INTEGER DEFAULT 0,
    github_open_issues INTEGER DEFAULT 0,
    processed_items INTEGER DEFAULT 0, fixed_items INTEGER DEFAULT 0,
    branch_name TEXT, pr_url TEXT, summary TEXT, report_json TEXT,
    duration_sec INTEGER, trigger_source TEXT DEFAULT 'scheduled',
    exit_code INTEGER, error_detail TEXT, log_file_path TEXT,
    parent_run_id TEXT, log_stdout TEXT, log_stderr TEXT,
    config_snapshot TEXT
);
CREATE TABLE IF NOT EXISTS nightly_job_definitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL, description TEXT, schedule TEXT DEFAULT '0 3 * * *',
    script_path TEXT, target_host TEXT DEFAULT 'local',
    enabled INTEGER DEFAULT 1, created_at TEXT DEFAULT (datetime('now')),
    project_id TEXT DEFAULT 'mirrorbuddy',
    run_fixes INTEGER DEFAULT 0, timeout_sec INTEGER DEFAULT 3600
);
CREATE TABLE IF NOT EXISTS mesh_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT, plan_id INTEGER, source_peer TEXT,
    payload TEXT, status TEXT DEFAULT 'pending',
    created_at TEXT DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS plans (
    id INTEGER PRIMARY KEY, name TEXT, status TEXT, project_id TEXT
);
CREATE TABLE IF NOT EXISTS daemon_config (key TEXT PRIMARY KEY, value TEXT);
";

pub(super) const SEED: &str = "
INSERT INTO nightly_job_definitions (name, description, schedule, script_path, project_id)
    VALUES ('mirrorbuddy-guardian', 'MirrorBuddy nightly check', '0 1 * * *',
            'scripts/nightly/mirrorbuddy-guardian.sh', 'mirrorbuddy');
INSERT INTO nightly_jobs (run_id, job_name, status, host, summary, trigger_source)
    VALUES ('run-001', 'mirrorbuddy-guardian', 'completed', 'm5max',
            'All checks passed', 'scheduled');
INSERT INTO mesh_events (event_type, plan_id, source_peer, payload, status)
    VALUES ('plan_started', 742, 'm5max', '{\"plan_id\": 742}', 'pending');
";

pub(super) fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-nightly-test-{}-{n}.db",
        std::process::id()
    ));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    conn.execute_batch(SEED).expect("seed");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

pub(super) async fn get(router: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

pub(super) async fn post_json(router: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
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

// --- GET /api/nightly/jobs ---

#[tokio::test]
async fn nightly_jobs_list() {
    let r = test_router();
    let (s, j) = get(&r, "/api/nightly/jobs").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["history"].is_array());
    assert!(j["definitions"].is_array());
    assert!(j["total"].is_number());
}

#[tokio::test]
async fn nightly_jobs_list_pagination() {
    let r = test_router();
    let (s, j) = get(&r, "/api/nightly/jobs?page=1&per_page=10").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["page"], 1);
    assert_eq!(j["per_page"], 10);
}

// --- GET /api/nightly/jobs/:id ---

#[tokio::test]
async fn nightly_job_detail() {
    let r = test_router();
    let (s, j) = get(&r, "/api/nightly/jobs/1").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["run_id"], "run-001");
    assert_eq!(j["status"], "completed");
}

#[tokio::test]
async fn nightly_job_detail_not_found() {
    let r = test_router();
    let (s, _) = get(&r, "/api/nightly/jobs/9999").await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

// --- POST /api/nightly/jobs/create ---

#[tokio::test]
async fn nightly_job_create_success() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/nightly/jobs/create",
        serde_json::json!({
            "name": "virtualbpm-guardian",
            "script_path": "scripts/nightly/virtualbpm-guardian.sh",
            "description": "VirtualBPM nightly health check",
            "schedule": "0 2 * * *",
            "project_id": "virtualbpm"
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["name"], "virtualbpm-guardian");
}

#[tokio::test]
async fn nightly_job_create_empty_name_returns_400() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/nightly/jobs/create",
        serde_json::json!({"name": "", "script_path": "x.sh"}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn nightly_job_create_empty_script_returns_400() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/nightly/jobs/create",
        serde_json::json!({"name": "job-1", "script_path": "  "}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

// --- POST /api/nightly/jobs/trigger ---

#[tokio::test]
async fn nightly_job_trigger_default_project() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/nightly/jobs/trigger",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["triggered"], true);
    assert_eq!(j["project_id"], "mirrorbuddy");
}

#[tokio::test]
async fn nightly_job_trigger_custom_project() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/nightly/jobs/trigger",
        serde_json::json!({"project_id": "convergio"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["project_id"], "convergio");
}
