// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for evolution engine API endpoints (api_evolution.rs).

use super::api_evolution::router;
use super::state::ServerState;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

fn test_state() -> (ServerState, TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let state = ServerState::new(db_path, None);
    (state, tmp)
}

fn app(state: ServerState) -> Router {
    router().with_state(state)
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn seed_proposal(state: &ServerState) -> i64 {
    let conn = state.get_conn().unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS evolution_proposals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            hypothesis TEXT NOT NULL,
            target_metric TEXT NOT NULL,
            expected_delta REAL DEFAULT 0,
            blast_radius TEXT DEFAULT 'SingleRepo',
            status TEXT NOT NULL DEFAULT 'pending'
                CHECK(status IN ('pending','approved','rejected','running','completed','rolled_back')),
            reviewer TEXT, reviewed_at TEXT, review_reason TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        )",
    )
    .unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS evolution_experiments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            proposal_id INTEGER NOT NULL REFERENCES evolution_proposals(id),
            mode TEXT NOT NULL DEFAULT 'canary',
            before_metrics TEXT, after_metrics TEXT,
            result TEXT DEFAULT 'pending'
                CHECK(result IN ('pending','success','failure','rolled_back')),
            started_at TEXT DEFAULT (datetime('now')), completed_at TEXT
        )",
    )
    .unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS evolution_audit (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            proposal_id INTEGER NOT NULL REFERENCES evolution_proposals(id),
            action TEXT NOT NULL, actor TEXT, reason TEXT,
            created_at TEXT DEFAULT (datetime('now'))
        )",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO evolution_proposals (hypothesis, target_metric, expected_delta) \
         VALUES ('test hypothesis', 'latency', 0.15)",
        [],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[tokio::test]
async fn list_proposals_empty() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .uri("/api/evolution/proposals")
        .body(Body::empty())
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["proposals"], json!([]));
}

#[tokio::test]
async fn approve_proposal_success() {
    let (state, _tmp) = test_state();
    let id = seed_proposal(&state);
    let router = app(state);
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/evolution/proposals/{id}/approve"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"reason": "looks good", "actor": "tester"}).to_string(),
        ))
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["status"], "approved");
    assert_eq!(json["id"], id);
}

#[tokio::test]
async fn approve_nonexistent_proposal_returns_bad_request() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .method("POST")
        .uri("/api/evolution/proposals/9999/approve")
        .header("content-type", "application/json")
        .body(Body::from(json!({}).to_string()))
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn reject_proposal_success() {
    let (state, _tmp) = test_state();
    let id = seed_proposal(&state);
    let router = app(state);
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/evolution/proposals/{id}/reject"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"reason": "not viable", "actor": "reviewer"}).to_string(),
        ))
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["status"], "rejected");
}

#[tokio::test]
async fn double_approve_returns_bad_request() {
    let (state, _tmp) = test_state();
    let id = seed_proposal(&state);
    let router = app(state);
    // First approve succeeds
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/evolution/proposals/{id}/approve"))
        .header("content-type", "application/json")
        .body(Body::from(json!({}).to_string()))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Second approve fails — proposal is no longer pending
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/evolution/proposals/{id}/approve"))
        .header("content-type", "application/json")
        .body(Body::from(json!({}).to_string()))
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn list_experiments_empty() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .uri("/api/evolution/experiments")
        .body(Body::empty())
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["experiments"], json!([]));
}

#[tokio::test]
async fn roi_empty_db() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .uri("/api/evolution/roi")
        .body(Body::empty())
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["experimentsRun"], 0);
    assert_eq!(json["successes"], 0);
    assert_eq!(json["rollbacks"], 0);
    assert_eq!(json["successRate"], 0.0);
    assert!(json["proposalsByStatus"].is_array());
}

#[tokio::test]
async fn audit_trail_empty() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .uri("/api/evolution/audit/1")
        .body(Body::empty())
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["audit"], json!([]));
    assert_eq!(json["proposal_id"], 1);
}

#[tokio::test]
async fn approve_creates_audit_entry() {
    let (state, _tmp) = test_state();
    let id = seed_proposal(&state);
    let router = app(state);
    // Approve the proposal
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/evolution/proposals/{id}/approve"))
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"reason": "audit test", "actor": "auditor"}).to_string(),
        ))
        .unwrap();
    router.clone().oneshot(req).await.unwrap();

    // Check audit trail
    let req = Request::builder()
        .uri(&format!("/api/evolution/audit/{id}"))
        .body(Body::empty())
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    let json = body_json(resp).await;
    let entries = json["audit"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["action"], "approve");
    assert_eq!(entries[0]["actor"], "auditor");
    assert_eq!(entries[0]["reason"], "audit test");
}
