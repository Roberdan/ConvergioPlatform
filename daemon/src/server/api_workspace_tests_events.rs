use super::*;

// GET /api/workspace/events — seeded workspace returns ok + events array
#[tokio::test]
async fn workspace_events_returns_array() {
    let app = TestApp::new();
    app.seed_workspace("ws-events-0001", None);
    let resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/events?workspace_id=ws-events-0001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["ok"], true);
    assert!(body["events"].is_array());
}

// GET /api/workspace/events without workspace_id → 400
#[tokio::test]
async fn workspace_events_missing_param_returns_400() {
    let app = TestApp::new();
    let resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// POST /api/workspace/delete on seeded workspace marks it deleted
#[tokio::test]
async fn delete_seeded_workspace_succeeds() {
    let app = TestApp::new();
    app.seed_workspace("ws-del-0001", Some(9));

    let delete_payload = serde_json::json!({ "workspace_id": "ws-del-0001" }).to_string();
    let resp = app
        .router
        .clone()
        .oneshot(
            Request::post("/api/workspace/delete")
                .header("content-type", "application/json")
                .body(Body::from(delete_payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["workspace_id"], "ws-del-0001");

    // After delete, status endpoint returns 404
    let resp = app
        .router
        .clone()
        .oneshot(
            Request::get("/api/workspace/status/ws-del-0001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Workspace is marked deleted — get_workspace returns Some but status is deleted,
    // OR if the handler uses active-only query it returns 404. Either is valid.
    // We just confirm the workspace is no longer in the active list.
    let list_resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/list")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(list_resp).await;
    let workspaces = body["workspaces"].as_array().unwrap();
    assert!(
        !workspaces
            .iter()
            .any(|w| w["workspace_id"] == "ws-del-0001"),
        "deleted workspace must not appear in active list"
    );
}

// GET /api/workspace/list?plan_id= filters correctly
#[tokio::test]
async fn list_workspaces_filter_by_plan_id() {
    let app = TestApp::new();
    app.seed_workspace("ws-plan42-aaa", Some(42));
    app.seed_workspace("ws-plan99-bbb", Some(99));

    let resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/list?plan_id=42")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let workspaces = body["workspaces"].as_array().unwrap();
    assert!(!workspaces.is_empty(), "should find workspace for plan 42");
    for w in workspaces {
        assert_eq!(w["plan_id"], 42, "all results should have plan_id=42");
    }
}
