use super::*;

// 4. Full workspace lifecycle: seed → status → record event → delete → verify gone
#[tokio::test]
async fn test_workspace_lifecycle() {
    let app = TestApp::new();
    let ws_id = "ws-integ-lc-01";
    app.seed_workspace(ws_id, Some(10));

    // status → active workspace found
    let status_resp = app
        .router
        .clone()
        .oneshot(
            Request::get(format!("/api/workspace/status/{ws_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(status_resp.status(), StatusCode::OK);
    let status_body = body_json(status_resp).await;
    assert_eq!(status_body["workspace"]["workspace_id"], ws_id);
    assert!(status_body["recent_events"].is_array());

    // record an event
    let ev_payload = serde_json::json!({
        "workspace_id": ws_id,
        "agent":        "lifecycle-test",
        "action":       "file_read",
    })
    .to_string();
    let ev_resp = app
        .router
        .clone()
        .oneshot(
            Request::post("/api/workspace/events/record")
                .header("content-type", "application/json")
                .body(Body::from(ev_payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ev_resp.status(), StatusCode::OK);

    // delete workspace
    let del_payload = serde_json::json!({"workspace_id": ws_id}).to_string();
    let del_resp = app
        .router
        .clone()
        .oneshot(
            Request::post("/api/workspace/delete")
                .header("content-type", "application/json")
                .body(Body::from(del_payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(del_resp.status(), StatusCode::OK);

    // must not appear in active list
    let list_resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/list")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let list_body = body_json(list_resp).await;
    let workspaces = list_body["workspaces"].as_array().unwrap();
    assert!(
        !workspaces.iter().any(|w| w["workspace_id"] == ws_id),
        "deleted workspace must not appear in active list"
    );
}

// 5. POST /api/workspace/release with unknown workspace_id → error response
#[tokio::test]
async fn test_release_endpoint_missing_workspace() {
    let app = TestApp::new();
    let payload = serde_json::json!({
        "workspace_id": "ws-no-such-xyz",
        "repo":         "example-org/example-repo"
    })
    .to_string();
    let resp = app
        .router
        .oneshot(
            Request::post("/api/workspace/release")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    // release requires a real workspace; non-existent must not return 2xx
    assert_ne!(
        resp.status().as_u16() / 100,
        2,
        "release on missing workspace must not return 2xx"
    );
}

// 6. GET /api/workspace/deliverables?workspace_id=X on seeded workspace → array
#[tokio::test]
async fn test_deliverables_endpoint() {
    let app = TestApp::new();
    app.seed_workspace("ws-integ-del-01", Some(20));

    let resp = app
        .router
        .oneshot(
            Request::get("/api/workspace/deliverables?workspace_id=ws-integ-del-01")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["ok"], true);
    assert_eq!(body["workspace_id"], "ws-integ-del-01");
    assert!(
        body["deliverables"].is_array(),
        "deliverables must be an array (may be empty on fresh workspace)"
    );
}

// 7. POST /api/workspace/quality-gate on seeded workspace → gates array + all_passed bool
#[tokio::test]
async fn test_quality_gate_seeded_workspace() {
    let app = TestApp::new();
    app.seed_workspace("ws-integ-qg-01", None);

    let payload = serde_json::json!({"workspace_id": "ws-integ-qg-01"}).to_string();
    let resp = app
        .router
        .oneshot(
            Request::post("/api/workspace/quality-gate")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["ok"], true);
    assert!(body["gates"].is_array(), "gates must be an array");
    assert!(
        body["all_passed"].is_boolean(),
        "all_passed must be a boolean"
    );
}
