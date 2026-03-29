use super::*;
use std::collections::HashMap;

#[test]
fn mesh_socket_tuning_defaults() {
    let tuning = mesh_socket_tuning();
    assert!(tuning.nodelay);
    assert_eq!(tuning.keepalive_idle_secs, 30);
    assert_eq!(tuning.keepalive_interval_secs, 10);
}

#[test]
fn prefer_tailscale_resolves_hostname_to_ip() {
    let lookup = HashMap::from([
        ("worker-1".to_string(), "100.64.0.1".to_string()),
        ("worker-2.ts.net".to_string(), "100.64.0.2".to_string()),
    ]);
    assert_eq!(
        prefer_tailscale_peer_addr("worker-1:9420", &lookup),
        "100.64.0.1:9420"
    );
}

#[test]
fn prefer_tailscale_passes_through_ip_addr() {
    let lookup = HashMap::from([("worker-1".to_string(), "100.64.0.1".to_string())]);
    // Already an IP — no change
    assert_eq!(
        prefer_tailscale_peer_addr("192.168.1.1:9420", &lookup),
        "192.168.1.1:9420"
    );
}

#[test]
fn prefer_tailscale_passes_through_unknown() {
    let lookup = HashMap::new();
    assert_eq!(
        prefer_tailscale_peer_addr("unknown-host:8420", &lookup),
        "unknown-host:8420"
    );
}

#[test]
fn prefer_tailscale_strips_trailing_dot() {
    let lookup = HashMap::from([("my-node".to_string(), "100.1.2.3".to_string())]);
    assert_eq!(
        prefer_tailscale_peer_addr("my-node.:9420", &lookup),
        "100.1.2.3:9420"
    );
}

#[test]
fn prefer_tailscale_no_port_returns_unchanged() {
    let lookup = HashMap::from([("host".to_string(), "100.1.2.3".to_string())]);
    // No ':' port separator — cannot split, returns as-is
    assert_eq!(prefer_tailscale_peer_addr("justhost", &lookup), "justhost");
}

#[test]
fn split_host_port_valid() {
    assert_eq!(split_host_port("host:1234"), Some(("host", "1234")));
    assert_eq!(
        split_host_port("100.64.0.1:9420"),
        Some(("100.64.0.1", "9420"))
    );
}

#[test]
fn split_host_port_empty_parts() {
    assert_eq!(split_host_port(":1234"), None); // empty host
    assert_eq!(split_host_port("host:"), None); // empty port
    assert_eq!(split_host_port("noport"), None);
}

#[test]
fn split_host_port_multiple_colons() {
    // IPv6-like: rsplit_once picks the last ':'
    assert_eq!(
        split_host_port("[::1]:8080"),
        Some(("[::1]", "8080"))
    );
}
