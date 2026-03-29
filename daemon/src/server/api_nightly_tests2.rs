// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for nightly jobs API — toggle, config, and events.
// List, detail, create, and trigger tests → api_nightly_tests.rs.

use axum::http::StatusCode;
use serde_json;

use super::api_nightly_tests::{get, post_json, test_router};

// --- POST /api/nightly/jobs/definitions/:id/toggle ---

#[tokio::test]
async fn nightly_def_toggle() {
    let r = test_router();
    let (s, j) = post_json(
        &r,
        "/api/nightly/jobs/definitions/1/toggle",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["id"], 1);
    assert!(j["enabled"].is_boolean());
}

#[tokio::test]
async fn nightly_def_toggle_not_found() {
    let r = test_router();
    let (s, _) = post_json(
        &r,
        "/api/nightly/jobs/definitions/9999/toggle",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

// --- GET /api/nightly/config/:project_id ---

#[tokio::test]
async fn nightly_config_get() {
    let r = test_router();
    let (s, j) = get(&r, "/api/nightly/config/mirrorbuddy").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["project_id"], "mirrorbuddy");
    assert!(j["definitions"].is_array());
}

// --- GET /api/events ---

#[tokio::test]
async fn events_list() {
    let r = test_router();
    let (s, j) = get(&r, "/api/events").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j.is_array());
    let arr = j.as_array().unwrap();
    assert!(!arr.is_empty());
}
