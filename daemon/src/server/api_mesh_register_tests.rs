use super::super::state;

/// RAII guard that restores HOME on drop.
struct HomeGuard(Option<String>);
impl HomeGuard {
    fn new() -> Self {
        Self(std::env::var("HOME").ok())
    }
}
impl Drop for HomeGuard {
    fn drop(&mut self) {
        match &self.0 {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
    }
}

#[tokio::test]
async fn register_new_peer() {
    super::super::middleware::set_dev_mode(true);
    let _home_guard = HomeGuard::new();
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let st = state::ServerState::new(db_path, None);

    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".claude/config")).unwrap();
    std::fs::write(
        dir.path().join(".claude/config/peers.conf"),
        "[mesh]\nshared_secret=test-secret\n\n[coordinator]\n\
         ssh_alias=coord\nuser=admin\nos=macos\ntailscale_ip=100.1.1.1\n\
         dns_name=coord.ts.net\ncapabilities=claude\nrole=coordinator\n\
         status=active\n",
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
}

#[tokio::test]
async fn register_updates_existing_peer() {
    super::super::middleware::set_dev_mode(true);
    let _home_guard = HomeGuard::new();
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
    assert!(json["peers_config"].as_str().unwrap().contains("100.0.0.2"));
}

#[tokio::test]
async fn register_rejects_ini_injection() {
    super::super::middleware::set_dev_mode(true);
    let _home_guard = HomeGuard::new();
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let st = state::ServerState::new(db_path, None);

    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".claude/config")).unwrap();
    std::fs::write(
        dir.path().join(".claude/config/peers.conf"),
        "[mesh]\nshared_secret=s\n",
    )
    .unwrap();

    let req = super::RegisterRequest {
        name: "evil\n[injected]".to_string(),
        ssh_alias: "x".to_string(),
        user: "x".to_string(),
        os: "linux".to_string(),
        tailscale_ip: "100.0.0.1".to_string(),
        dns_name: "x.ts.net".to_string(),
        capabilities: vec![],
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
    assert!(result.is_err());
}

#[tokio::test]
async fn register_rejects_empty_ip() {
    super::super::middleware::set_dev_mode(true);
    let _home_guard = HomeGuard::new();
    let dir = tempfile::tempdir().unwrap();
    let st = state::ServerState::new(dir.path().join("t.db"), None);

    std::env::set_var("HOME", dir.path());
    std::fs::create_dir_all(dir.path().join(".claude/config")).unwrap();
    std::fs::write(
        dir.path().join(".claude/config/peers.conf"),
        "[mesh]\nshared_secret=s\n",
    )
    .unwrap();

    let req = super::RegisterRequest {
        name: "node".to_string(), ssh_alias: "n".to_string(),
        user: "u".to_string(), os: "linux".to_string(),
        tailscale_ip: "".to_string(), dns_name: "n.ts.net".to_string(),
        capabilities: vec![], role: "worker".to_string(),
        lan_ip: None, mac_address: None, gh_account: None,
    };
    let result = super::handle_register(
        axum::extract::State(st), axum::Json(req),
    ).await;
    assert!(result.is_err());
}
