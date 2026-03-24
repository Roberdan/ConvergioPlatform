// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for domain→skill mapping API endpoints (api_domain.rs).

use super::api_domain::router;
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
async fn list_domains_empty() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .uri("/api/domain/list")
        .body(Body::empty())
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    // domain_skill_map is seeded by migrations, so we expect items array
    assert!(json["items"].is_array());
}

#[tokio::test]
async fn map_domain_success() {
    let (state, _tmp) = test_state();
    let router = app(state);
    let req = Request::builder()
        .method("POST")
        .uri("/api/domain/map")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "domain": "testing",
                "skill_name": "unit-test-skill",
                "description": "A test domain mapping"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["domain"], "testing");
    assert_eq!(json["skill_name"], "unit-test-skill");

    // Verify it appears in the list
    let req = Request::builder()
        .uri("/api/domain/list")
        .body(Body::empty())
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    let json = body_json(resp).await;
    let items = json["items"].as_array().unwrap();
    let found = items
        .iter()
        .any(|r| r["domain"] == "testing" && r["skill_name"] == "unit-test-skill");
    assert!(found, "newly mapped domain should appear in list");
}

#[tokio::test]
async fn map_domain_duplicate_returns_conflict() {
    let (state, _tmp) = test_state();
    let router = app(state);
    let body = json!({
        "domain": "dup-domain",
        "skill_name": "dup-skill"
    })
    .to_string();

    let req = Request::builder()
        .method("POST")
        .uri("/api/domain/map")
        .header("content-type", "application/json")
        .body(Body::from(body.clone()))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Second insert with same domain+skill_name should conflict
    let req = Request::builder()
        .method("POST")
        .uri("/api/domain/map")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn map_domain_empty_domain_returns_bad_request() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .method("POST")
        .uri("/api/domain/map")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"domain": "", "skill_name": "some-skill"}).to_string(),
        ))
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn map_domain_empty_skill_name_returns_bad_request() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .method("POST")
        .uri("/api/domain/map")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"domain": "valid-domain", "skill_name": "  "}).to_string(),
        ))
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn map_domain_without_description() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .method("POST")
        .uri("/api/domain/map")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"domain": "nodesc", "skill_name": "nodesc-skill"}).to_string(),
        ))
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["domain"], "nodesc");
}

#[tokio::test]
async fn list_domains_returns_expected_fields() {
    let (state, _tmp) = test_state();
    let router = app(state);
    // Insert a mapping first
    let req = Request::builder()
        .method("POST")
        .uri("/api/domain/map")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"domain": "fields-test", "skill_name": "sk1", "description": "d1"}).to_string(),
        ))
        .unwrap();
    router.clone().oneshot(req).await.unwrap();

    let req = Request::builder()
        .uri("/api/domain/list")
        .body(Body::empty())
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    let json = body_json(resp).await;
    let items = json["items"].as_array().unwrap();
    let row = items
        .iter()
        .find(|r| r["domain"] == "fields-test")
        .expect("row must exist");
    // Verify all expected fields are present
    assert!(row.get("id").is_some());
    assert_eq!(row["domain"], "fields-test");
    assert_eq!(row["skill_name"], "sk1");
    assert_eq!(row["description"], "d1");
    assert!(row.get("created_at").is_some());
}
