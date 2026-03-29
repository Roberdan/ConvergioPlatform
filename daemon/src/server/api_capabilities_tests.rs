// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for capabilities API (list, register, invoke, schema, permissions).

use super::api_capabilities::router as cap_router;
use super::state::ServerState;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

fn test_app() -> (Router, TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let state = ServerState::new(db_path, None);
    let app = cap_router().with_state(state);
    (app, tmp)
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

fn post_req(uri: &str, payload: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap()
}

fn put_req(uri: &str, payload: Value) -> Request<Body> {
    Request::builder()
        .method("PUT")
        .uri(uri)
        .header("Content-Type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap()
}

fn get_req(uri: &str) -> Request<Body> {
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

// --- POST /api/capabilities/invoke ---

#[tokio::test]
async fn invoke_nonexistent_capability_returns_404() {
    let (app, _tmp) = test_app();
    let resp = app
        .oneshot(post_req(
            "/api/capabilities/invoke",
            json!({
                "name": "does-not-exist",
                "input": {},
                "agent_id": "test-agent"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn invoke_registered_capability_succeeds() {
    let (app, _tmp) = test_app();
    // Register a ring-2 capability (no permissions required)
    app.clone()
        .oneshot(post_req(
            "/api/capabilities/register",
            json!({
                "name": "mesh-status",
                "description": "Query mesh topology",
                "ring": 2
            }),
        ))
        .await
        .unwrap();

    // Grant permission to agent
    app.clone()
        .oneshot(put_req(
            "/api/capabilities/permissions",
            json!({"agent_id": "executor-1", "grant": "mesh-status"}),
        ))
        .await
        .unwrap();

    // Invoke
    let resp = app
        .oneshot(post_req(
            "/api/capabilities/invoke",
            json!({
                "name": "mesh-status",
                "input": {"node": "m5max"},
                "agent_id": "executor-1"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["ok"], true);
    assert_eq!(j["capability"], "mesh-status");
}

// --- PUT /api/capabilities/permissions ---

#[tokio::test]
async fn grant_and_revoke_permissions() {
    let (app, _tmp) = test_app();
    // Grant
    let resp = app
        .clone()
        .oneshot(put_req(
            "/api/capabilities/permissions",
            json!({"agent_id": "ali-orchestrator", "grant": "plan-manager"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let j = body_json(resp).await;
    assert_eq!(j["ok"], true);
    assert_eq!(j["agent_id"], "ali-orchestrator");
    let tools = j["allowed_tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t == "plan-manager"));

    // Revoke
    let resp = app
        .oneshot(put_req(
            "/api/capabilities/permissions",
            json!({"agent_id": "ali-orchestrator", "revoke": "plan-manager"}),
        ))
        .await
        .unwrap();
    let j = body_json(resp).await;
    let tools = j["allowed_tools"].as_array().unwrap();
    assert!(!tools.iter().any(|t| t == "plan-manager"));
}

// --- GET /api/capabilities/list?ring=N ---

#[tokio::test]
async fn list_capabilities_with_ring_filter() {
    let (app, _tmp) = test_app();
    // Register ring 0 (with required permissions) and ring 2 capabilities
    app.clone()
        .oneshot(post_req(
            "/api/capabilities/register",
            json!({
                "name": "core-tool",
                "description": "Core system tool",
                "ring": 0,
                "permissions_required": ["system:admin", "system:read"]
            }),
        ))
        .await
        .unwrap();
    app.clone()
        .oneshot(post_req(
            "/api/capabilities/register",
            json!({"name": "community-tool", "description": "Community tool", "ring": 2}),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(get_req("/api/capabilities/list?ring=0"))
        .await
        .unwrap();
    let j = body_json(resp).await;
    assert_eq!(j["ok"], true);
    assert_eq!(j["count"], 1);
}
