//! Integration tests for tokens, projects, notifications, peers, and nightly job endpoints.
use super::{get, post, put, test_router};
use axum::http::StatusCode;

#[tokio::test]
async fn tokens_daily_returns_array() {
    let r = test_router();
    let (s, j) = get(&r, "/api/tokens/daily").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j.is_array());
}

#[tokio::test]
async fn tokens_models_returns_array() {
    let r = test_router();
    let (s, j) = get(&r, "/api/tokens/models").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j.is_array());
}

#[tokio::test]
async fn projects_returns_all() {
    let r = test_router();
    let (s, j) = get(&r, "/api/projects").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j.as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn notifications_returns_unread() {
    let r = test_router();
    let (s, j) = get(&r, "/api/notifications").await;
    assert_eq!(s, StatusCode::OK);
    let arr = j.as_array().unwrap();
    assert!(!arr.is_empty());
    assert_eq!(arr[0]["is_read"], 0);
}

#[tokio::test]
async fn peers_returns_list() {
    let r = test_router();
    let (s, j) = get(&r, "/api/peers").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["peers"].is_array());
}

#[tokio::test]
async fn plans_assignable_returns_active() {
    let r = test_router();
    let (s, j) = get(&r, "/api/plans/assignable").await;
    assert_eq!(s, StatusCode::OK);
    let arr = j.as_array().unwrap();
    assert!(arr
        .iter()
        .any(|p| p["status"] == "doing" || p["status"] == "todo"));
}

#[tokio::test]
async fn nightly_retry_creates_child_run() {
    let r = test_router();
    let (s, j) = post(&r, "/api/nightly/jobs/1/retry", serde_json::json!({})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["triggered"], true);
    assert_eq!(j["parent_run_id"], "mirrorbuddy-nightly-20260308-030000");
}

#[tokio::test]
async fn nightly_trigger_creates_manual_run() {
    let r = test_router();
    let (s, j) = post(
        &r,
        "/api/nightly/jobs/trigger",
        serde_json::json!({"project_id": "mirrorbuddy"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["triggered"], true);
    assert_eq!(j["project_id"], "mirrorbuddy");
}

#[tokio::test]
async fn nightly_config_get_filters_by_project() {
    let r = test_router();
    let (s, j) = get(&r, "/api/nightly/config/mirrorbuddy").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["project_id"], "mirrorbuddy");
    let definitions = j["definitions"].as_array().expect("definitions array");
    assert_eq!(definitions.len(), 2);
    assert!(definitions
        .iter()
        .all(|row| row["name"] != "proj1-guardian"));
    assert!(definitions[0].get("run_fixes").is_some());
    assert!(definitions[0].get("timeout_sec").is_some());
}

#[tokio::test]
async fn nightly_config_update_and_toggle_persist_changes() {
    let r = test_router();
    let (s, j) = put(
        &r,
        "/api/nightly/config/mirrorbuddy",
        serde_json::json!({
            "run_fixes": 0,
            "schedule": "15 4 * * *",
            "timeout_sec": 3600
        }),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["rows_affected"].as_u64().unwrap() >= 2);

    let (toggle_status, toggle_json) = post(
        &r,
        "/api/nightly/jobs/definitions/1/toggle",
        serde_json::json!({}),
    )
    .await;
    assert_eq!(toggle_status, StatusCode::OK);
    assert_eq!(toggle_json["enabled"], false);

    let (_, updated) = get(&r, "/api/nightly/config/mirrorbuddy").await;
    let definitions = updated["definitions"]
        .as_array()
        .expect("definitions array");
    let primary = definitions
        .iter()
        .find(|row| row["id"].as_i64() == Some(1))
        .expect("primary definition");
    assert_eq!(primary["run_fixes"], 0);
    assert_eq!(primary["schedule"], "15 4 * * *");
    assert_eq!(primary["timeout_sec"], 3600);
    assert_eq!(primary["enabled"], 0);
}
