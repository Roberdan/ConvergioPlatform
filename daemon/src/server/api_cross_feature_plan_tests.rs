// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Cross-feature: non-code plan completion with output_type=document tasks.

use super::api_cross_feature_helpers::{get_json, post_json, setup_state};
use super::state::ServerState;
use axum::http::StatusCode;
use axum::Router;
use serde_json::json;

fn plan_app(state: ServerState) -> Router {
    use super::api_plan_db::router as plan_db_router;
    use super::api_plan_db_import::router as import_router;
    use super::api_plan_db_lifecycle::router as lifecycle_router;
    use super::api_plan_db_ops::router as ops_router;
    use super::api_plan_db_query::router as query_router;
    use super::api_plan_db_review::router as review_router;
    lifecycle_router()
        .merge(ops_router())
        .merge(query_router())
        .merge(review_router())
        .merge(import_router())
        .merge(plan_db_router())
        .with_state(state)
}

/// Non-code plan: all output_type=document -> mark all done -> plan complete
#[tokio::test]
async fn non_code_plan_document_tasks_complete() {
    let (state, _tmp) = setup_state("doc-proj", "Doc Project");
    let app = plan_app(state);

    // 1. Create plan
    let (status, resp) = post_json(
        &app,
        "/api/plan-db/create",
        json!({"project_id": "doc-proj", "name": "Documentation Plan"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "create plan: {resp}");
    let plan_id = resp["plan_id"].as_i64().expect("plan_id");

    // 2. Import waves with document-output tasks
    let (status, resp) = post_json(
        &app,
        "/api/plan-db/import",
        json!({
            "plan_id": plan_id,
            "waves": [{
                "id": "W1",
                "name": "Documentation Wave",
                "tasks": [
                    {
                        "id": "TD-01",
                        "title": "Write architecture ADR",
                        "output_type": "document",
                        "validator_agent": "doc-validator"
                    },
                    {
                        "id": "TD-02",
                        "title": "Write API reference",
                        "output_type": "document",
                        "validator_agent": "doc-validator"
                    },
                    {
                        "id": "TD-03",
                        "title": "Write onboarding guide",
                        "output_type": "document",
                        "validator_agent": "doc-validator"
                    }
                ]
            }]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "import: {resp}");
    assert_eq!(resp["tasks_created"], 3);

    // 3. Register review + approve + start
    let (status, _) = post_json(
        &app,
        "/api/plan-db/review/register",
        json!({
            "plan_id": plan_id,
            "reviewer_agent": "plan-reviewer",
            "verdict": "approved"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = post_json(
        &app,
        &format!("/api/plan-db/approve/{plan_id}"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, resp) = post_json(
        &app,
        &format!("/api/plan-db/start/{plan_id}"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "start: {resp}");
    assert_eq!(resp["status"], "doing");

    // 4. Get task IDs from execution tree
    let (status, resp) = get_json(
        &app,
        &format!("/api/plan-db/execution-tree/{plan_id}"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let tasks = resp["tree"].as_array().unwrap()[0]["tasks"]
        .as_array()
        .expect("tasks");
    assert_eq!(tasks.len(), 3);

    // 5. Mark all tasks done
    for task in tasks {
        let task_id = task["id"].as_i64().expect("task id");
        let (status, _) = post_json(
            &app,
            "/api/plan-db/task/update",
            json!({
                "task_id": task_id,
                "status": "done",
                "notes": "Document written and reviewed"
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "update task {task_id}");
    }

    // 6. Complete plan — all tasks done
    let (status, resp) = post_json(
        &app,
        &format!("/api/plan-db/complete/{plan_id}"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "complete: {resp}");
    assert_eq!(resp["status"], "completed");
}

/// Verify incomplete document plan cannot be completed
#[tokio::test]
async fn non_code_plan_blocks_completion_with_pending_docs() {
    let (state, _tmp) = setup_state("doc-proj", "Doc Project");
    let app = plan_app(state);

    let (_, resp) = post_json(
        &app,
        "/api/plan-db/create",
        json!({"project_id": "doc-proj", "name": "Incomplete Doc Plan"}),
    )
    .await;
    let plan_id = resp["plan_id"].as_i64().unwrap();

    post_json(
        &app,
        "/api/plan-db/import",
        json!({
            "plan_id": plan_id,
            "waves": [{
                "id": "W1",
                "name": "Docs",
                "tasks": [
                    {"id": "TD-01", "title": "Write spec", "output_type": "document"}
                ]
            }]
        }),
    )
    .await;

    // Start plan
    post_json(&app, &format!("/api/plan-db/start/{plan_id}"), json!({})).await;

    // Try complete with pending task — should fail
    let (status, resp) = post_json(
        &app,
        &format!("/api/plan-db/complete/{plan_id}"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "should block: {resp}");
    assert_eq!(resp["ok"], false);
}
