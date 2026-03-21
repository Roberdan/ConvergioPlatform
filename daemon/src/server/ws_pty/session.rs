//! Peer resolution and PTY session utilities for ws_pty.
use super::super::state::ServerState;
use std::collections::HashSet;

pub(super) const MAX_TMUX_SESSION_LEN: usize = 64;
pub(super) const MAX_PEER_LEN: usize = 128;

pub(super) fn parse_known_peers(content: &str) -> HashSet<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('[') && line.ends_with(']'))
        .map(|line| line[1..line.len() - 1].trim().to_string())
        .filter(|name| !name.is_empty() && name != "mesh")
        .collect()
}

pub(super) fn load_known_peers() -> HashSet<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let path = std::path::PathBuf::from(home).join(".claude/config/peers.conf");
    let Ok(content) = std::fs::read_to_string(path) else {
        return HashSet::new();
    };
    parse_known_peers(&content)
}

pub(crate) fn validate_peer(peer: &str, known_peers: &HashSet<String>) -> Result<(), String> {
    if peer.len() > MAX_PEER_LEN {
        return Err(format!("peer exceeds max length ({MAX_PEER_LEN})"));
    }
    if peer == "local" || peer == "localhost" {
        return Ok(());
    }
    if known_peers.contains(peer) {
        Ok(())
    } else {
        Err(format!("unknown peer: {peer}"))
    }
}

pub(super) fn read_peer_conf(peer: &str) -> Option<std::collections::HashMap<String, String>> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let path = std::path::PathBuf::from(home).join(".claude/config/peers.conf");
    let text = std::fs::read_to_string(path).ok()?;
    let mut found = false;
    let mut map = std::collections::HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if found {
                break;
            }
            found = &line[1..line.len() - 1] == peer;
            continue;
        }
        if found {
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    if found { Some(map) } else { None }
}

pub(crate) fn tailscale_resolve(peer: &str) -> Option<(String, bool, bool)> {
    let output = std::process::Command::new("tailscale")
        .args(["status", "--json"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let conf = read_peer_conf(peer);
    let conf_ip = conf.as_ref().and_then(|c| c.get("tailscale_ip").cloned());
    let conf_dns = conf.as_ref().and_then(|c| c.get("dns_name").cloned());
    if let Some(self_node) = json.get("Self") {
        if ts_node_matches(self_node, conf_ip.as_deref(), conf_dns.as_deref(), peer) {
            return Some((ts_first_ip(self_node)?, true, true));
        }
    }
    if let Some(peers) = json.get("Peer").and_then(|p| p.as_object()) {
        for (_key, node) in peers {
            if ts_node_matches(node, conf_ip.as_deref(), conf_dns.as_deref(), peer) {
                let ip = ts_first_ip(node)?;
                let online = node
                    .get("Online")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                return Some((ip, online, false));
            }
        }
    }
    None
}

fn ts_node_matches(
    node: &serde_json::Value,
    conf_ip: Option<&str>,
    conf_dns: Option<&str>,
    peer: &str,
) -> bool {
    if let Some(ip) = conf_ip {
        if let Some(ips) = node.get("TailscaleIPs").and_then(|v| v.as_array()) {
            if ips.iter().any(|v| v.as_str() == Some(ip)) {
                return true;
            }
        }
    }
    if let Some(dns) = conf_dns {
        if let Some(node_dns) = node.get("DNSName").and_then(|v| v.as_str()) {
            let (a, b) = (dns.trim_end_matches('.'), node_dns.trim_end_matches('.'));
            if a.eq_ignore_ascii_case(b) {
                return true;
            }
        }
    }
    ts_name_matches(
        node,
        &peer.to_lowercase().replace(['-', '_', ' ', '\''], ""),
    )
}

pub(crate) fn ts_name_matches(node: &serde_json::Value, normalized_peer: &str) -> bool {
    let peer_norm = normalized_peer
        .to_lowercase()
        .replace(['-', '_', ' ', '\'', '.'], "");
    for field in ["HostName", "DNSName"] {
        if let Some(val) = node.get(field).and_then(|v| v.as_str()) {
            let norm = val.to_lowercase().replace(['-', '_', ' ', '\'', '.'], "");
            if norm.contains(&peer_norm)
                || peer_norm.contains(
                    norm.split("tail")
                        .next()
                        .unwrap_or("")
                        .trim_end_matches('.'),
                )
            {
                return true;
            }
        }
    }
    false
}

pub(crate) fn ts_first_ip(node: &serde_json::Value) -> Option<String> {
    node.get("TailscaleIPs")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

pub(crate) fn peer_ssh_user(peer: &str) -> Option<String> {
    read_peer_conf(peer).and_then(|m| m.get("user").cloned())
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedPeer {
    pub ip: String,
    pub user: Option<String>,
    pub is_self: bool,
}

pub(crate) fn resolve_peer(_state: &ServerState, peer: &str) -> Option<ResolvedPeer> {
    let (ip, _online, is_self) = tailscale_resolve(peer)?;
    Some(ResolvedPeer {
        ip,
        user: peer_ssh_user(peer),
        is_self,
    })
}

pub(crate) fn peer_ssh_alias(state: &ServerState, peer: &str) -> Option<String> {
    let resolved = resolve_peer(state, peer)?;
    if resolved.is_self {
        return None;
    }
    match resolved.user {
        Some(u) => Some(format!("{u}@{}", resolved.ip)),
        None => Some(resolved.ip),
    }
}

pub(crate) fn is_local_peer(state: &ServerState, peer: &str) -> bool {
    if peer == "local" || peer == "localhost" {
        return true;
    }
    matches!(resolve_peer(state, peer), Some(r) if r.is_self)
}
