use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let db = std::env::temp_dir().join(format!("orgs-test-{}-{n}.db", std::process::id()));
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), db, None)
}

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 131072).await.expect("body bytes");
    serde_json::from_slice(&bytes).expect("json body")
}

#[tokio::test]
async fn create_org_endpoint_creates_org() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "id": "org-alpha",
                        "mission": "Ship quality software",
                        "objectives": "Velocity, reliability",
                        "ceo_agent": "roberto",
                        "budget": 1200.0
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn add_member_endpoint_persists_member() {
    let app = test_router();
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "id": "org-beta",
                        "mission": "Operate platform",
                        "objectives": "SLOs",
                        "ceo_agent": "priya",
                        "budget": 800.0
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create");
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs/org-beta/members")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"agent":"alice","role":"engineer","department":"platform"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("member");
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn list_services_endpoint_returns_registered_service() {
    let app = test_router();
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "id": "org-gamma",
                        "mission": "Support customers",
                        "objectives": "Uptime",
                        "ceo_agent": "nina",
                        "budget": 1500.0
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create");
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs/org-gamma/services")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"name":"ticketing","endpoint":"https://svc.local/ticketing","status":"active"})
                        .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("service");
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/orgs/org-gamma/services")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("list");
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    let services = json["services"].as_array().expect("services array");
    assert_eq!(services.len(), 1);
    assert_eq!(services[0]["name"], "ticketing");
}

#[tokio::test]
async fn decisions_endpoint_logs_decision() {
    let app = test_router();
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "id": "org-delta",
                        "mission": "Decision quality",
                        "objectives": "Accuracy",
                        "ceo_agent": "maya",
                        "budget": 900.0
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create");
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs/org-delta/decisions")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "decision": "Use queue backpressure",
                        "rationale": "Avoid overload",
                        "made_by": "maya",
                        "refs": ["msg-1", "T3-02"]
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("log decision");
    assert_eq!(resp.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn decisions_endpoint_lists_paginated_newest_first() {
    let app = test_router();
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "id": "org-epsilon",
                        "mission": "Operate safely",
                        "objectives": "Reliability",
                        "ceo_agent": "kai",
                        "budget": 1100.0
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create");

    for decision in ["first", "second", "third"] {
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/orgs/org-epsilon/decisions")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "decision": decision,
                            "rationale": format!("{decision} rationale"),
                            "made_by": "kai",
                            "refs": ["msg-2"]
                        })
                        .to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("log decision");
    }

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/orgs/org-epsilon/decisions?limit=2&offset=1")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("list decisions");
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    let decisions = json["decisions"].as_array().expect("decisions array");
    assert_eq!(decisions.len(), 2);
    assert_eq!(decisions[0]["decision"], "second");
    assert_eq!(decisions[1]["decision"], "first");
}
