//! Integration tests for GET /api/health/deep.
//!
//! TDD: these tests drive the health/deep endpoint implementation.

use crate::server::routes::GET_ROUTES;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn build_test_app() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp =
        std::env::temp_dir().join(format!("claude-hd-test-{}-{n}.db", std::process::id()));
    // dev-mode disables auth middleware so tests can hit the endpoint
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

#[tokio::test]
async fn health_deep_returns_200() {
    let app = build_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health/deep")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn health_deep_body_contains_component_fields() {
    let app = build_test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health/deep")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("request failed");
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("valid json");

    // Must have overall status
    assert!(json.get("status").is_some(), "missing 'status' field");

    // Must have per-component breakdown
    let components = json.get("components").expect("missing 'components' field");
    assert!(components.is_array(), "'components' must be an array");

    let names: Vec<&str> = components
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
        .collect();

    assert!(names.contains(&"database"), "missing 'database' component");
    assert!(
        names.contains(&"filesystem"),
        "missing 'filesystem' component"
    );
    assert!(
        names.contains(&"ipc_engine"),
        "missing 'ipc_engine' component"
    );
    assert!(names.contains(&"swarm"), "missing 'swarm' component");
}

#[test]
fn get_routes_includes_health_deep() {
    assert!(
        GET_ROUTES.contains(&"/api/health/deep"),
        "/api/health/deep must appear in GET_ROUTES"
    );
}
