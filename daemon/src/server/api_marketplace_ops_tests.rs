use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let db = std::env::temp_dir().join(format!("mktops-test-{}-{n}.db", std::process::id()));
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), db, None)
}

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 131072).await.expect("body bytes");
    serde_json::from_slice(&bytes).expect("json body")
}

async fn setup_org_and_service(app: &axum::Router) {
    // Create requester org with budget
    let _ = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/orgs")
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "id": "org-requester",
                "mission": "Buy services",
                "objectives": "Integration",
                "ceo_agent": "alice",
                "budget": 500.0
            }).to_string())).unwrap(),
    ).await.unwrap();

    // Create provider org
    let _ = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/orgs")
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "id": "org-provider",
                "mission": "Provide services",
                "objectives": "Revenue",
                "ceo_agent": "bob",
                "budget": 100.0
            }).to_string())).unwrap(),
    ).await.unwrap();

    // Register service with cost in metadata
    let _ = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/orgs/org-provider/services")
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "name": "code-review",
                "endpoint": "/api/review",
                "metadata": { "cost": 25.0 }
            }).to_string())).unwrap(),
    ).await.unwrap();
}

#[tokio::test]
async fn create_service_request_deducts_budget() {
    let app = test_router();
    setup_org_and_service(&app).await;

    let resp = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/services/request")
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "requester_org": "org-requester",
                "service_name": "code-review",
                "request_payload": "PR #42"
            }).to_string())).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["provider_org"], "org-provider");
    assert_eq!(body["cost"], 25.0);
    assert!(body["request_id"].as_str().unwrap().starts_with("svcreq-"));

    // Verify budget was deducted (500 - 25 = 475)
    let resp = app.clone().oneshot(
        Request::builder().method("GET").uri("/api/orgs/org-requester")
            .body(Body::empty()).unwrap(),
    ).await.unwrap();
    let body = body_json(resp.into_body()).await;
    let budget: f64 = body["org"]["budget"].as_f64().unwrap();
    assert!((budget - 475.0).abs() < 0.01, "budget should be 475, got {budget}");
}

#[tokio::test]
async fn create_request_unknown_service_returns_404() {
    let app = test_router();
    setup_org_and_service(&app).await;

    let resp = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/services/request")
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "requester_org": "org-requester",
                "service_name": "nonexistent-service"
            }).to_string())).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_request_status_full_flow() {
    let app = test_router();
    setup_org_and_service(&app).await;

    // Create request
    let resp = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/services/request")
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "requester_org": "org-requester",
                "service_name": "code-review"
            }).to_string())).unwrap(),
    ).await.unwrap();
    let body = body_json(resp.into_body()).await;
    let req_id = body["request_id"].as_str().unwrap().to_string();

    // Update to in_progress
    let resp = app.clone().oneshot(
        Request::builder().method("PUT")
            .uri(format!("/api/services/requests/{req_id}"))
            .header("content-type", "application/json")
            .body(Body::from(json!({"status": "in_progress"}).to_string())).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Update to completed
    let resp = app.clone().oneshot(
        Request::builder().method("PUT")
            .uri(format!("/api/services/requests/{req_id}"))
            .header("content-type", "application/json")
            .body(Body::from(json!({"status": "completed"}).to_string())).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp.into_body()).await;
    assert_eq!(body["status"], "completed");
}

#[tokio::test]
async fn update_request_invalid_status_returns_400() {
    let app = test_router();
    setup_org_and_service(&app).await;

    let resp = app.clone().oneshot(
        Request::builder().method("POST").uri("/api/services/request")
            .header("content-type", "application/json")
            .body(Body::from(json!({
                "requester_org": "org-requester",
                "service_name": "code-review"
            }).to_string())).unwrap(),
    ).await.unwrap();
    let body = body_json(resp.into_body()).await;
    let req_id = body["request_id"].as_str().unwrap().to_string();

    let resp = app.clone().oneshot(
        Request::builder().method("PUT")
            .uri(format!("/api/services/requests/{req_id}"))
            .header("content-type", "application/json")
            .body(Body::from(json!({"status": "cancelled"}).to_string())).unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
