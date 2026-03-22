// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Cross-feature integration tests: end-to-end flows spanning multiple API domains.

use super::api_cross_feature_helpers::{get_json, post_json, setup_state};
use super::state::ServerState;
use axum::http::StatusCode;
use axum::Router;
use serde_json::json;

fn full_app(state: ServerState) -> Router {
    use super::api_agent_catalog::router as catalog_router;
    use super::api_agent_triage::router as triage_router;
    use super::api_audit::router as audit_router;
    use super::api_deliverables::router as deliv_router;
    catalog_router()
        .merge(triage_router())
        .merge(audit_router())
        .merge(deliv_router())
        .with_state(state)
}

/// E2E: create deliverable -> approve -> audit sees it
#[tokio::test]
async fn e2e_deliverable_approve_audit() {
    let (state, _tmp) = setup_state("integ-proj", "Integration Project");
    let app = full_app(state);

    // 1. Create deliverable
    let (status, resp) = post_json(
        &app,
        "/api/deliverables",
        json!({
            "project_id": "integ-proj",
            "name": "Design Doc",
            "output_type": "document"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create deliverable: {resp}");
    let deliv_id = resp["id"].as_i64().expect("deliverable id");
    assert!(resp["output_path"].as_str().is_some());

    // 2. Approve it
    let (status, resp) = post_json(
        &app,
        &format!("/api/deliverables/{deliv_id}/approve"),
        json!({"approved_by": "reviewer-bot"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "approve: {resp}");
    assert_eq!(resp["status"], "approved");
    assert_eq!(resp["approved_by"], "reviewer-bot");

    // 3. Audit shows the deliverable
    let (status, resp) = get_json(&app, "/api/audit/project/integ-proj").await;
    assert_eq!(status, StatusCode::OK, "audit: {resp}");
    let deliverables = resp["deliverables"].as_array().expect("deliverables array");
    assert_eq!(deliverables.len(), 1);
    assert_eq!(deliverables[0]["status"], "approved");
    assert_eq!(deliverables[0]["name"], "Design Doc");
}

/// E2E: create agent in catalog -> enable -> triage finds it by domain
#[tokio::test]
async fn e2e_skill_enable_triage_finds_agent() {
    let (state, tmp) = setup_state("integ-proj", "Integration Project");
    let app = full_app(state);

    // 1. Create agent in catalog
    let (status, _) = post_json(
        &app,
        "/api/agents/create",
        json!({
            "name": "security-scanner",
            "category": "security",
            "description": "Scans code for security vulnerabilities",
            "model": "claude-sonnet-4-6"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 2. Enable it to a target dir
    let target_dir = tmp.path().join("agents-enabled");
    std::fs::create_dir_all(&target_dir).unwrap();
    let (status, resp) = post_json(
        &app,
        "/api/agents/enable",
        json!({
            "name": "security-scanner",
            "target_dir": target_dir.to_string_lossy()
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "enable: {resp}");
    assert_eq!(resp["enabled"], "security-scanner");
    assert!(target_dir.join("security-scanner.agent.md").exists());

    // 3. Triage with a security problem — exact domain match = 1.0
    let (status, resp) = post_json(
        &app,
        "/api/agents/triage",
        json!({
            "problem_description": "Need to scan for security vulnerabilities",
            "domain": "security"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "triage: {resp}");
    let suggestions = resp["suggestions"].as_array().expect("suggestions");
    assert!(!suggestions.is_empty());
    assert_eq!(suggestions[0]["name"], "security-scanner");
    assert_eq!(suggestions[0]["score"], 1.0);
    assert_eq!(resp["suggest_creation"], false);
}

/// Permission preflight: empty project_id -> 400
#[tokio::test]
async fn permission_preflight_empty_project_returns_error() {
    let (state, _tmp) = setup_state("integ-proj", "Integration Project");
    let app = full_app(state);
    let (status, _) = post_json(
        &app,
        "/api/deliverables",
        json!({"project_id": "", "name": "Bad", "output_type": "document"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// Permission preflight: empty name -> 400
#[tokio::test]
async fn permission_preflight_empty_name_returns_error() {
    let (state, _tmp) = setup_state("integ-proj", "Integration Project");
    let app = full_app(state);
    let (status, _) = post_json(
        &app,
        "/api/deliverables",
        json!({"project_id": "integ-proj", "name": "", "output_type": "document"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// Permission preflight: approve without reviewer -> 400
#[tokio::test]
async fn permission_preflight_approve_without_reviewer_returns_error() {
    let (state, _tmp) = setup_state("integ-proj", "Integration Project");
    let app = full_app(state);

    let (_, resp) = post_json(
        &app,
        "/api/deliverables",
        json!({"project_id": "integ-proj", "name": "Output", "output_type": "document"}),
    )
    .await;
    let deliv_id = resp["id"].as_i64().unwrap();

    let (status, _) = post_json(
        &app,
        &format!("/api/deliverables/{deliv_id}/approve"),
        json!({"approved_by": ""}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// Permission preflight: empty triage problem -> 400
#[tokio::test]
async fn permission_preflight_triage_empty_problem_returns_error() {
    let (state, _tmp) = setup_state("integ-proj", "Integration Project");
    let app = full_app(state);
    let (status, _) = post_json(
        &app,
        "/api/agents/triage",
        json!({"problem_description": ""}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
