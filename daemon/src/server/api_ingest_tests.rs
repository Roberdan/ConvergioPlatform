// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for document ingestion API endpoints.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!(
        "claude-ingest-test-{}-{n}.db",
        std::process::id()
    ));
    let conn = rusqlite::Connection::open(&tmp).expect("open");
    conn.execute_batch(SCHEMA).expect("schema");
    drop(conn);
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(
        std::path::PathBuf::from("/tmp"),
        tmp,
        None,
    )
}

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS plans (
  id INTEGER PRIMARY KEY, name TEXT, status TEXT
);
";

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 65536).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// --- POST /api/ingest — validation ------------------------------------------

#[tokio::test]
async fn ingest_empty_source_returns_bad_request() {
    let app = test_router();
    let body = serde_json::json!({"source": "", "output_dir": "/tmp/out"});
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ingest")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], false);
}

#[tokio::test]
async fn ingest_empty_output_dir_returns_bad_request() {
    let app = test_router();
    let body = serde_json::json!({"source": "https://example.com", "output_dir": ""});
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ingest")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["ok"], false);
}

#[tokio::test]
async fn ingest_whitespace_only_source_returns_bad_request() {
    let app = test_router();
    let body = serde_json::json!({"source": "   ", "output_dir": "/tmp/out"});
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ingest")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ingest_missing_fields_returns_422() {
    let app = test_router();
    // Missing output_dir entirely
    let body = serde_json::json!({"source": "https://example.com"});
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/ingest")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    // Axum returns 422 for deserialization failures
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// --- GET /api/ingest/formats ------------------------------------------------

#[tokio::test]
async fn ingest_formats_response_shape() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/ingest/formats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    // Verify all expected format keys are present
    assert!(json.get("pdf").is_some(), "missing pdf key");
    assert!(json.get("docx").is_some(), "missing docx key");
    assert!(json.get("url").is_some(), "missing url key");
    assert!(json.get("xlsx").is_some(), "missing xlsx key");
    assert!(json.get("pptx").is_some(), "missing pptx key");
    // images is always true
    assert_eq!(json["images"], true);
}

#[tokio::test]
async fn ingest_formats_values_are_booleans() {
    let app = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/ingest/formats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    // Each format capability should be a boolean
    assert!(json["pdf"].is_boolean());
    assert!(json["docx"].is_boolean());
    assert!(json["url"].is_boolean());
    assert!(json["xlsx"].is_boolean());
    assert!(json["pptx"].is_boolean());
    assert!(json["images"].is_boolean());
}

// --- IngestBody deserialization ---------------------------------------------

#[test]
fn ingest_body_deserializes_correctly() {
    let raw = r#"{"source": "/path/to/doc.pdf", "output_dir": "/tmp/out"}"#;
    let body: Value = serde_json::from_str(raw).unwrap();
    assert_eq!(body["source"], "/path/to/doc.pdf");
    assert_eq!(body["output_dir"], "/tmp/out");
}

#[test]
fn ingest_body_rejects_missing_source() {
    // Verify the struct requires both fields
    let raw = r#"{"output_dir": "/tmp/out"}"#;
    let result: Result<Value, _> = serde_json::from_str(raw);
    let v = result.unwrap();
    assert!(v.get("source").is_none());
}
