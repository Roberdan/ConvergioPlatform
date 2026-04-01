// Tests for LAN peer discovery via mDNS.

use super::*;
use std::net::{IpAddr, Ipv4Addr};

#[test]
fn discovered_peer_fields() {
    let peer = DiscoveredPeer {
        name: "m5max".to_string(),
        ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42)),
        port: 9420,
        version: "20.4.0".to_string(),
        role: "coordinator".to_string(),
    };
    assert_eq!(peer.name, "m5max");
    assert_eq!(peer.port, DEFAULT_MESH_PORT);
    assert_eq!(peer.version, "20.4.0");
    assert_eq!(peer.role, "coordinator");
}

#[test]
fn service_type_is_well_formed() {
    assert!(SERVICE_TYPE.starts_with('_'));
    assert!(SERVICE_TYPE.ends_with(".local."));
    assert!(SERVICE_TYPE.contains("._tcp"));
}

#[test]
fn default_mesh_port_value() {
    assert_eq!(DEFAULT_MESH_PORT, 9420);
}

#[test]
fn discovered_peer_equality() {
    let a = DiscoveredPeer {
        name: "node-a".to_string(),
        ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
        port: 9420,
        version: "1.0.0".to_string(),
        role: "worker".to_string(),
    };
    let b = a.clone();
    assert_eq!(a, b);
}

#[test]
fn discovered_peer_hash_dedup() {
    use std::collections::HashSet;
    let peer = DiscoveredPeer {
        name: "node-x".to_string(),
        ip: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
        port: 9420,
        version: "20.4.0".to_string(),
        role: "standalone".to_string(),
    };
    let mut set = HashSet::new();
    set.insert(peer.clone());
    set.insert(peer.clone());
    assert_eq!(set.len(), 1);
}

/// Integration test: register + discover roundtrip on real mDNS.
/// Ignored by default — requires mDNS multicast support (not available
/// in many CI environments).
#[tokio::test]
#[ignore]
async fn register_and_discover_roundtrip() {
    let _mdns = register_service("test-node", "0.0.1", "worker", 9420)
        .await
        .expect("register should succeed");

    // Give mDNS a moment to propagate.
    tokio::time::sleep(Duration::from_secs(2)).await;

    let peers = discover_peers(Duration::from_secs(3))
        .await
        .expect("discover should succeed");

    let found = peers.iter().any(|p| p.name == "test-node");
    assert!(found, "should discover our own registered service");
}
