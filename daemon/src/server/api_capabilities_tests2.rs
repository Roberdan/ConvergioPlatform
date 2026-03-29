// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for capabilities API — invoke, permissions, ring filter.
// List and register tests → api_capabilities_tests.rs.

use axum::http::StatusCode;
use serde_json::json;
use tower::ServiceExt;

use super::api_capabilities_tests::{body_json, get_req, post_req, put_req, test_app};

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
