// LAN peer discovery via mDNS (zero-config networking).
// Uses mdns-sd for pure-Rust, cross-platform service advertisement + browsing.

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::HashSet;
use std::net::IpAddr;
use std::time::Duration;

/// mDNS service type for Convergio mesh peers.
pub const SERVICE_TYPE: &str = "_convergio._tcp.local.";

/// Default mesh port for mDNS-advertised services.
pub const DEFAULT_MESH_PORT: u16 = 9420;

/// A peer discovered via mDNS on the local network.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiscoveredPeer {
    pub name: String,
    pub ip: IpAddr,
    pub port: u16,
    pub version: String,
    pub role: String,
}

/// Register this node as a Convergio service on mDNS.
///
/// Advertises `_convergio._tcp.local.` with TXT records carrying
/// version, role, and node name so that other peers can identify us.
pub async fn register_service(
    node_name: &str,
    version: &str,
    role: &str,
    port: u16,
) -> Result<ServiceDaemon, mdns_sd::Error> {
    let mdns = ServiceDaemon::new()?;
    let properties = [
        ("version", version),
        ("role", role),
        ("name", node_name),
    ];
    let host = format!("{}.local.", node_name);
    let service = ServiceInfo::new(
        SERVICE_TYPE,
        node_name,
        &host,
        "",
        port,
        &properties[..],
    )?;
    mdns.register(service)?;
    tracing::info!(
        "[mesh] mDNS: registered {node_name} on port {port} (v{version}, {role})"
    );
    Ok(mdns)
}

/// Browse the local network for Convergio peers, collecting responses
/// for `timeout` duration.
pub async fn discover_peers(
    timeout: Duration,
) -> Result<Vec<DiscoveredPeer>, mdns_sd::Error> {
    let mdns = ServiceDaemon::new()?;
    let receiver = mdns.browse(SERVICE_TYPE)?;
    let mut peers = Vec::new();
    let mut seen = HashSet::new();
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, tokio::task::spawn_blocking({
            let rx = receiver.clone();
            move || rx.recv_timeout(Duration::from_millis(500))
        }))
        .await
        {
            Ok(Ok(Ok(ServiceEvent::ServiceResolved(info)))) => {
                if let Some(peer) = resolved_to_peer(&info) {
                    let key = (peer.name.clone(), peer.ip, peer.port);
                    if seen.insert(key) {
                        peers.push(peer);
                    }
                }
            }
            Ok(Ok(Ok(_))) => {
                // Other events (search started, removed, etc.) — skip.
            }
            Ok(Ok(Err(_))) | Ok(Err(_)) | Err(_) => {
                // Timeout or channel closed — continue until deadline.
            }
        }
    }

    let _ = mdns.shutdown();
    Ok(peers)
}

/// Background task: register this node and periodically discover peers.
///
/// Spawned once at startup when mesh transport is "lan" or discovery
/// is "mdns". Logs each newly-discovered peer.
pub async fn run_discovery_loop(
    node_name: String,
    version: String,
    role: String,
) {
    let port = DEFAULT_MESH_PORT;
    let _mdns = match register_service(&node_name, &version, &role, port).await {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("[mesh] mDNS registration failed: {e}");
            return;
        }
    };

    let mut known: HashSet<(String, IpAddr, u16)> = HashSet::new();
    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        interval.tick().await;
        match discover_peers(Duration::from_secs(3)).await {
            Ok(peers) => {
                for peer in peers {
                    if peer.name == node_name {
                        continue; // skip self
                    }
                    let key = (peer.name.clone(), peer.ip, peer.port);
                    if known.insert(key) {
                        tracing::info!(
                            "[mesh] Discovered: {} ({}) v{} [{}]",
                            peer.name,
                            peer.ip,
                            peer.version,
                            peer.role,
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!("[mesh] mDNS discovery error: {e}");
            }
        }
    }
}

/// Extract a `DiscoveredPeer` from a resolved mDNS service entry.
fn resolved_to_peer(info: &ServiceInfo) -> Option<DiscoveredPeer> {
    let ip = info.get_addresses().iter().next().copied()?;
    let port = info.get_port();
    let props = info.get_properties();
    let name = props
        .get("name")
        .map(|v| v.val_str().to_string())
        .unwrap_or_else(|| info.get_fullname().to_string());
    let version = props
        .get("version")
        .map(|v| v.val_str().to_string())
        .unwrap_or_default();
    let role = props
        .get("role")
        .map(|v| v.val_str().to_string())
        .unwrap_or_default();
    Some(DiscoveredPeer {
        name,
        ip,
        port,
        version,
        role,
    })
}

#[cfg(test)]
#[path = "lan_discovery_tests.rs"]
mod lan_discovery_tests;
