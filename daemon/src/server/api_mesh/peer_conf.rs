//! peers.conf parsing and local identity detection utilities.
use std::collections::HashMap;

pub(crate) fn parse_peers_conf(content: &str) -> HashMap<String, HashMap<String, String>> {
    let mut peers: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current = String::new();
    for raw in content.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current = line
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string();
            // Skip [mesh] section — it's config, not a peer node
            if current == "mesh" {
                current.clear();
                continue;
            }
            peers.insert(current.clone(), HashMap::new());
            continue;
        }
        if current.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if let Some(fields) = peers.get_mut(&current) {
                fields.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    peers
}

pub(crate) fn detect_local_identity() -> (String, String) {
    // hostname: cross-platform via gethostname or fallback
    let hostname = {
        #[cfg(unix)]
        {
            std::process::Command::new("hostname")
                .arg("-s")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_lowercase())
                .unwrap_or_default()
        }
        #[cfg(windows)]
        {
            std::env::var("COMPUTERNAME")
                .unwrap_or_default()
                .to_lowercase()
        }
        #[cfg(not(any(unix, windows)))]
        {
            String::new()
        }
    };
    // Tailscale IP: try multiple binary locations
    let ts_candidates = &[
        "tailscale",
        "/usr/local/bin/tailscale",
        "/opt/homebrew/bin/tailscale",
        "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
        "C:\\Program Files\\Tailscale\\tailscale.exe",
    ];
    let ts_ip = ts_candidates
        .iter()
        .find_map(|cmd| {
            std::process::Command::new(cmd)
                .args(["ip", "-4"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_default();
    (hostname, ts_ip)
}

pub(crate) fn is_local_peer_conf(
    hostname: &str,
    ts_ip: &str,
    fields: &HashMap<String, String>,
) -> bool {
    // Match by Tailscale IP (most reliable)
    if !ts_ip.is_empty() {
        if let Some(peer_ip) = fields.get("tailscale_ip") {
            if peer_ip == ts_ip {
                return true;
            }
        }
    }
    // Match by hostname substring in dns_name or ssh_alias
    if !hostname.is_empty() {
        for key in &["dns_name", "ssh_alias"] {
            if let Some(val) = fields.get(*key) {
                let val_l = val.to_lowercase();
                if val_l.contains(hostname) || hostname.contains(&val_l) {
                    return true;
                }
            }
        }
    }
    false
}

pub(crate) fn build_ip_name_map(conf: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_name = String::new();
    for line in conf.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            current_name = trimmed[1..trimmed.len() - 1].to_string();
        } else if trimmed.starts_with("tailscale_ip=")
            && !current_name.is_empty()
            && current_name != "mesh"
        {
            let ip = trimmed.trim_start_matches("tailscale_ip=").trim();
            map.insert(format!("{ip}:9420"), current_name.clone());
        }
    }
    map
}

pub(crate) fn detect_local_node(conf: &str) -> String {
    // Use Tailscale IP as the most reliable cross-platform identifier
    let (_, ts_ip) = detect_local_identity();
    if !ts_ip.is_empty() {
        let mut current_name = String::new();
        for line in conf.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_name = trimmed[1..trimmed.len() - 1].to_string();
            } else if trimmed.starts_with("tailscale_ip=")
                && !current_name.is_empty()
                && current_name != "mesh"
            {
                let ip = trimmed.trim_start_matches("tailscale_ip=").trim();
                if ip == ts_ip {
                    return current_name;
                }
            }
        }
    }
    "unknown".to_string()
}
