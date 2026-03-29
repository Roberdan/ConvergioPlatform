// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for capabilities API — list, register.
// Invoke, permissions, and ring-filter tests → api_capabilities_tests2.rs.

use super::api_capabilities::router as cap_router;
use super::state::ServerState;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

pub(super) fn test_app() -> (Router, TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let state = ServerState::new(db_path, None);
    let app = cap_router().with_state(state);
    (app, tmp)
}

pub(super) async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

pub(super) fn post_req(uri: &str, payload: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap()
}

pub(super) fn put_req(uri: &str, payload: Value) -> Request<Body> {
    Request::builder()
        .method("PUT")
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap()
}

pub(super) fn get_req(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

// --- GET /api/capabilities/list ---

#[tokio::test]
async fn list_capabilities_empty() {
    let (app, _tmp) = test_app();
    let resp = app.oneshot(get_req("/api/capabilities/list")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["ok"], true);
    assert_eq!(j["count"], 0);
    assert!(j["capabilities"].is_array());
}

// --- POST /api/capabilities/register ---

#[tokio::test]
async fn register_capability_success() {
    let (app, _tmp) = test_app();
    let resp = app
        .clone()
        .oneshot(post_req(
            "/api/capabilities/register",
            json!({
                "name": "plan-manager",
                "description": "Manage execution plans",
                "ring": 1,
                "mcp_server": "convergio-mcp",
                "input_schema": {"type": "object", "properties": {"plan_id": {"type": "integer"}}},
                "permissions_required": ["plans:read", "plans:write"]
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["ok"], true);

    // Verify it appears in list
    let resp = app
        .oneshot(get_req("/api/capabilities/list"))
        .await
        .unwrap();
    let j = body_json(resp).await;
    assert_eq!(j["count"], 1);
}

#[tokio::test]
async fn register_capability_security_gate_failure() {
    let (app, _tmp) = test_app();
    // Ring 0 without permissions_required should fail SecurityGate G-03
    let resp = app
        .oneshot(post_req(
            "/api/capabilities/register",
            json!({
                "name": "unprotected-core-tool",
                "description": "Missing required permissions",
                "ring": 0
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn register_capability_mcp_without_schema_fails() {
    let (app, _tmp) = test_app();
    // MCP tools must have input_schema (SecurityGate G-04)
    let resp = app
        .oneshot(post_req(
            "/api/capabilities/register",
            json!({
                "name": "mcp-no-schema",
                "description": "MCP tool without schema",
                "ring": 2,
                "mcp_server": "stdio://tool"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn register_capability_missing_name_returns_422() {
    let (app, _tmp) = test_app();
    let resp = app
        .oneshot(post_req(
            "/api/capabilities/register",
            json!({"description": "no name", "ring": 0}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
