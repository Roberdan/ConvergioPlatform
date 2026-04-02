use super::super::state;

#[tokio::test]
async fn register_new_peer() {
    super::super::middleware::set_dev_mode(true);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let st = state::ServerState::new(db_path, None);

    // Write a minimal peers.conf
    let peers_dir = dir.path().join("peers");
    std::fs::create_dir_all(&peers_dir).unwrap();
    let peers_path = peers_dir.join("peers.conf");
    std::fs::write(
        &peers_path,
        "[mesh]\nshared_secret=test-secret\n\n[coordinator]\n\
         ssh_alias=coord\nuser=admin\nos=macos\ntailscale_ip=100.1.1.1\n\
         dns_name=coord.ts.net\ncapabilities=claude\nrole=coordinator\n\
         status=active\n",
    )
    .unwrap();

    // Override HOME so peers_conf_path() finds our temp dir
    let original_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".claude/config")).unwrap();
    std::fs::copy(
        &peers_path,
        dir.path().join(".claude/config/peers.conf"),
    )
    .unwrap();

    let req = super::RegisterRequest {
        name: "new-worker".to_string(),
        ssh_alias: "worker-ts".to_string(),
        user: "bob".to_string(),
        os: "linux".to_string(),
        tailscale_ip: "100.2.2.2".to_string(),
        dns_name: "worker.ts.net".to_string(),
        capabilities: vec!["claude".into(), "copilot".into()],
        role: "worker".to_string(),
        lan_ip: Some("192.168.1.50".to_string()),
        mac_address: None,
        gh_account: None,
    };

    let result = super::handle_register(
        axum::extract::State(st),
        axum::Json(req),
    )
    .await;
    assert!(result.is_ok());
    let json = result.unwrap().0;
    assert_eq!(json["ok"], true);
    assert!(json["message"].as_str().unwrap().contains("registered"));
    assert!(json["peers_config"].as_str().unwrap().contains("new-worker"));
    assert!(json["peers_config"].as_str().unwrap().contains("100.2.2.2"));

    // Restore HOME
    if let Some(h) = original_home {
        std::env::set_var("HOME", h);
    }
}

#[tokio::test]
async fn register_updates_existing_peer() {
    super::super::middleware::set_dev_mode(true);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let st = state::ServerState::new(db_path, None);

    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".claude/config")).unwrap();
    std::fs::write(
        dir.path().join(".claude/config/peers.conf"),
        "[mesh]\nshared_secret=s\n\n[existing]\n\
         ssh_alias=old\nuser=u\nos=linux\ntailscale_ip=100.0.0.1\n\
         dns_name=old.ts.net\ncapabilities=claude\nrole=worker\nstatus=active\n",
    )
    .unwrap();

    let req = super::RegisterRequest {
        name: "existing".to_string(),
        ssh_alias: "new-alias".to_string(),
        user: "u".to_string(),
        os: "linux".to_string(),
        tailscale_ip: "100.0.0.2".to_string(),
        dns_name: "new.ts.net".to_string(),
        capabilities: vec!["claude".into()],
        role: "worker".to_string(),
        lan_ip: None,
        mac_address: None,
        gh_account: None,
    };

    let result = super::handle_register(
        axum::extract::State(st),
        axum::Json(req),
    )
    .await;
    let json = result.unwrap().0;
    assert_eq!(json["ok"], true);
    assert!(json["message"].as_str().unwrap().contains("updated"));
    // New IP should be present
    assert!(json["peers_config"].as_str().unwrap().contains("100.0.0.2"));
}
