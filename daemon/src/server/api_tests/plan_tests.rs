//! Integration tests for plan, mission, history, and task endpoints.
use super::{get, post, test_router};
use axum::http::StatusCode;

#[tokio::test]
async fn health_returns_ok_with_version() {
    let r = test_router();
    let (s, j) = get(&r, "/api/health").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["db"], true);
    assert!(j["version"].is_string());
    assert!(j["tables"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn overview_returns_plan_counts() {
    let r = test_router();
    let (s, j) = get(&r, "/api/overview").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j["plans_total"].as_i64().unwrap() >= 4);
    assert!(j["plans_active"].as_i64().unwrap() >= 1);
    assert!(j["plans_done"].as_i64().unwrap() >= 1);
}

#[tokio::test]
async fn mission_returns_plans_with_waves_and_project_name() {
    let r = test_router();
    let (s, j) = get(&r, "/api/mission").await;
    assert_eq!(s, StatusCode::OK);
    let plans = j["plans"].as_array().expect("plans array");
    assert!(!plans.is_empty());
    let active = plans
        .iter()
        .find(|m| m["plan"]["id"].as_i64() == Some(1))
        .expect("active plan #1");
    assert_eq!(active["plan"]["name"], "Active Plan Alpha");
    assert_eq!(active["plan"]["project_name"], "TestProject");
    assert_eq!(active["plan"]["status"], "doing");
    let waves = active["waves"].as_array().expect("waves array");
    assert_eq!(waves.len(), 2);
    assert!(waves[0].get("validated_at").is_some());
    let tasks = active["tasks"].as_array().expect("tasks array");
    assert_eq!(tasks.len(), 5);
}

#[tokio::test]
async fn mission_includes_cancelled_in_parking_lot() {
    let r = test_router();
    let (s, j) = get(&r, "/api/mission").await;
    assert_eq!(s, StatusCode::OK);
    let plans = j["plans"].as_array().unwrap();
    assert!(
        plans.iter().any(|m| m["plan"]["status"] == "cancelled"),
        "cancelled plans must appear for parking lot"
    );
}

#[tokio::test]
async fn plan_detail_returns_nested_shape() {
    let r = test_router();
    let (s, j) = get(&r, "/api/plan/1").await;
    assert_eq!(s, StatusCode::OK);
    assert!(j.get("plan").is_some(), "must have .plan");
    assert!(j.get("waves").is_some(), "must have .waves");
    assert!(j.get("tasks").is_some(), "must have .tasks");
    assert!(j.get("cost").is_some(), "must have .cost");
    assert_eq!(j["plan"]["id"], 1);
    assert_eq!(j["plan"]["project_name"], "TestProject");
    assert!(j["plan"]["human_summary"].is_string());
    let waves = j["waves"].as_array().unwrap();
    assert_eq!(waves.len(), 2);
    assert!(waves[0].get("validated_at").is_some());
    assert!(waves[0].get("pr_number").is_some());
    assert!(j["cost"]["tokens"].is_number());
    assert!(j["cost"]["cost"].is_number());
}

#[tokio::test]
async fn plan_detail_400_for_missing() {
    let r = test_router();
    let (s, _) = get(&r, "/api/plan/99999").await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn history_returns_done_plans() {
    let r = test_router();
    let (s, j) = get(&r, "/api/history").await;
    assert_eq!(s, StatusCode::OK);
    let arr = j.as_array().unwrap();
    assert!(!arr.is_empty());
    assert!(arr[0].get("project_name").is_some());
    assert!(arr[0].get("lines_added").is_some());
}

#[tokio::test]
async fn recent_missions_returns_last_day_with_nested_shape() {
    let r = test_router();
    let (s, j) = get(&r, "/api/missions/recent").await;
    assert_eq!(s, StatusCode::OK);
    let plans = j["plans"].as_array().expect("plans array");
    assert!(!plans.is_empty());
    assert_eq!(plans[0]["plan"]["id"], 2);
    assert!(plans[0]["plan"].get("finished_at").is_some());
    assert!(plans[0]["waves"].is_array());
    assert!(plans[0]["tasks"].is_array());
    assert!(plans.iter().all(|m| m["plan"]["status"] == "done"));
    assert!(plans.iter().all(|m| {
        let name = m["plan"]["name"]
            .as_str()
            .unwrap_or("")
            .to_ascii_lowercase();
        !name.contains("test")
    }));
    assert!(plans.iter().all(|m| {
        let name = m["plan"]["name"]
            .as_str()
            .unwrap_or("")
            .to_ascii_lowercase();
        let project = m["plan"]["project_name"]
            .as_str()
            .unwrap_or("")
            .to_ascii_lowercase();
        !name.contains("hyperdemo") && !project.contains("hyperdemo")
    }));
    assert!(plans.iter().all(|m| m["plan"]["id"] != 6));
    assert!(plans.iter().all(|m| m["plan"]["id"] != 7));
}

#[tokio::test]
async fn plan_status_changes_state() {
    let r = test_router();
    let (s, j) = post(
        &r,
        "/api/plan-status",
        serde_json::json!({"plan_id": 4, "status": "doing"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
}

#[tokio::test]
async fn plan_status_rejects_invalid() {
    let r = test_router();
    let (s, _) = post(
        &r,
        "/api/plan-status",
        serde_json::json!({"plan_id": 4, "status": "invalid"}),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn cancel_plan_cascades() {
    let r = test_router();
    let (s, j) = post(&r, "/api/plan/cancel?plan_id=1", serde_json::json!({})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert_eq!(j["action"], "cancelled");
    let (_, detail) = get(&r, "/api/plan/1").await;
    assert_eq!(detail["plan"]["status"], "cancelled");
}

#[tokio::test]
async fn reset_plan_resets_waves() {
    let r = test_router();
    let (s, j) = post(&r, "/api/plan/reset?plan_id=1", serde_json::json!({})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    let (_, detail) = get(&r, "/api/plan/1").await;
    assert_eq!(detail["plan"]["status"], "todo");
}

#[tokio::test]
async fn validate_plan_sets_done() {
    let r = test_router();
    let (s, j) = post(&r, "/api/plans/5/validate", serde_json::json!({})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(j["ok"], true);
    assert!(j["validated"].as_i64().unwrap() >= 1);
}

#[tokio::test]
async fn tasks_distribution_returns_counts() {
    let r = test_router();
    let (s, j) = get(&r, "/api/tasks/distribution").await;
    assert_eq!(s, StatusCode::OK);
    let arr = j.as_array().unwrap();
    assert!(!arr.is_empty());
    assert!(arr[0].get("status").is_some());
    assert!(arr[0].get("count").is_some());
}

#[tokio::test]
async fn tasks_blocked_returns_blocked() {
    let r = test_router();
    let (s, j) = get(&r, "/api/tasks/blocked").await;
    assert_eq!(s, StatusCode::OK);
    let arr = j.as_array().unwrap();
    assert!(arr.iter().any(|t| t["status"] == "blocked"));
}
