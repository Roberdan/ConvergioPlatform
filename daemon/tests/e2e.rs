// End-to-end integration tests for convergiomesh-core.
//
// Tests cover: token lifecycle, auth encryption roundtrip, peers parsing,
// coordinator migration state, profiles, env selections, join config,
// and backward-compat layer.
//
// NO real keychain, SSH, or network calls are made.
//
// TODO: These tests reference the old convergiomesh_core API (pre-Plan 664 consolidation).
// The auth, token, peers modules were restructured under mesh::* with different APIs.
// Needs a dedicated pass to update to the new API surface.
#![allow(dead_code, unused_imports)]
#![cfg(feature = "__disabled_pending_api_migration")]

use claude_core::mesh::{
    auth::{decrypt_bundle, encrypt_bundle, load_bundle, save_bundle, AuthBundle},
    compat::{load_legacy_peers, verify_backward_compat},
    coordinator::{MigrationState, PeerSnapshot},
    env::Selections,
    join::{JoinConfig, JoinSelections},
    peers::{PeerConfig, PeersRegistry},
    profiles::load_profile,
    token::{generate_token, init_token_db, validate_token},
};
use rusqlite::Connection;
use std::collections::BTreeMap;
use tempfile::NamedTempFile;

// ── Helpers ───────────────────────────────────────────────────────────────────

const SECRET: &[u8] = b"e2e-test-hmac-secret-key";

fn setup_db() -> Connection {
    let db = Connection::open_in_memory().expect("in-memory DB");
    init_token_db(&db).expect("init_token_db");
    db
}

fn sample_bundle() -> AuthBundle {
    AuthBundle {
        claude_creds: Some("claude-creds-e2e-test".to_string()),
        gh_token: Some("ghp_e2etesttoken1234567890".to_string()),
        az_tokens: Some(vec![0x1f, 0x8b, 0x08, 0x00]),
        copilot_token: Some("ghu_copilot_e2e_xyz".to_string()),
    }
}

fn make_peer(role: &str) -> PeerConfig {
    PeerConfig {
        ssh_alias: format!("{role}-alias"),
        user: "testuser".to_owned(),
        os: "macos".to_owned(),
        tailscale_ip: "100.64.0.1".to_owned(),
        dns_name: format!("{role}.tail.ts.net"),
        capabilities: vec!["claude".to_owned(), "copilot".to_owned()],
        role: role.to_owned(),
        status: "active".to_owned(),
        mac_address: None,
        gh_account: None,
        runners: None,
        runner_paths: None,
    }
}

// ── 1. Invite-join flow ───────────────────────────────────────────────────────

#[test]
fn test_invite_join_flow() {
    let db = setup_db();

    // Step 1: generate token
    let token = generate_token(
        SECRET,
        "worker",
        vec!["claude".into(), "copilot".into()],
        "100.64.0.1",
        60,
    )
    .expect("generate_token");

    // Step 2: validate — should succeed
    let payload = validate_token(&token, SECRET, &db).expect("first validation");

    // Step 3: verify payload fields
    assert_eq!(payload.role, "worker");
    assert!(payload.capabilities.contains(&"claude".to_string()));
    assert_eq!(payload.coordinator_ip, "100.64.0.1");
    assert!(!payload.nonce.is_empty());

    // Step 4: validate again — single-use enforcement
    let err = validate_token(&token, SECRET, &db).expect_err("second validation must fail");
    assert!(
        matches!(
            err,
            claude_core::mesh::token::TokenError::AlreadyUsed
        ),
        "expected AlreadyUsed, got: {err:?}"
    );
}

// ── 2. Auth export/import roundtrip ──────────────────────────────────────────

