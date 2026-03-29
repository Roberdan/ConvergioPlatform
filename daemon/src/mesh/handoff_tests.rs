use super::{PeerConfig, StaleHostStatus, SyncSourceInfo};
use std::collections::HashMap;

// ── detect_sync_source ───────────────────────────────────────────────────────

fn make_peers() -> HashMap<String, PeerConfig> {
    let mut peers = HashMap::new();
    peers.insert(
        "node-a".to_string(),
        PeerConfig {
            peer_name: "node-a".to_string(),
            ssh_alias: Some("node-a-ssh".to_string()),
            dns_name: Some("node-a.ts.net".to_string()),
        },
    );
    peers.insert(
        "node-b".to_string(),
        PeerConfig {
            peer_name: "node-b".to_string(),
            ssh_alias: Some("ubuntu@100.88.1.2".to_string()),
            dns_name: Some("node-b.local".to_string()),
        },
    );
    peers
}

#[test]
fn detect_sync_source_marks_same_node_when_execution_host_matches_target_alias() {
    let mut peers = HashMap::new();
    peers.insert(
        "node-b".to_string(),
        PeerConfig {
            peer_name: "node-b".to_string(),
            ssh_alias: Some("ubuntu@100.88.1.2".to_string()),
            dns_name: Some("node-b.local".to_string()),
        },
    );
    let info = super::detect_sync_source(
        "node-b",
        "ubuntu@100.88.1.2",
        "node-a",
        "node-b.local",
        "/tmp/worktree",
        "doing",
        1,
        &peers,
    );
    assert_eq!(
        info,
        SyncSourceInfo {
            source: "same_node".to_string(),
            ssh_source: Some("ubuntu@100.88.1.2".to_string()),
            ssh_target: "ubuntu@100.88.1.2".to_string(),
            worktree: "/tmp/worktree".to_string(),
            needs_stop: false,
            needs_stash: false,
        }
    );
}

#[test]
fn resolve_cli_prefers_primary_then_fallbacks() {
    let preferred = super::resolve_cli_command(
        "copilot",
        &HashMap::from([
            ("copilot".to_string(), "MISSING".to_string()),
            ("claude".to_string(), "claude".to_string()),
        ]),
    );
    assert_eq!(
        preferred,
        Some("claude --dangerously-skip-permissions --model sonnet".to_string())
    );
}

#[test]
fn stale_host_requires_heartbeat_age_or_ssh_reachability() {
    let stale = super::check_stale_host(1_000, Some(120), 10, true);
    assert!(stale.stale);
    assert!(stale.can_recover);
    assert_eq!(stale.reason, "heartbeat stale but SSH ok");
}

// ── parse_peers_conf ─────────────────────────────────────────────────────────

#[test]
fn parse_peers_conf_basic() {
    let conf = "[node-a]\nssh_alias=node-a.local\ndns_name=node-a.ts.net\n";
    let peers = super::parse_peers_conf(conf);
    assert_eq!(peers.len(), 1);
    let a = &peers["node-a"];
    assert_eq!(a.ssh_alias.as_deref(), Some("node-a.local"));
    assert_eq!(a.dns_name.as_deref(), Some("node-a.ts.net"));
}

#[test]
fn parse_peers_conf_multiple_sections() {
    let conf = "\
[alpha]\nssh_alias=alpha-ssh\n\
[beta]\ndns_name=beta.ts.net\n\
[gamma]\nssh_alias=gamma-ssh\ndns_name=gamma.ts.net\n";
    let peers = super::parse_peers_conf(conf);
    assert_eq!(peers.len(), 3);
    assert!(peers.contains_key("alpha"));
    assert!(peers.contains_key("beta"));
    assert!(peers.contains_key("gamma"));
}

#[test]
fn parse_peers_conf_ignores_comments() {
    let conf = "# top comment\n[node]\nssh_alias=n.local # inline comment\n";
    let peers = super::parse_peers_conf(conf);
    assert_eq!(peers.len(), 1);
    assert_eq!(peers["node"].ssh_alias.as_deref(), Some("n.local"));
}

#[test]
fn parse_peers_conf_empty_input() {
    let peers = super::parse_peers_conf("");
    assert!(peers.is_empty());
}

