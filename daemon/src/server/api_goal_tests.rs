// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for goal decomposer API (F-T4-03).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "cvg-goal-test-{}-{n}.db",
        std::process::id()
    ));
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

async fn post_json(router: &axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

#[tokio::test]
async fn goal_decompose_saas() {
    let r = test_router();
    let (s, j) = post_json(&r, "/api/goal/decompose", json!({"goal": "Launch recipe SaaS"})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["domains"].is_array());
    assert_eq!(j["waves"].as_array().map(|w| w.len()).unwrap_or(0), 5);
    assert!(j["estimated_tasks"].as_u64().unwrap_or(0) > 0);
    let agents = j["agents"].as_array().unwrap();
    assert!(!agents.is_empty());
}

#[tokio::test]
async fn goal_decompose_missing_goal() {
    let r = test_router();
    let (s, _) = post_json(&r, "/api/goal/decompose", json!({})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn goal_decompose_empty_goal() {
    let r = test_router();
    let (s, _) = post_json(&r, "/api/goal/decompose", json!({"goal": ""})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn goal_decompose_domain_agent_present() {
    let r = test_router();
    let (s, j) = post_json(&r, "/api/goal/decompose", json!({"goal": "Launch recipe SaaS"})).await;
    assert_eq!(s, StatusCode::OK);
    let domains = j["domains"].as_array().unwrap();
    let has_tech = domains.iter().any(|d| d["domain"] == "technology");
    let has_baccio = domains.iter().any(|d| d["agent"] == "baccio-tech-architect");
    assert!(has_tech, "technology domain expected for SaaS goal");
    assert!(has_baccio, "baccio-tech-architect expected");
}

#[tokio::test]
async fn goal_execute_stub() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/goal/execute",
        json!({"goal_id": "goal-launch-recipe-saas"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    let plan_id = j["plan_id"].as_str().unwrap_or("");
    assert!(plan_id.starts_with("plan-"), "plan_id must start with 'plan-'");
    assert_eq!(j["status"], "queued");
}

#[tokio::test]
async fn goal_execute_missing_goal_id() {
    let r = test_router();
    let (s, _) = post_json(&r, "/api/goal/execute", json!({})).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}
