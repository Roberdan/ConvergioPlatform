use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let db = std::env::temp_dir().join(format!("org-telemetry-test-{}-{n}.db", std::process::id()));
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), db, None)
}

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 131072).await.expect("body");
    serde_json::from_slice(&bytes).expect("json")
}

async fn create_org(app: &axum::Router, id: &str) {
    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"id": id, "mission": "m", "objectives": "o", "ceo_agent": "ceo", "budget": 10.0}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create org");
}

#[tokio::test]
async fn record_and_aggregate_telemetry() {
    let app = test_router();
    create_org(&app, "org-telemetry").await;
    let rec = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/orgs/org-telemetry/telemetry")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"agent":"a1","tokens_in":100,"tokens_out":40,"cost":1.25,"period":"day"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("record");
    assert_eq!(rec.status(), StatusCode::CREATED);
    let agg = app
        .oneshot(
            Request::builder()
                .uri("/api/orgs/org-telemetry/telemetry?period=day")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("aggregate");
    assert_eq!(agg.status(), StatusCode::OK);
    let v = body_json(agg.into_body()).await;
    assert_eq!(v["aggregate"]["tokens_in"], 100);
    assert_eq!(v["aggregate"]["tokens_out"], 40);
}

#[tokio::test]
async fn telemetry_agents_breakdown() {
    let app = test_router();
    create_org(&app, "org-breakdown").await;
    for (agent, in_t, out_t) in [("a1", 20, 10), ("a2", 15, 30)] {
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/orgs/org-breakdown/telemetry")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"agent":agent,"tokens_in":in_t,"tokens_out":out_t,"cost":0.5,"period":"day"}).to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("record");
    }
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/orgs/org-breakdown/telemetry/agents?period=day")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("agents");
    assert_eq!(resp.status(), StatusCode::OK);
    let v = body_json(resp.into_body()).await;
    let agents = v["agents"].as_array().expect("agents");
    assert_eq!(agents.len(), 2);
}
