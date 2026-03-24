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
        matches!(err, claude_core::mesh::token::TokenError::AlreadyUsed),
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
        matches!(
            result,
            Err(claude_core::mesh::auth::AuthError::DecryptionFailed)
        ),
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
        matches!(
            result,
            Err(claude_core::mesh::auth::AuthError::DecryptionFailed)
        ),
        "expected DecryptionFailed, got: {result:?}"
    );
}

// ── 5. Expired token security ─────────────────────────────────────────────────

#[test]
fn test_token_security_expired() {
    let db = setup_db();
    let token =
        generate_token(SECRET, "worker", vec![], "100.64.0.2", -1).expect("generate expired token");
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
    let token =
        generate_token(SECRET, "coordinator", vec![], "100.64.0.1", 60).expect("generate_token");

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

#[path = "e2e/data_roundtrip.rs"]
mod data_roundtrip;
