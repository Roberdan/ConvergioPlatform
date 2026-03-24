use super::*;

/// Heartbeat + watchdog
#[tokio::test]
async fn e2e_heartbeat_and_watchdog() {
    let (app, _tmp) = setup_app();

    let (s, r) = post_json(&app, "/api/heartbeat", json!({"peer_name": "test-node"})).await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["ok"].as_bool().unwrap());

    let (s, r) = get_json(&app, "/api/heartbeat/status").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["peers"].as_array().unwrap().len() >= 1);

    let (s, r) = get_json(&app, "/api/watchdog/status").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r.get("stale_tasks").is_some());
}

/// Peer topology + diagnostics
#[tokio::test]
async fn e2e_mesh_topology() {
    let (app, _tmp) = setup_app();

    let (s, _r) = get_json(&app, "/api/peers/coordinator").await;
    assert_eq!(s, StatusCode::OK);

    let (s, r) = get_json(&app, "/api/mesh/topology").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["nodes"].as_array().unwrap().len() >= 2);

    let (s, r) = get_json(&app, "/api/mesh/diagnostics").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["total_peers"].as_i64().unwrap() >= 2);
}

/// KB search
#[tokio::test]
async fn e2e_kb_search() {
    let (app, _tmp) = setup_app();
    let (s, r) = get_json(&app, "/api/plan-db/kb-search?q=E2E").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["count"].as_i64().unwrap() >= 1);
}

/// Worker launch
#[tokio::test]
async fn e2e_worker_launch() {
    let (app, _tmp) = setup_app();

    let (s, r) = post_json(
        &app,
        "/api/workers/launch",
        json!({"agent_type": "copilot", "model": "sonnet", "description": "E2E test"}),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["agent_id"].as_str().unwrap().contains("copilot"));

    let (s, r) = get_json(&app, "/api/workers").await;
    assert_eq!(s, StatusCode::OK);
    assert!(r["count"].as_i64().unwrap() >= 1);
}
