// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Phase 2 cross-module integration tests (Plan 10042, WF-T1).
// Tests span orgs, orgchart, brain, nightly, metrics, and telemetry.
// Endpoints from unmerged W1-W4 worktrees use #[ignore] stubs.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tower::ServiceExt;

fn test_router() -> axum::Router {
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::SeqCst);
    let db = std::env::temp_dir().join(format!(
        "phase2-integ-{}-{n}.db",
        std::process::id()
    ));
    super::middleware::set_dev_mode(true);
    super::routes::build_router_with_db(std::path::PathBuf::from("/tmp"), db, None)
}

async fn body_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, 131072).await.expect("body");
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn post(app: &axum::Router, uri: &str, payload: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    (status, body_json(resp.into_body()).await)
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    (status, body_json(resp.into_body()).await)
}

fn seed_org(app: &axum::Router) -> axum::Router {
    app.clone()
}

// -- Org + orgchart integration (endpoints exist on main) --

#[tokio::test]
async fn orgchart_returns_json_for_seeded_org() {
    let app = test_router();
    let (s, _) = post(
        &app,
        "/api/orgs",
        json!({
            "id": "acme-chart",
            "mission": "Build products",
            "objectives": "Revenue",
            "ceo_agent": "carlo",
            "budget": 2000.0
        }),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);

    let (s, _) = post(
        &app,
        "/api/orgs/acme-chart/members",
        json!({"agent":"elena","role":"engineer","department":"platform"}),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);

    let (s, j) = get(&app, "/api/orgs/acme-chart/orgchart").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["departments"].is_array());
    let depts = j["departments"].as_array().unwrap();
    assert!(!depts.is_empty());
    assert_eq!(depts[0]["name"], "platform");
}

#[tokio::test]
async fn orgchart_not_found_for_missing_org() {
    let app = test_router();
    let (s, _) = get(&app, "/api/orgs/nonexistent/orgchart").await;
    assert!(s == StatusCode::NOT_FOUND || s == StatusCode::BAD_REQUEST);
}

// -- Org telemetry as metrics proxy --

#[tokio::test]
async fn org_telemetry_record_and_aggregate() {
    let app = test_router();
    let (s, _) = post(
        &app,
        "/api/orgs",
        json!({
            "id": "metrics-org",
            "mission": "Measure everything",
            "objectives": "Visibility",
            "ceo_agent": "nina",
            "budget": 500.0
        }),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);

    let (s, _) = post(
        &app,
        "/api/orgs/metrics-org/telemetry",
        json!({"agent":"worker-1","tokens_in":100,"tokens_out":50,"cost_usd":0.03}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (s, j) = get(&app, "/api/orgs/metrics-org/telemetry").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["total_tokens"].as_i64().unwrap_or(0) >= 150);
}

// -- Brain endpoint shape (current main) --

#[tokio::test]
async fn brain_returns_expected_arrays_on_empty_db() {
    let app = test_router();
    let (s, j) = get(&app, "/api/brain").await;
    assert_eq!(s, StatusCode::OK);
    for key in &["sessions", "agents", "plans", "tasks", "commits", "token_summary"] {
        assert!(j[key].is_array(), "missing array: {key}");
    }
}

// -- Night mode config boundary (unit, uses mesh::sandbox) --

#[test]
fn night_mode_boundary_cross_midnight() {
    use crate::mesh::sandbox::NightModeConfig;
    let cfg = NightModeConfig::default(); // 22-06
    assert!(cfg.is_active_at_hour(23));
    assert!(cfg.is_active_at_hour(0));
    assert!(cfg.is_active_at_hour(5));
    assert!(!cfg.is_active_at_hour(6));
    assert!(!cfg.is_active_at_hour(12));
    assert!(!cfg.is_active_at_hour(21));
    assert!(cfg.is_active_at_hour(22));
}

#[test]
fn night_mode_disabled_always_false() {
    use crate::mesh::sandbox::NightModeConfig;
    let cfg = NightModeConfig {
        enabled: false,
        ..Default::default()
    };
    assert!(!cfg.is_active_at_hour(0));
    assert!(!cfg.is_active_at_hour(23));
}

#[test]
fn night_mode_same_day_window() {
    use crate::mesh::sandbox::NightModeConfig;
    let cfg = NightModeConfig {
        start_hour: 2,
        end_hour: 5,
        ..Default::default()
    };
    assert!(!cfg.is_active_at_hour(1));
    assert!(cfg.is_active_at_hour(2));
    assert!(cfg.is_active_at_hour(4));
    assert!(!cfg.is_active_at_hour(5));
}

// -- Nightly job trigger integration --

#[tokio::test]
async fn nightly_jobs_list_has_required_shape() {
    let app = test_router();
    let (s, j) = get(&app, "/api/nightly/jobs").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["history"].is_array());
    assert!(j["definitions"].is_array());
}

// -- Stubs for W1-W4 features (not yet merged to main) --

#[tokio::test]
#[ignore = "W1: plan-org link endpoint not merged"]
async fn plan_org_link_create_plan_with_org_id() {
    let app = test_router();
    let _ = seed_org(&app);
    let (_s, _j) = post(
        &app,
        "/api/plans",
        json!({"name":"Linked Plan","org_id":"acme-chart","status":"doing"}),
    )
    .await;
    // After merge: GET /api/orgs/acme-chart/plans should include this plan
}

#[tokio::test]
#[ignore = "W2: org timeline endpoint not merged"]
async fn org_timeline_post_event_and_filter() {
    let app = test_router();
    let _ = seed_org(&app);
    // After merge: POST /api/orgs/:slug/timeline + GET with ?type= filter
}

#[tokio::test]
#[ignore = "W3: brain orgs array not merged"]
async fn brain_includes_orgs_array() {
    let app = test_router();
    let (s, j) = get(&app, "/api/brain").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["orgs"].is_array(), "brain should include orgs after W3 merge");
}

#[tokio::test]
#[ignore = "W3: global orgchart endpoint not merged"]
async fn global_orgchart_returns_chart() {
    let app = test_router();
    let (s, j) = get(&app, "/api/orgs/chart").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["chart"].is_string() || j["orgs"].is_array());
}

#[tokio::test]
#[ignore = "W4: marketplace endpoint not merged"]
async fn marketplace_lists_seeded_services() {
    let app = test_router();
    let (s, _j) = get(&app, "/api/marketplace").await;
    assert_eq!(s, StatusCode::OK);
    // After merge: verify services array with org provenance
}
