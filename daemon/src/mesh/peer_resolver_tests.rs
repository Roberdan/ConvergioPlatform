use super::*;
use crate::mesh::peers::{PeerConfig, PeersRegistry};
use std::collections::BTreeMap;

fn test_registry() -> PeersRegistry {
    let mut peers = BTreeMap::new();
    peers.insert(
        "m5max".to_string(),
        PeerConfig {
            ssh_alias: "RoberdanM5Max.local".to_string(),
            user: "roberdan".to_string(),
            os: "macos".to_string(),
            tailscale_ip: "198.51.100.1".to_string(),
            dns_name: "macbook-pro-di-roberdan.tail01f12c.ts.net".to_string(),
            capabilities: vec!["claude".into(), "copilot".into()],
            role: "coordinator".to_string(),
            status: "active".to_string(),
            thunderbolt_ip: None,
            mac_address: None,
            gh_account: None,
            runners: None,
            runner_paths: None,
            lan_ip: None,
            aliases: vec![],
        },
    );
    peers.insert(
        "m1pro".to_string(),
        PeerConfig {
            ssh_alias: "robertos-mbp-m1.tail01f12c.ts.net".to_string(),
            user: "roberdan".to_string(),
            os: "macos".to_string(),
            tailscale_ip: "198.51.100.2".to_string(),
            dns_name: "m1-pro-worker.tail01f12c.ts.net".to_string(),
            capabilities: vec!["claude".into()],
            role: "worker".to_string(),
            status: "active".to_string(),
            thunderbolt_ip: Some("10.0.0.2".to_string()),
            mac_address: None,
            gh_account: None,
            runners: None,
            runner_paths: None,
            lan_ip: None,
            aliases: vec!["worker1".into(), "macmini".into()],
        },
    );
    PeersRegistry {
        shared_secret: "test-secret".to_string(),
        peers,
    }
}

#[test]
fn resolve_exact_section_name() {
    let reg = test_registry();
    let resolved = resolve_from_registry("m5max", &reg).unwrap();
    assert_eq!(resolved.canonical_name, "m5max");
    assert_eq!(resolved.host, "RoberdanM5Max.local");
    assert_eq!(resolved.user, "roberdan");
    assert_eq!(resolved.port, DEFAULT_SSH_PORT);
}

#[test]
fn resolve_case_insensitive_section_name() {
    let reg = test_registry();
    let resolved = resolve_from_registry("M5Max", &reg).unwrap();
    assert_eq!(resolved.canonical_name, "m5max");
}

#[test]
fn resolve_by_tailscale_ip() {
    let reg = test_registry();
    let resolved = resolve_from_registry("198.51.100.2", &reg).unwrap();
    assert_eq!(resolved.canonical_name, "m1pro");
}

#[test]
fn resolve_by_ssh_alias() {
    let reg = test_registry();
    let resolved = resolve_from_registry("RoberdanM5Max.local", &reg).unwrap();
    assert_eq!(resolved.canonical_name, "m5max");
}

#[test]
fn resolve_by_dns_name_substring() {
    let reg = test_registry();
    let resolved = resolve_from_registry("m1-pro-worker", &reg).unwrap();
    assert_eq!(resolved.canonical_name, "m1pro");
}

#[test]
fn resolve_unknown_peer_returns_error() {
    let reg = test_registry();
    let err = resolve_from_registry("nonexistent", &reg);
    assert!(err.is_err());
}

#[test]
fn normalize_name_strips_separators() {
    assert_eq!(normalize_name("Mac-Worker_1"), "macworker1");
    assert_eq!(normalize_name("M5Max"), "m5max");
    assert_eq!(normalize_name("roberto's-mbp"), "robertosmbp");
}

#[test]
fn ssh_destination_prefers_alias() {
    let resolved = ResolvedPeer {
        canonical_name: "m5max".to_string(),
        host: "RoberdanM5Max.local".to_string(),
        port: DEFAULT_SSH_PORT,
        user: "roberdan".to_string(),
        ssh_alias: "RoberdanM5Max.local".to_string(),
        tailscale_ip: "198.51.100.1".to_string(),
        thunderbolt_ip: None,
        lan_ip: None,
        transport: "tailscale".to_string(),
    };
    assert_eq!(ssh_destination(&resolved), "roberdan@RoberdanM5Max.local");
}

#[test]
fn ssh_destination_falls_back_to_user_at_host() {
    let resolved = ResolvedPeer {
        canonical_name: "worker".to_string(),
        host: "198.51.100.2".to_string(),
        port: DEFAULT_SSH_PORT,
        user: "roberdan".to_string(),
        ssh_alias: String::new(),
        tailscale_ip: "198.51.100.2".to_string(),
        thunderbolt_ip: None,
        lan_ip: None,
        transport: "tailscale".to_string(),
    };
    assert_eq!(ssh_destination(&resolved), "roberdan@198.51.100.2");
}

#[test]
fn fallback_chain_ssh_alias_first() {
    let reg = test_registry();
    let resolved = resolve_from_registry("m5max", &reg).unwrap();
    // ssh_alias is "RoberdanM5Max.local", so host should be that
    assert_eq!(resolved.host, "RoberdanM5Max.local");
}

#[test]
fn fallback_chain_tailscale_when_no_ssh_alias() {
    let mut reg = test_registry();
    reg.peers.get_mut("m1pro").unwrap().ssh_alias = String::new();
    let resolved = resolve_from_registry("m1pro", &reg).unwrap();
    assert_eq!(resolved.host, "198.51.100.2");
}

#[test]
fn fallback_chain_dns_name_when_no_ip() {
    let mut reg = test_registry();
    let peer = reg.peers.get_mut("m1pro").unwrap();
    peer.ssh_alias = String::new();
    peer.tailscale_ip = String::new();
    peer.thunderbolt_ip = None;
    peer.lan_ip = None;
    let resolved = resolve_from_registry("m1pro", &reg).unwrap();
    assert_eq!(resolved.host, "m1-pro-worker.tail01f12c.ts.net");
}

#[test]
fn resolve_by_alias() {
    let reg = test_registry();
    let resolved = resolve_from_registry("worker1", &reg).unwrap();
    assert_eq!(resolved.canonical_name, "m1pro");
    // Case-insensitive alias match
    let resolved2 = resolve_from_registry("MacMini", &reg).unwrap();
    assert_eq!(resolved2.canonical_name, "m1pro");
}

#[test]
fn resolve_unknown_alias_fails() {
    let reg = test_registry();
    let err = resolve_from_registry("nonexistent-alias", &reg);
    assert!(err.is_err());
}
