// Network utilities: peers.conf parsing, WS detection, system stats

use super::types::DaemonConfig;
use crate::mesh::net::{load_tailscale_peer_ips, prefer_tailscale_peer_addr};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

pub const WS_BRAIN_ROUTE: &str = "/ws/brain";

pub fn parse_peers_conf(content: &str) -> Vec<String> {
    // Parse INI-style peers.conf: extract tailscale_ip from each [peer] section
    // and return as "ip:9420" entries for daemon TCP connections.
    let mut peers = Vec::new();
    let mut current_ip: Option<String> = None;
    for line in content.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(ip) = current_ip.take() {
                peers.push(format!("{ip}:9420"));
            }
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            if key.trim() == "tailscale_ip" {
                current_ip = Some(value.trim().to_string());
            }
        }
    }
    if let Some(ip) = current_ip {
        peers.push(format!("{ip}:9420"));
    }
    peers
}

pub fn is_ws_brain_request(request_head: &str) -> bool {
    request_head.starts_with("GET ") && request_head.contains(WS_BRAIN_ROUTE)
}

pub fn websocket_key(request: &str) -> Option<String> {
    request.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("sec-websocket-key") {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

pub fn read_peers_conf(path: &PathBuf) -> Vec<String> {
    fs::read_to_string(path)
        .ok()
        .map(|v| parse_peers_conf(&v))
        .unwrap_or_default()
}

pub fn load_peer_addrs(config: &DaemonConfig, bind_addr: &str) -> HashSet<String> {
    let tailscale_lookup = load_tailscale_peer_ips();
    read_peers_conf(&config.peers_conf_path)
        .into_iter()
        .map(|peer| prefer_tailscale_peer_addr(&peer, &tailscale_lookup))
        .filter(|p| p != bind_addr)
        .collect()
}

/// Resolve this node's friendly name from peers.conf by matching bind_ip
pub fn resolve_local_node_name(peers_conf_path: &std::path::Path, bind_ip: &str) -> String {
    if let Ok(content) = fs::read_to_string(peers_conf_path) {
        let mut section_name: Option<String> = None;
        for line in content.lines().map(str::trim) {
            if line.starts_with('[') && line.ends_with(']') {
                let name = line[1..line.len() - 1].to_string();
                if name == "mesh" {
                    section_name = None;
                    continue;
                }
                section_name = Some(name);
            } else if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "tailscale_ip" && value.trim() == bind_ip {
                    if let Some(name) = &section_name {
                        return name.clone();
                    }
                }
            }
        }
    }
    bind_ip.to_string()
}

/// Cross-platform system stats via sysinfo crate (macOS/Linux/Windows).
pub fn collect_system_stats() -> serde_json::Value {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_cpu_all();
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_cpu_all();
    sys.refresh_memory();
    let cpu = sys.global_cpu_usage() as f64;
    let mem_total_gb = sys.total_memory() as f64 / 1073741824.0;
    let mem_used_gb = sys.used_memory() as f64 / 1073741824.0;
    let (net_rx, net_tx) = collect_net_bytes();
    serde_json::json!({
        "cpu": (cpu * 10.0).round() / 10.0,
        "mem_total_gb": (mem_total_gb * 10.0).round() / 10.0,
        "mem_used_gb": (mem_used_gb * 10.0).round() / 10.0,
        "net_rx_bytes": net_rx,
        "net_tx_bytes": net_tx
    })
}

/// Cross-platform Tailscale interface byte counters via sysinfo::Networks.
/// Interface names: macOS=utun*, Linux=tailscale0, Windows=Tailscale
fn collect_net_bytes() -> (u64, u64) {
    use sysinfo::Networks;
    let networks = Networks::new_with_refreshed_list();
    for (name, data) in &networks {
        let n = name.to_lowercase();
        if n == "tailscale0" || n.contains("tailscale") {
            return (data.total_received(), data.total_transmitted());
        }
    }
    // macOS: utun interfaces — pick highest-traffic one (likely Tailscale)
    let mut best = (0u64, 0u64, 0u64);
    for (name, data) in &networks {
        if name.starts_with("utun") {
            let total = data.total_received() + data.total_transmitted();
            if total > best.2 {
                best = (data.total_received(), data.total_transmitted(), total);
            }
        }
    }
    if best.2 > 0 {
        return (best.0, best.1);
    }
    (0, 0)
}

pub fn detect_tailscale_ip() -> Option<String> {
    const CANDIDATES: &[&str] = &[
        "tailscale",
        "/usr/local/bin/tailscale",
        "/opt/homebrew/bin/tailscale",
        "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
    ];
    for cmd in CANDIDATES {
        if let Ok(output) = std::process::Command::new(cmd).arg("ip").arg("-4").output() {
            if output.status.success() {
                return String::from_utf8(output.stdout)
                    .ok()?
                    .lines()
                    .next()
                    .map(|line| line.trim().to_string());
            }
        }
    }
    None
}
