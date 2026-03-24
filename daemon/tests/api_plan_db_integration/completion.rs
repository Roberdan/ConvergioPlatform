use super::*;

#[tokio::test]
async fn api_plan_db_integration_cancel() {
    let (app, _tmp) = setup_app();

    let (_, resp) = post_json(
        &app,
        "/api/plan-db/create",
        json!({"project_id": "test", "name": "Cancel Test"}),
    )
    .await;
    let plan_id = resp["plan_id"].as_i64().unwrap();

    // Start it
    post_json(&app, &format!("/api/plan-db/start/{plan_id}"), json!({})).await;

    // Cancel
    let (status, resp) = post_json(
        &app,
        &format!("/api/plan-db/cancel/{plan_id}"),
        json!({"reason": "no longer needed"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["status"], "cancelled");
}

#[tokio::test]
async fn api_plan_db_integration_complete_blocks_with_pending() {
    let (app, _tmp) = setup_app();

    let (_, resp) = post_json(
        &app,
        "/api/plan-db/create",
        json!({"project_id": "test", "name": "Block Test"}),
    )
    .await;
    let plan_id = resp["plan_id"].as_i64().unwrap();

    // Import tasks
    post_json(
        &app,
        "/api/plan-db/import",
        json!({
            "plan_id": plan_id,
            "waves": [{"id": "W1", "name": "Wave 1", "tasks": [
                {"id": "T1", "title": "Pending task"}
            ]}]
        }),
    )
    .await;

    // Start
    post_json(&app, &format!("/api/plan-db/start/{plan_id}"), json!({})).await;

    // Try to complete — should fail due to pending tasks
    let (status, resp) =
        post_json(&app, &format!("/api/plan-db/complete/{plan_id}"), json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(resp["ok"], false);
}
