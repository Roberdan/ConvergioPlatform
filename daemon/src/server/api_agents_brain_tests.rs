// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Integration tests for GET /api/brain endpoint.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use super::api_agents_tests::{body_json, seed_db, test_router};

// --- GET /api/brain --------------------------------------------------------

#[tokio::test]
async fn brain_returns_full_graph_shape() {
    let (app, db) = test_router();
    seed_db(&db);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/brain")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert!(json["sessions"].is_array());
    assert!(json["agents"].is_array());
    assert!(json["recent"].is_array());
    assert!(json["plans"].is_array());
    assert!(json["tasks"].is_array());
    assert!(json["commits"].is_array());
    assert!(json["token_summary"].is_array());
}

#[tokio::test]
async fn brain_sessions_and_agents_are_separate() {
    let (app, db) = test_router();
    seed_db(&db);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/brain")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    let sessions = json["sessions"].as_array().unwrap();
    let agents = json["agents"].as_array().unwrap();
    // session-cli-001 in sessions, worker-A in agents
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["agent_id"], "session-cli-001");
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["agent_id"], "worker-A");
    assert_eq!(agents[0]["parent_session"], "session-cli-001");
}

#[tokio::test]
async fn brain_plans_shows_active_plans() {
    let (app, db) = test_router();
    seed_db(&db);
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/brain")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let json = body_json(resp.into_body()).await;
    let plans = json["plans"].as_array().unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0]["name"], "Deploy Alpha");
    assert_eq!(plans[0]["status"], "doing");
}

#[tokio::test]
async fn brain_empty_db_returns_all_arrays() {
    let (app, _db) = test_router();
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/brain")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp.into_body()).await;
    assert_eq!(json["sessions"].as_array().unwrap().len(), 0);
    assert_eq!(json["agents"].as_array().unwrap().len(), 0);
    assert_eq!(json["plans"].as_array().unwrap().len(), 0);
}
