use super::*;

// ── 8. Peers roundtrip with real format ──────────────────────────────────────

#[test]
fn test_peers_roundtrip_with_real_format() {
    let test_conf = r#"
[mesh]
shared_secret=test-secret-v1

[testnode1]
ssh_alias=test1.local
user=testuser
os=macos
tailscale_ip=100.1.2.3
dns_name=test1.tail.ts.net
capabilities=claude,copilot
role=coordinator
status=active
"#;

    let tmp = NamedTempFile::new().expect("tempfile");
    std::fs::write(tmp.path(), test_conf).expect("write conf");

    // Parse
    let reg = PeersRegistry::load(tmp.path()).expect("load");
    assert_eq!(reg.shared_secret, "test-secret-v1");
    assert_eq!(reg.peers.len(), 1);

    let node = reg.peers.get("testnode1").expect("testnode1 present");
    assert_eq!(node.ssh_alias, "test1.local");
    assert_eq!(node.user, "testuser");
    assert_eq!(node.os, "macos");
    assert_eq!(node.tailscale_ip, "100.1.2.3");
    assert_eq!(node.role, "coordinator");
    assert!(node.capabilities.contains(&"claude".to_string()));
    assert!(node.capabilities.contains(&"copilot".to_string()));

    // Write back and re-parse
    let tmp2 = NamedTempFile::new().expect("tempfile2");
    reg.save(tmp2.path()).expect("save");
    let reg2 = PeersRegistry::load(tmp2.path()).expect("reload");

    assert_eq!(reg2.shared_secret, reg.shared_secret);
    assert_eq!(reg2.peers.len(), reg.peers.len());
    let node2 = reg2.peers.get("testnode1").expect("testnode1 in reload");
    assert_eq!(node2.role, node.role);
    assert_eq!(node2.capabilities, node.capabilities);
}

// ── 9. Profiles load ─────────────────────────────────────────────────────────

#[test]
fn test_profiles_load() {
    let toml_content = r#"
name = "dev-mac"
description = "Full macOS developer setup"
modules = ["brew", "vscode", "repos", "shell", "macos"]
"#;

    let tmp = NamedTempFile::new().expect("tempfile");
    std::fs::write(tmp.path(), toml_content).expect("write toml");

    let profile = load_profile(tmp.path()).expect("load_profile");
    assert_eq!(profile.name, "dev-mac");
    assert_eq!(profile.description, "Full macOS developer setup");
    assert!(profile.modules.contains(&"brew".to_string()));
    assert!(profile.modules.contains(&"macos".to_string()));
    assert_eq!(profile.modules.len(), 5);
}

// ── 10. Env Selections default ───────────────────────────────────────────────

#[test]
fn test_env_selections_default() {
    let sel = Selections::default();
    assert!(!sel.brew);
    assert!(!sel.vscode);
    assert!(!sel.repos);
    assert!(!sel.shell);
    assert!(!sel.macos);
    assert!(!sel.runners);
}

#[test]
fn test_env_selections_all() {
    let sel = Selections::all();
    assert!(sel.brew);
    assert!(sel.vscode);
    assert!(sel.repos);
    assert!(sel.shell);
    assert!(sel.macos);
    assert!(sel.runners);
}

// ── 11. JoinConfig serialization ─────────────────────────────────────────────

#[test]
fn test_join_config_serialization() {
    let config = JoinConfig {
        token: "tok.sig".to_owned(),
        admin_password: "hunter2".to_owned(),
        profiles: vec!["dev-mac".to_owned(), "claude-mesh".to_owned()],
        interactive: true,
        selections: JoinSelections::all(),
    };

    let json = serde_json::to_string(&config).expect("serialize");
    let back: JoinConfig = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(back.token, "tok.sig");
    assert_eq!(back.admin_password, "hunter2");
    assert_eq!(back.profiles, vec!["dev-mac", "claude-mesh"]);
    assert!(back.interactive);
    assert!(back.selections.network);
    assert!(back.selections.auth);
    assert!(back.selections.coordinator_migration);
}

// ── 12. Backward compat legacy peers ─────────────────────────────────────────

#[test]
fn test_backward_compat_legacy_peers() {
    let conf = "\
[mesh]
shared_secret=test-shared-secret-for-unit-tests

[mac-worker-1]
ssh_alias=mac-dev-ts
user=testuser
os=macos
tailscale_ip=100.64.0.1
dns_name=worker-1.example.ts.net
capabilities=claude,copilot
role=worker
status=active

[mac-worker-2]
ssh_alias=worker-2.example.ts.net
user=roberdan
os=macos
tailscale_ip=100.64.0.10
dns_name=worker-2.example.ts.net
capabilities=claude,copilot,ollama
role=coordinator
status=active
";

    let tmp = NamedTempFile::new().expect("tempfile");
    std::fs::write(tmp.path(), conf).expect("write");

    // load_legacy_peers
    let reg = load_legacy_peers(tmp.path()).expect("load_legacy_peers");
    assert_eq!(reg.shared_secret, "test-shared-secret-for-unit-tests");
    assert_eq!(reg.peers.len(), 2);

    let coordinator = reg.get_coordinator().expect("coordinator present");
    assert_eq!(coordinator.0, "mac-worker-2");

    // verify_backward_compat
    let report = verify_backward_compat(tmp.path()).expect("verify_backward_compat");
    assert!(report.has_shared_secret);
    assert_eq!(report.peer_count, 2);
    assert!(report.coordinator_present);
}

// ── 13. PeersRegistry add/remove/update ──────────────────────────────────────

#[test]
fn test_peers_registry_mutations() {
    let mut reg = PeersRegistry {
        shared_secret: "sec".to_owned(),
        peers: BTreeMap::new(),
    };

    reg.add_peer("alpha", make_peer("coordinator"));
    reg.add_peer("beta", make_peer("worker"));
    assert_eq!(reg.peers.len(), 2);

    // get_coordinator
    let (name, _) = reg.get_coordinator().expect("coordinator found");
    assert_eq!(name, "alpha");

    // list_active — both are active
    assert_eq!(reg.list_active().len(), 2);

    // update_role
    reg.update_role("alpha", "worker").expect("update_role");
    assert!(
        reg.get_coordinator().is_none(),
        "no coordinator after update"
    );

    // remove_peer
    let removed = reg.remove_peer("beta").expect("removed beta");
    assert_eq!(removed.role, "worker");
    assert_eq!(reg.peers.len(), 1);
}