#[test]
fn parse_peers_conf_unknown_keys_ignored() {
    let conf = "[node]\nssh_alias=n.local\nunknown_key=value\ncustom=data\n";
    let peers = super::parse_peers_conf(conf);
    assert_eq!(peers["node"].ssh_alias.as_deref(), Some("n.local"));
}

// ── detect_sync_source additional ──────��─────────────────────────────────────

#[test]
fn detect_sync_source_coordinator_when_local() {
    let peers = make_peers();
    let info = super::detect_sync_source(
        "node-b", "node-b-ssh", "myhost", "myhost", "/wt", "doing", 1, &peers,
    );
    assert_eq!(info.source, "coordinator");
    assert!(!info.needs_stop);
    assert!(!info.needs_stash);
}

#[test]
fn detect_sync_source_coordinator_when_empty_or_prefix_host() {
    let peers = make_peers();
    let info = super::detect_sync_source(
        "node-b", "node-b-ssh", "myhost", "", "/wt", "doing", 1, &peers,
    );
    assert_eq!(info.source, "coordinator");
    let info2 = super::detect_sync_source(
        "node-b", "ssh", "myhost", "myhost.local", "/wt", "done", 0, &peers,
    );
    assert_eq!(info2.source, "coordinator");
}

#[test]
fn detect_sync_source_worker_needs_stop_when_doing() {
    let peers = make_peers();
    let info = super::detect_sync_source(
        "node-a",
        "node-a-ssh",
        "coordinator",
        "remote-worker",
        "/wt",
        "doing",
        2,
        &peers,
    );
    assert!(info.source.starts_with("worker:"));
    assert!(info.needs_stop);
    assert!(info.needs_stash);
}

#[test]
fn detect_sync_source_worker_no_stop_when_done() {
    let peers = make_peers();
    let info = super::detect_sync_source(
        "node-a",
        "node-a-ssh",
        "coordinator",
        "remote-worker",
        "/wt",
        "done",
        0,
        &peers,
    );
    assert!(info.source.starts_with("worker:"));
    assert!(!info.needs_stop);
}

// ── resolve_cli_command additional ────��──────────────────────────────────────

#[test]
fn resolve_cli_command_direct_copilot() {
    let detections = HashMap::from([("copilot".to_string(), "copilot".to_string())]);
    let result = super::resolve_cli_command("copilot", &detections);
    assert_eq!(result, Some("copilot --yolo".to_string()));
}

#[test]
fn resolve_cli_command_gh_copilot_variant() {
    let detections = HashMap::from([("copilot".to_string(), "gh-copilot".to_string())]);
    let result = super::resolve_cli_command("copilot", &detections);
    assert_eq!(result, Some("gh copilot -p".to_string()));
}

#[test]
fn resolve_cli_command_all_missing_returns_none() {
    let detections = HashMap::from([
        ("copilot".to_string(), "MISSING".to_string()),
        ("claude".to_string(), "MISSING".to_string()),
        ("opencode".to_string(), "MISSING".to_string()),
    ]);
    let result = super::resolve_cli_command("copilot", &detections);
    assert!(result.is_none());
}

#[test]
fn resolve_cli_command_empty_detections_returns_none() {
    let result = super::resolve_cli_command("copilot", &HashMap::new());
    assert!(result.is_none());
}

// ── check_stale_host additional ──────────────────────────────────────────────

#[test]
fn check_stale_host_fresh_heartbeat() {
    let status = super::check_stale_host(1000, Some(995), 60, false);
    assert!(!status.stale);
    assert!(status.reason.contains("5s ago"));
}

#[test]
fn check_stale_host_no_heartbeat_ssh_down() {
    let status = super::check_stale_host(1000, None, 60, false);
    assert!(status.stale);
    assert!(!status.can_recover);
    assert!(status.reason.contains("unreachable"));
}

#[test]
fn check_stale_host_exactly_at_threshold() {
    // age = 60, threshold = 60 → stale
    let status = super::check_stale_host(100, Some(40), 60, true);
    assert!(status.stale);
    assert!(status.can_recover);
}
