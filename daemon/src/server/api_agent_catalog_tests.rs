// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for agent catalog API endpoints.

use super::api_agent_catalog::router;
use super::state::ServerState;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use std::path::PathBuf;
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

#[tokio::test]
async fn catalog_sync_scans_agent_files() {
    let (state, _tmp) = test_state();
    let router = app(state);

    let agent_dir = tempfile::tempdir().unwrap();
    let agent_content = "---\nname: synced-agent\ndescription: \"Synced from file\"\nmodel: claude-sonnet-4-6\ntools:\n  - view\n  - edit\n---\n\n# synced-agent\n";
    std::fs::write(
        agent_dir.path().join("synced-agent.agent.md"),
        agent_content,
    )
    .unwrap();
    // Non-agent file should be skipped
    std::fs::write(agent_dir.path().join("readme.md"), "# Readme").unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/sync")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"source_dir": agent_dir.path().to_string_lossy()}).to_string(),
        ))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["synced"], 1);
    assert_eq!(json["added"], 1);

    // Verify it's in the catalog
    let req = Request::builder()
        .uri("/api/agents/catalog")
        .body(Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["agents"][0]["name"], "synced-agent");

    // Re-sync should show synced=1, added=0
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/sync")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"source_dir": agent_dir.path().to_string_lossy()}).to_string(),
        ))
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["synced"], 1);
    assert_eq!(json["added"], 0);
}

#[tokio::test]
async fn catalog_sync_bad_dir_returns_error() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/sync")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"source_dir": "/nonexistent/path"}).to_string(),
        ))
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn catalog_enable_and_disable() {
    let (state, _tmp) = test_state();
    let router = app(state);

    // First create an agent in the catalog
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/create")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"name": "my-agent", "description": "My agent", "model": "claude-sonnet-4-6"})
                .to_string(),
        ))
        .unwrap();
    router.clone().oneshot(req).await.unwrap();

    // Enable it to a target dir
    let target_dir = tempfile::tempdir().unwrap();
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/enable")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"name": "my-agent", "target_dir": target_dir.path().to_string_lossy()})
                .to_string(),
        ))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["enabled"], "my-agent");

    // File should exist
    let agent_file = target_dir.path().join("my-agent.agent.md");
    assert!(agent_file.exists());
    let content = std::fs::read_to_string(&agent_file).unwrap();
    assert!(content.contains("name: my-agent"));

    // Disable it
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/disable")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"name": "my-agent", "target_dir": target_dir.path().to_string_lossy()})
                .to_string(),
        ))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["disabled"], "my-agent");

    // File should be gone
    assert!(!agent_file.exists());
}