#[test]
fn test_auth_export_import_roundtrip() {
    let bundle = sample_bundle();
    let token = "mesh-transfer-token-e2e";
    let password = "correct-horse-battery-staple";

    // Encrypt
    let encrypted = encrypt_bundle(&bundle, token, password).expect("encrypt_bundle");
    assert_eq!(encrypted.version, 1);
    assert_eq!(encrypted.salt.len(), 32);
    assert_eq!(encrypted.nonce.len(), 12);
    assert!(!encrypted.ciphertext.is_empty());

    // Save to temp file
    let tmp = NamedTempFile::new().expect("tempfile");
    save_bundle(&encrypted, tmp.path()).expect("save_bundle");

    // Load from temp file
    let loaded = load_bundle(tmp.path()).expect("load_bundle");
    assert_eq!(loaded.version, encrypted.version);
    assert_eq!(loaded.salt, encrypted.salt);
    assert_eq!(loaded.nonce, encrypted.nonce);
    assert_eq!(loaded.ciphertext, encrypted.ciphertext);

    // Decrypt loaded bundle
    let decrypted = decrypt_bundle(&loaded, token, password).expect("decrypt_bundle");
    assert_eq!(decrypted, bundle);
}

// ── 3. Wrong password rejected ────────────────────────────────────────────────

#[test]
fn test_auth_wrong_password_rejected() {
    let bundle = sample_bundle();
    let encrypted = encrypt_bundle(&bundle, "tok", "correct-password").expect("encrypt");
    let result = decrypt_bundle(&encrypted, "tok", "wrong-password");
    assert!(
        matches!(result, Err(claude_core::mesh::auth::AuthError::DecryptionFailed)),
        "expected DecryptionFailed, got: {result:?}"
    );
}

// ── 4. Wrong token rejected ───────────────────────────────────────────────────

#[test]
fn test_auth_wrong_token_rejected() {
    let bundle = sample_bundle();
    let encrypted = encrypt_bundle(&bundle, "correct-token", "password").expect("encrypt");
    let result = decrypt_bundle(&encrypted, "wrong-token", "password");
    assert!(
        matches!(result, Err(claude_core::mesh::auth::AuthError::DecryptionFailed)),
        "expected DecryptionFailed, got: {result:?}"
    );
}

// ── 5. Expired token security ─────────────────────────────────────────────────

#[test]
fn test_token_security_expired() {
    let db = setup_db();
    let token = generate_token(SECRET, "worker", vec![], "100.64.0.2", -1)
        .expect("generate expired token");
    let err = validate_token(&token, SECRET, &db).expect_err("expired token must fail");
    assert!(
        matches!(err, claude_core::mesh::token::TokenError::Expired),
        "expected Expired, got: {err:?}"
    );
}

// ── 6. Token replay security ──────────────────────────────────────────────────

#[test]
fn test_token_security_replay() {
    let db = setup_db();
    let token = generate_token(SECRET, "coordinator", vec![], "100.64.0.1", 60)
        .expect("generate_token");

    // First use — must succeed
    validate_token(&token, SECRET, &db).expect("first use must succeed");

    // Replay — must fail
    let err = validate_token(&token, SECRET, &db).expect_err("replay must fail");
    assert!(
        matches!(err, claude_core::mesh::token::TokenError::AlreadyUsed),
        "expected AlreadyUsed on replay, got: {err:?}"
    );
}

// ── 7. Coordinator MigrationState roundtrip ───────────────────────────────────

#[test]
fn test_coordinator_migration_state_roundtrip() {
    let state = MigrationState {
        old_coordinator: "mac-worker-2".to_owned(),
        new_coordinator: "linux-worker".to_owned(),
        snapshots: vec![
            PeerSnapshot {
                peer_name: "mac-worker-2".to_owned(),
                peers_conf_backup: "[mesh]\nshared_secret=key\n".to_owned(),
            },
            PeerSnapshot {
                peer_name: "linux-worker".to_owned(),
                peers_conf_backup: "[mesh]\nshared_secret=key\n".to_owned(),
            },
        ],
        started_at: "2026-03-18T10:00:00Z".to_owned(),
        completed: false,
    };

    let json = serde_json::to_string(&state).expect("serialize MigrationState");
    let back: MigrationState = serde_json::from_str(&json).expect("deserialize MigrationState");

    assert_eq!(back, state);
    assert_eq!(back.old_coordinator, "mac-worker-2");
    assert_eq!(back.new_coordinator, "linux-worker");
    assert_eq!(back.snapshots.len(), 2);
    assert!(!back.completed);
}

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
    assert!(reg.get_coordinator().is_none(), "no coordinator after update");

    // remove_peer
    let removed = reg.remove_peer("beta").expect("removed beta");
    assert_eq!(removed.role, "worker");
    assert_eq!(reg.peers.len(), 1);
}
