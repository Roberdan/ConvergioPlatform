// Centralized peer resolver: single source of truth for peer name -> connection details.
// Fixes B6 (SSH doesn't resolve peers from peers.conf) and B9 (5 names for 2 machines).

use super::peers::{PeerConfig, PeersError, PeersRegistry};
use std::path::{Path, PathBuf};

pub const DEFAULT_SSH_PORT: u16 = 22;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPeer {
    pub canonical_name: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub ssh_alias: String,
    pub tailscale_ip: String,
    pub thunderbolt_ip: Option<String>,
    pub lan_ip: Option<String>,
    pub transport: String,
}

/// Normalize a peer name by lowercasing and stripping separators for fuzzy matching.
pub fn normalize_name(name: &str) -> String {
    name.to_lowercase()
        .replace(['-', '_', ' ', '\'', '.'], "")
}

/// Resolve any peer name/alias to canonical connection details.
/// Reads `~/.claude/config/peers.conf`.
pub fn resolve(peer_name: &str) -> Result<ResolvedPeer, PeersError> {
    let path = default_peers_conf_path();
    resolve_with_conf(peer_name, &path)
}

/// Resolve using an explicit peers.conf path.
pub fn resolve_with_conf(peer_name: &str, conf_path: &Path) -> Result<ResolvedPeer, PeersError> {
    let registry = PeersRegistry::load(conf_path)?;
    resolve_from_registry(peer_name, &registry)
}

/// Resolve from an already-loaded registry (no filesystem I/O).
/// Uses probe-based transport selection: Thunderbolt > LAN > Tailscale > static fallback.
pub fn resolve_from_registry(
    peer_name: &str,
    registry: &PeersRegistry,
) -> Result<ResolvedPeer, PeersError> {
    let (canonical, config) = find_peer(peer_name, registry)?;
    let (host, transport) = select_host_with_probe(&config);
    Ok(ResolvedPeer {
        canonical_name: canonical,
        host,
        port: DEFAULT_SSH_PORT,
        user: config.user.clone(),
        ssh_alias: config.ssh_alias.clone(),
        tailscale_ip: config.tailscale_ip.clone(),
        thunderbolt_ip: config.thunderbolt_ip.clone(),
        lan_ip: config.lan_ip.clone(),
        transport,
    })
}

/// Probe-based transport selection: try each IP with a 1s TCP connect.
/// Order: Thunderbolt (fastest) > LAN (local) > Tailscale (remote) > static fallback.
fn select_host_with_probe(config: &PeerConfig) -> (String, String) {
    use std::net::TcpStream;
    use std::time::Duration;
    let candidates: Vec<(&str, &str)> = [
        ("thunderbolt", config.thunderbolt_ip.as_deref().unwrap_or("")),
        ("lan", config.lan_ip.as_deref().unwrap_or("")),
        ("tailscale", &config.tailscale_ip),
    ]
    .into_iter()
    .filter(|(_, ip)| !ip.is_empty())
    .collect();

    for (transport, ip) in &candidates {
        let addr = format!("{ip}:22");
        if let Ok(a) = addr.parse() {
            if TcpStream::connect_timeout(&a, Duration::from_secs(1)).is_ok() {
                return (ip.to_string(), transport.to_string());
            }
        }
    }
    // All probes failed — static fallback (ssh_alias > tailscale > dns)
    let host = if !config.ssh_alias.is_empty() { config.ssh_alias.clone() }
        else if !config.tailscale_ip.is_empty() { config.tailscale_ip.clone() }
        else if !config.dns_name.is_empty() { config.dns_name.clone() }
        else { "localhost".to_string() };
    (host, "fallback".to_string())
}

/// Find peer in registry: exact name -> case-insensitive name -> alias/IP/DNS fuzzy match.
fn find_peer(name: &str, registry: &PeersRegistry) -> Result<(String, PeerConfig), PeersError> {
    // 1. Exact match on section name
    if let Some(config) = registry.peers.get(name) {
        return Ok((name.to_string(), config.clone()));
    }
    // 2. Case-insensitive match on section name
    let name_lower = name.to_lowercase();
    for (key, config) in &registry.peers {
        if key.to_lowercase() == name_lower {
            return Ok((key.clone(), config.clone()));
        }
    }
    // 3. Match against fields: tailscale_ip, ssh_alias, dns_name
    let normalized = normalize_name(name);
    for (key, config) in &registry.peers {
        if config.tailscale_ip == name {
            return Ok((key.clone(), config.clone()));
        }
        if !config.ssh_alias.is_empty() && normalize_name(&config.ssh_alias) == normalized {
            return Ok((key.clone(), config.clone()));
        }
        let dns_norm = normalize_name(&config.dns_name);
        if !dns_norm.is_empty() && dns_norm.contains(&normalized) {
            return Ok((key.clone(), config.clone()));
        }
    }
    Err(PeersError::NotFound(name.to_string()))
}

/// Build SSH destination string from resolved peer.
/// Uses the probed host IP (not ssh_alias) so it works even when Tailscale is down.
pub fn ssh_destination(resolved: &ResolvedPeer) -> String {
    if !resolved.user.is_empty() {
        format!("{}@{}", resolved.user, resolved.host)
    } else {
        resolved.host.clone()
    }
}

/// Normalize peer name to canonical form from peers.conf.
/// Returns the input unchanged if not found in registry.
pub fn canonicalize(peer_name: &str, conf_path: &Path) -> String {
    resolve_with_conf(peer_name, conf_path)
        .map(|r| r.canonical_name)
        .unwrap_or_else(|_| peer_name.to_string())
}

fn default_peers_conf_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".claude/config/peers.conf")
}

#[cfg(test)]
#[path = "peer_resolver_tests.rs"]
mod tests;
