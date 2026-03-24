use super::*;

#[tokio::test]
async fn catalog_sync_scans_agent_files() {
    let (state, _tmp) = test_state();
    let router = app(state);

    let agent_dir = tempfile::tempdir().unwrap();
    let agent_content = "---\nname: synced-agent\ndescription: \"Synced from file\"\nmodel: claude-sonnet-4-6\ntools:\n  - view\n  - edit\n---\n\n# synced-agent\n";
    std::fs::write(
        agent_dir.path().join("synced-agent.agent.md"),
        agent_content,
    )
    .unwrap();
    // Non-agent file should be skipped
    std::fs::write(agent_dir.path().join("readme.md"), "# Readme").unwrap();

    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/sync")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"source_dir": agent_dir.path().to_string_lossy()}).to_string(),
        ))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["synced"], 1);
    assert_eq!(json["added"], 1);

    // Verify it's in the catalog
    let req = Request::builder()
        .uri("/api/agents/catalog")
        .body(Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["agents"][0]["name"], "synced-agent");

    // Re-sync should show synced=1, added=0
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/sync")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"source_dir": agent_dir.path().to_string_lossy()}).to_string(),
        ))
        .unwrap();
    let resp = router.oneshot(req).await.unwrap();
    let json = body_json(resp).await;
    assert_eq!(json["synced"], 1);
    assert_eq!(json["added"], 0);
}

#[tokio::test]
async fn catalog_sync_bad_dir_returns_error() {
    let (state, _tmp) = test_state();
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/sync")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"source_dir": "/nonexistent/path"}).to_string(),
        ))
        .unwrap();
    let resp = app(state).oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn catalog_enable_and_disable() {
    let (state, _tmp) = test_state();
    let router = app(state);

    // First create an agent in the catalog
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/create")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"name": "my-agent", "description": "My agent", "model": "claude-sonnet-4-6"})
                .to_string(),
        ))
        .unwrap();
    router.clone().oneshot(req).await.unwrap();

    // Enable it to a target dir
    let target_dir = tempfile::tempdir().unwrap();
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/enable")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"name": "my-agent", "target_dir": target_dir.path().to_string_lossy()})
                .to_string(),
        ))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["enabled"], "my-agent");

    // File should exist
    let agent_file = target_dir.path().join("my-agent.agent.md");
    assert!(agent_file.exists());
    let content = std::fs::read_to_string(&agent_file).unwrap();
    assert!(content.contains("name: my-agent"));

    // Disable it
    let req = Request::builder()
        .method("POST")
        .uri("/api/agents/disable")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"name": "my-agent", "target_dir": target_dir.path().to_string_lossy()})
                .to_string(),
        ))
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["ok"], true);
    assert_eq!(json["disabled"], "my-agent");

    // File should be gone
    assert!(!agent_file.exists());
}
