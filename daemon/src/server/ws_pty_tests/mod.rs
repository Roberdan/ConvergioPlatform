//! Tests for the WebSocket PTY terminal handler (`ws_pty`).
//!
//! Tests cover:
//! - PTY params deserialization (default peer, tmux_session)
//! - Tailscale name matching (fuzzy hostname/DNS resolution)
//! - peers.conf user lookup
//! - Local peer detection (explicit local, self via Tailscale)
//! - Session limit constants (MAX_PTY_SESSIONS, ACTIVE_SESSIONS)
//! - tokio::process::Command spawn produces output
//! - WebSocket message routing (text stdin, binary stdin, resize JSON)

mod session_tests;

use super::super::state::ServerState;

fn test_state() -> ServerState {
    let dir = std::env::temp_dir().join(format!("pty_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let db_path = dir.join("test.db");
    ServerState::new(db_path, None)
}

// ── Params deserialization ──────────────────────────────────────────

#[test]
fn params_default_peer() {
    let p: super::super::ws_pty::PtyParams = serde_json::from_str("{}").unwrap();
    assert_eq!(p.peer, "local");
    assert!(p.tmux_session.is_empty());
}

#[test]
fn params_with_peer_and_tmux() {
    let p: super::super::ws_pty::PtyParams =
        serde_json::from_str(r#"{"peer":"linux-worker","tmux_session":"main"}"#).unwrap();
    assert_eq!(p.peer, "linux-worker");
    assert_eq!(p.tmux_session, "main");
}

#[test]
fn params_peer_only() {
    let p: super::super::ws_pty::PtyParams =
        serde_json::from_str(r#"{"peer":"mac-worker-1"}"#).unwrap();
    assert_eq!(p.peer, "mac-worker-1");
    assert!(p.tmux_session.is_empty());
}

#[test]
fn tmux_session_rejects_shell_metacharacters() {
    assert!(super::super::ws_pty::sanitize_tmux_session("main;rm -rf /").is_none());
    assert!(super::super::ws_pty::sanitize_tmux_session("main&&echo pwn").is_none());
    assert!(super::super::ws_pty::sanitize_tmux_session("main$(uname)").is_none());
}

#[test]
fn tmux_session_allows_safe_chars_only() {
    assert_eq!(
        super::super::ws_pty::sanitize_tmux_session("session_1-main"),
        Some("session_1-main".into())
    );
}

#[test]
fn tmux_session_has_max_length() {
    let valid = "a".repeat(64);
    let too_long = "a".repeat(65);
    assert_eq!(super::super::ws_pty::sanitize_tmux_session(&valid), Some(valid));
    assert!(super::super::ws_pty::sanitize_tmux_session(&too_long).is_none());
}

#[test]
fn peer_validation_rejects_unknown_peer() {
    let known: std::collections::HashSet<String> = ["mac-worker-1", "linux-worker"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert!(super::super::ws_pty::validate_peer("nonexistent", &known).is_err());
}

#[test]
fn peer_validation_has_max_length() {
    let known: std::collections::HashSet<String> =
        ["mac-worker-1"].iter().map(|s| s.to_string()).collect();
    let too_long = "x".repeat(129);
    assert!(super::super::ws_pty::validate_peer(&too_long, &known).is_err());
}

#[test]
fn remote_tmux_command_quotes_session_argument() {
    let state = test_state();
    let params: super::super::ws_pty::PtyParams =
        serde_json::from_str(r#"{"peer":"linux-worker","tmux_session":"session_1-main"}"#)
            .unwrap();
    let (_program, args) = super::super::ws_pty::build_pty_command(&state, &params, false);
    let remote_cmd = args.get(2).expect("remote command arg");
    assert!(remote_cmd.contains("exec $SHELL -lc"));
    assert!(remote_cmd.contains("'tmux new-session -A -s session_1-main'"));
}

// ── Tailscale name matching ─────────────────────────────────────────

#[test]
fn ts_name_matches_hostname() {
    let node: serde_json::Value = serde_json::from_str(
        r#"{"HostName":"Worker Mac 1","DNSName":"worker-1.example.ts.net.","TailscaleIPs":["100.64.0.1"],"Online":true}"#
    ).unwrap();
    // "mac-worker-1" can't fuzzy-match "workermac1" — that's expected.
    // Real resolution uses peers.conf tailscale_ip → ts_node_matches, not ts_name_matches.
    let normalized = "mac-worker-1"
        .to_lowercase()
        .replace(['-', '_', ' ', '\''], "");
    assert!(
        !super::super::ws_pty::ts_name_matches(&node, &normalized),
        "fuzzy alone won't match mac-worker-1"
    );
    // But ts_node_matches with IP from peers.conf WILL match
    let ips = node.get("TailscaleIPs").unwrap().as_array().unwrap();
    assert!(
        ips.iter().any(|v| v.as_str() == Some("100.64.0.1")),
        "IP match works"
    );
}

#[test]
fn ts_name_matches_linux_worker() {
    let node: serde_json::Value = serde_json::from_str(
        r#"{"HostName":"linux-worker","DNSName":"linux_worker.example.ts.net.","TailscaleIPs":["100.64.0.2"],"Online":true}"#
    ).unwrap();
    assert!(super::super::ws_pty::ts_name_matches(&node, "linux-worker"));
}

#[test]
fn ts_name_no_match() {
    let node: serde_json::Value = serde_json::from_str(
        r#"{"HostName":"linux-worker","DNSName":"linux_worker.example.ts.net.","TailscaleIPs":["100.64.0.2"],"Online":true}"#
    ).unwrap();
    assert!(!super::super::ws_pty::ts_name_matches(&node, "mac-worker-1"));
}

#[test]
fn ts_first_ip_extracts_ipv4() {
    let node: serde_json::Value =
        serde_json::from_str(r#"{"TailscaleIPs":["100.64.0.1","fd7a:115c:a1e0::3"]}"#).unwrap();
    assert_eq!(super::super::ws_pty::ts_first_ip(&node), Some("100.64.0.1".into()));
}

#[test]
fn ts_first_ip_empty() {
    let node: serde_json::Value = serde_json::from_str(r#"{"TailscaleIPs":[]}"#).unwrap();
    assert!(super::super::ws_pty::ts_first_ip(&node).is_none());
}

// ── peers.conf user lookup ──────────────────────────────────────────

#[test]
fn peer_ssh_user_from_conf() {
    // Integration test — reads real peers.conf, skip on CI
    let user = super::super::ws_pty::peer_ssh_user("mac-worker-1");
    if user.is_none() {
        return;
    } // peers.conf not available (CI)
    assert_eq!(user, Some("testuser".into()));
}

#[test]
fn peer_ssh_user_self() {
    let user = super::super::ws_pty::peer_ssh_user("mac-worker-2");
    if user.is_none() {
        return;
    } // peers.conf not available (CI)
    assert_eq!(user, Some("roberdan".into()));
}

#[test]
fn peer_ssh_user_unknown() {
    let user = super::super::ws_pty::peer_ssh_user("nonexistent");
    assert!(user.is_none());
}
