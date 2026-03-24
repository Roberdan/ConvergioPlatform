// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for GitHub API endpoints and helper functions.

use super::api_github::router;
use super::api_github_handlers::{extract_nwo, format_epoch_date};
use super::state::ServerState;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

// --- Pure function tests for extract_nwo ---

#[test]
fn extract_nwo_standard_url() {
    assert_eq!(extract_nwo("https://github.com/Owner/Repo"), "Owner/Repo");
}

#[test]
fn extract_nwo_with_git_suffix() {
    assert_eq!(
        extract_nwo("https://github.com/Owner/Repo.git"),
        "Owner/Repo"
    );
}

#[test]
fn extract_nwo_with_trailing_slash() {
    assert_eq!(extract_nwo("https://github.com/Owner/Repo/"), "Owner/Repo");
}

#[test]
fn extract_nwo_with_extra_path() {
    assert_eq!(
        extract_nwo("https://github.com/Owner/Repo/tree/main"),
        "Owner/Repo"
    );
}

#[test]
fn extract_nwo_empty_string() {
    assert_eq!(extract_nwo(""), "");
}

#[test]
fn extract_nwo_non_github_url() {
    assert_eq!(extract_nwo("https://gitlab.com/Owner/Repo"), "");
}

#[test]
fn extract_nwo_incomplete_url() {
    assert_eq!(extract_nwo("https://github.com/OwnerOnly"), "");
}

// --- Pure function tests for format_epoch_date ---

#[test]
fn format_epoch_date_unix_epoch() {
    // 1970-01-01
    assert_eq!(format_epoch_date(0), "1970-01-01");
}

#[test]
fn format_epoch_date_known_date() {
    // 2024-01-01 00:00:00 UTC = 1704067200
    assert_eq!(format_epoch_date(1704067200), "2024-01-01");
}

#[test]
fn format_epoch_date_another_known_date() {
    // 2026-03-22 00:00:00 UTC = 1774051200
    // Verify with a well-known epoch
    // 2000-01-01 00:00:00 UTC = 946684800
    assert_eq!(format_epoch_date(946684800), "2000-01-01");
}

#[test]
fn format_epoch_date_leap_year() {
    // 2024-02-29 00:00:00 UTC = 1709164800
    assert_eq!(format_epoch_date(1709164800), "2024-02-29");
}

// --- Integration tests for handlers ---

fn test_state() -> (ServerState, TempDir) {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let state = ServerState::new(db_path, None);
    (state, tmp)
}

fn app(state: ServerState) -> Router {
    router().with_state(state)
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn repo_create_missing_name_returns_error() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .method("POST")
        .uri("/api/github/repo/create")
        .header("content-type", "application/json")
        .body(Body::from(json!({}).to_string()))
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], false);
    assert!(json["error"].as_str().unwrap().contains("missing"));
}

#[tokio::test]
async fn repo_create_empty_name_returns_error() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .method("POST")
        .uri("/api/github/repo/create")
        .header("content-type", "application/json")
        .body(Body::from(json!({"name": ""}).to_string()))
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["ok"], false);
}

#[tokio::test]
async fn repo_create_whitespace_name_returns_error() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .method("POST")
        .uri("/api/github/repo/create")
        .header("content-type", "application/json")
        .body(Body::from(json!({"name": "   "}).to_string()))
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["ok"], false);
}

#[tokio::test]
async fn repo_create_valid_name_returns_ok() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .method("POST")
        .uri("/api/github/repo/create")
        .header("content-type", "application/json")
        .body(Body::from(json!({"name": "my-new-repo"}).to_string()))
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["repo"]["nameWithOwner"], "my-new-repo");
    assert_eq!(json["repo"]["isPrivate"], true);
}

#[tokio::test]
async fn commits_endpoint_returns_ok_for_any_plan() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .uri("/api/github/commits/1")
        .body(Body::empty())
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["plan_id"], 1);
    assert_eq!(json["local_commits"], json!([]));
    assert_eq!(json["remote_commits"], json!([]));
}

#[tokio::test]
async fn events_endpoint_returns_ok() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .uri("/api/github/events/test-project")
        .body(Body::empty())
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["project_id"], "test-project");
    assert!(json["local_events"].is_array());
}

#[tokio::test]
async fn commits_with_seeded_data() {
    let (state, _tmp) = test_state();
    let conn = state.get_conn().unwrap();
    conn.execute(
        "INSERT INTO plan_commits (plan_id, commit_sha, commit_message, lines_added, lines_removed, files_changed) \
         VALUES (42, 'abc123', 'fix: test commit', 10, 3, 2)",
        [],
    )
    .unwrap();
    drop(conn);

    let req = Request::builder()
        .uri("/api/github/commits/42")
        .body(Body::empty())
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    let commits = json["local_commits"].as_array().unwrap();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0]["commit_sha"], "abc123");
    assert_eq!(commits[0]["lines_added"], 10);
}
