// Split from daemon_tests.rs — basic parsing + WS + net tests.
// Relay and resilience tests live in test_relay.rs.

#[cfg(test)]
mod test_relay;

use super::{is_ws_brain_request, parse_peers_conf, websocket_key};
use crate::mesh::net::{mesh_socket_tuning, prefer_tailscale_peer_addr};
use std::collections::HashMap;
use std::net::SocketAddr;

#[test]
fn parses_peers_file_and_skips_comments() {
    let ini = "\n# primary peers\n[peer1]\ntailscale_ip=100.101.102.10\n\n[peer2]\ntailscale_ip=100.101.102.11\n";
    let peers = parse_peers_conf(ini);
    assert_eq!(peers, vec!["100.101.102.10:9420", "100.101.102.11:9420"]);
}

#[test]
fn detects_ws_brain_upgrade_path() {
    assert!(is_ws_brain_request(
        "GET /ws/brain HTTP/1.1\r\nUpgrade: websocket\r\n"
    ));
    assert!(!is_ws_brain_request(
        "GET /ws/other HTTP/1.1\r\nUpgrade: websocket\r\n"
    ));
}

#[test]
fn extracts_sec_websocket_key() {
    let req = "GET /ws/brain HTTP/1.1\r\nSec-WebSocket-Key: abc123==\r\n\r\n";
    assert_eq!(websocket_key(req).as_deref(), Some("abc123=="));
}

#[test]
fn perf_prefers_tailscale_ip_over_dns_host() {
    let mut lookup = HashMap::new();
    lookup.insert("peer-a.mesh".to_string(), "100.82.10.4".to_string());
    let peer = prefer_tailscale_peer_addr("peer-a.mesh:9420", &lookup);
    assert_eq!(peer, "100.82.10.4:9420");
}

#[test]
fn perf_socket_tuning_enables_nodelay_and_keepalive() {
    let tuning = mesh_socket_tuning();
    assert!(tuning.nodelay);
    assert_eq!(tuning.keepalive_idle_secs, 30);
    assert_eq!(tuning.keepalive_interval_secs, 10);
}

// === W7: Node failure & resilience tests ===

#[test]
fn peers_conf_empty_returns_empty_vec() {
    let peers = parse_peers_conf("");
    assert!(peers.is_empty());
}

#[test]
fn peers_conf_only_comments() {
    let ini = "# comment\n# another comment\n";
    let peers = parse_peers_conf(ini);
    assert!(peers.is_empty());
}

#[test]
fn peers_conf_malformed_no_ip() {
    let ini = "[peer1]\nname=test\n";
    let peers = parse_peers_conf(ini);
    assert!(
        peers.is_empty(),
        "peer without tailscale_ip should be skipped"
    );
}

#[test]
fn inbound_rate_limiter_rejects_excessive_connections_from_single_ip() {
    let limiter = super::InboundConnectionRateLimiter::new(10, 100);
    let remote: SocketAddr = "100.64.0.10:9420".parse().expect("valid socket addr");

    for _ in 0..10 {
        limiter
            .check(remote)
            .expect("first 10 connections accepted");
    }

    let err = limiter
        .check(remote)
        .expect_err("11th connection in one second should be rejected");
    assert!(err.contains("per-second"), "unexpected error: {err}");
}

#[test]
fn inbound_rate_limiter_accepts_legitimate_connection_rate() {
    let limiter = super::InboundConnectionRateLimiter::new(10, 100);
    let remote: SocketAddr = "100.64.0.20:9420".parse().expect("valid socket addr");

    for _ in 0..5 {
        limiter
            .check(remote)
            .expect("connection should be accepted");
    }
}
