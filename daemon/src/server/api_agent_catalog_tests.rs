// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for agent catalog API endpoints.

use super::api_agent_catalog::router;
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

#[tokio::test]
async fn catalog_list_empty() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .uri("/api/agents/catalog")
        .body(Body::empty())
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["agents"], json!([]));
}

#[tokio::test]
async fn catalog_create_and_list() {
    let (state, _tmp) = test_state();
    let router = app(state);

    // Create an agent
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/create")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"name": "test-agent", "category": "technical", "description": "A test agent", "model": "claude-sonnet-4-6"})
                .to_string(),
        ))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["created"], "test-agent");

    // List should contain the agent
    let req = Request::builder()
        .uri("/api/agents/catalog")
        .body(Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["agents"].as_array().unwrap().len(), 1);
    assert_eq!(json["agents"][0]["name"], "test-agent");

    // Filter by category
    let req = Request::builder()
        .uri("/api/agents/catalog?category=technical")
        .body(Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["agents"].as_array().unwrap().len(), 1);

    // Filter by non-existing category
    let req = Request::builder()
        .uri("/api/agents/catalog?category=nonexistent")
        .body(Body::empty())
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["agents"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn catalog_create_duplicate_returns_conflict() {
    let (state, _tmp) = test_state();
    let router = app(state);

    let body = json!({"name": "dup-agent", "category": "test"}).to_string();
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/create")
        .header("content-type", "application/json")
        .body(Body::from(body.clone()))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/create")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn catalog_create_empty_name_returns_bad_request() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/create")
        .header("content-type", "application/json")
        .body(Body::from(json!({"name": ""}).to_string()))
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// BUG-1 security tests: path traversal via agent name in enable/disable

#[tokio::test]
async fn catalog_enable_rejects_path_traversal_name() {
    let (state, tmp) = test_state();
    let router = app(state);

    // Create a valid agent first so the DB lookup would normally succeed
    let _create = Request::builder()
        .method("POST")
        .uri("/api/agents/create")
        .header("content-type", "application/json")
        .body(Body::from(json!({"name": "safe-agent"}).to_string()))
        .unwrap();
    let _ = router.clone().oneshot(_create).await;

    // Attempt path traversal via name
    let evil_names = vec!["../evil", "../../root", "/etc/passwd", "foo/bar", "foo\0bar"];
    for evil in evil_names {
        let req = Request::builder()
            .method("POST")
            .uri("/api/agents/enable")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"name": evil, "target_dir": tmp.path().to_str().unwrap()}).to_string(),
            ))
            .unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "expected 400 for evil name: {evil}"
        );
    }
}

#[tokio::test]
async fn catalog_disable_rejects_path_traversal_name() {
    let (state, tmp) = test_state();
    let router = app(state);

    let evil_names = vec!["../evil", "/absolute/path", "a/b"];
    for evil in evil_names {
        let req = Request::builder()
            .method("POST")
            .uri("/api/agents/disable")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"name": evil, "target_dir": tmp.path().to_str().unwrap()}).to_string(),
            ))
            .unwrap();
        let resp = router.clone().oneshot(req).await.unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "expected 400 for evil name: {evil}"
        );
    }
}

#[tokio::test]
async fn catalog_enable_accepts_valid_name() {
    let (state, tmp) = test_state();
    let router = app(state);

    // Insert agent directly so enable can find it
    let create_req = Request::builder()
        .method("POST")
        .uri("/api/agents/create")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"name": "valid-agent", "description": "desc", "model": "claude-sonnet-4-6", "tools": "view"})
                .to_string(),
        ))
        .unwrap();
    let resp = router.clone().oneshot(create_req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/enable")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"name": "valid-agent", "target_dir": tmp.path().to_str().unwrap()}).to_string(),
        ))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Cleanup: disable should also work
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/disable")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"name": "valid-agent", "target_dir": tmp.path().to_str().unwrap()}).to_string(),
        ))
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[path = "api_agent_catalog_tests_sync.rs"]
mod sync_tests;
