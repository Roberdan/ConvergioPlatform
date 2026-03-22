//! Lifecycle tests for execution runs: pause, resume, metrics, ingest.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn test_router() -> axum::Router {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-runs-lc-test-{}-{n}.db",
        std::process::id()
    ));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(super::api_runs_tests::CORE_SCHEMA)
        .expect("core schema");
    conn.execute_batch(super::api_runs_tests::SEED_DATA)
        .expect("seed data");
    drop(conn);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), tmp, None)
}

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 65536).await.expect("body bytes");
    serde_json::from_slice(&bytes).expect("json body")
}

// --- POST /api/runs (validation) --------------------------------------------

#[tokio::test]
async fn create_run_missing_goal_returns_422() {
    let app = test_router();
    let body = serde_json::json!({"plan_id": 10});
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/runs")
                .header("x-agent-token", "test")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// --- PUT /api/runs/:id -------------------------------------------------------

#[tokio::test]
async fn update_run_status() {
    let app = test_router();
    let body = serde_json::json!({
        "status": "completed",
        "cost_usd": 1.5,
        "agents_used": 3,
        "completed_at": "2026-03-21T10:00:00"
    });
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/runs/1")
                .header("x-agent-token", "test")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["status"], "completed");
    assert!((json["cost_usd"].as_f64().unwrap() - 1.5).abs() < 0.01);
}

// --- POST /api/runs/:id/pause ------------------------------------------------

#[tokio::test]
async fn pause_run_sets_status_and_paused_at() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/runs/1/pause")
                .header("x-agent-token", "test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["status"], "paused");
    assert!(!json["paused_at"].is_null(), "paused_at should be set");
}

// --- POST /api/runs/:id/resume -----------------------------------------------

#[tokio::test]
async fn resume_run_sets_status_running_clears_paused_at() {
    let app = test_router();
    // First pause it
    app.clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/runs/1/pause")
                .header("x-agent-token", "test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Then resume
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/runs/1/resume")
                .header("x-agent-token", "test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["status"], "running");
    assert!(json["paused_at"].is_null(), "paused_at should be cleared");
}

// --- GET /api/metrics/summary ------------------------------------------------

#[tokio::test]
async fn metrics_summary_returns_valid_json() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/metrics/summary")
                .header("x-agent-token", "test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], true, "summary ok field must be true");
    assert!(
        json["run_count"].as_i64().is_some(),
        "run_count must be numeric"
    );
    assert!(
        json["total_cost_usd"].as_f64().is_some(),
        "total_cost_usd must be numeric"
    );
    assert!(
        json["status_distribution"].is_array(),
        "status_distribution must be array"
    );
    assert!(json["top_agents"].is_array(), "top_agents must be array");
    // Seed data has 2 runs — verify count
    assert_eq!(json["run_count"].as_i64().unwrap(), 2);
}

// --- GET /api/ingest/formats -------------------------------------------------

#[tokio::test]
async fn ingest_formats_returns_format_booleans() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/ingest/formats")
                .header("x-agent-token", "test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    // Each format key must be a boolean
    for key in ["pdf", "docx", "url", "xlsx", "pptx", "images"] {
        assert!(
            json[key].is_boolean(),
            "formats.{key} must be a boolean, got: {}",
            json[key]
        );
    }
    // images is always advertised as true regardless of tool availability
    assert_eq!(json["images"], true, "images must always be true");
}
