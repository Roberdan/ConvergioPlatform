// Unit tests for delegation_pipeline_steps.
// No network calls — pure data parsing and field extraction.

use super::*;
use std::collections::HashMap;

fn make_peers(entries: &[(&str, &[(&str, &str)])]) -> HashMap<String, HashMap<String, String>> {
    entries
        .iter()
        .map(|(peer, fields)| {
            let field_map = fields
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            (peer.to_string(), field_map)
        })
        .collect()
}

#[test]
fn test_load_peer_config_valid_all_fields() {
    let peers = make_peers(&[(
        "m5max",
        &[
            ("ssh_alias", "mac-m5max"),
            ("tailscale_ip", "100.64.0.1"),
            ("user", "alice"),
        ],
    )]);

    let cfg = load_peer_config("m5max", &peers).unwrap();
    assert_eq!(cfg.name, "m5max");
    assert_eq!(cfg.ssh_alias, "mac-m5max");
    assert_eq!(cfg.tailscale_ip, "100.64.0.1");
    assert_eq!(cfg.user, "alice");
}

#[test]
fn test_load_peer_config_defaults_when_fields_missing() {
    // Only the peer entry exists — no sub-fields.
    let peers = make_peers(&[("m1pro", &[])]);

    let cfg = load_peer_config("m1pro", &peers).unwrap();
    assert_eq!(cfg.name, "m1pro");
    // ssh_alias falls back to peer name
    assert_eq!(cfg.ssh_alias, "m1pro");
    // tailscale_ip falls back to empty string
    assert_eq!(cfg.tailscale_ip, "");
    // user falls back to "roberdan"
    assert_eq!(cfg.user, "roberdan");
}

#[test]
fn test_load_peer_config_missing_peer_returns_err() {
    let peers = make_peers(&[("other-peer", &[])]);
    let result = load_peer_config("no-such-peer", &peers);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("no-such-peer"),
        "error must name the missing peer, got: {msg}"
    );
}

#[test]
fn test_load_peer_config_partial_fields() {
    // Only ssh_alias provided; tailscale_ip and user should use defaults.
    let peers = make_peers(&[("laptop", &[("ssh_alias", "my-laptop")])]);
    let cfg = load_peer_config("laptop", &peers).unwrap();
    assert_eq!(cfg.ssh_alias, "my-laptop");
    assert_eq!(cfg.tailscale_ip, "");
    assert_eq!(cfg.user, "roberdan");
}

#[test]
fn test_peer_config_fields_are_accessible() {
    // Verify PeerConfig struct fields are pub and carry correct values.
    let cfg = PeerConfig {
        name: "node-x".to_string(),
        ssh_alias: "alias-x".to_string(),
        tailscale_ip: "100.0.0.2".to_string(),
        user: "bob".to_string(),
    };
    assert_eq!(cfg.name, "node-x");
    assert_eq!(cfg.ssh_alias, "alias-x");
    assert_eq!(cfg.tailscale_ip, "100.0.0.2");
    assert_eq!(cfg.user, "bob");
}
