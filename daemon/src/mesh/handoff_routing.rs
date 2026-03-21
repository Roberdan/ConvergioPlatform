// Handoff routing: peer config parsing, sync source detection, CLI resolution, stale check.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerConfig {
    pub peer_name: String,
    pub ssh_alias: Option<String>,
    pub dns_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncSourceInfo {
    pub source: String,
    pub ssh_source: Option<String>,
    pub ssh_target: String,
    pub worktree: String,
    pub needs_stop: bool,
    pub needs_stash: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaleHostStatus {
    pub stale: bool,
    pub reason: String,
    pub can_recover: bool,
}

pub fn parse_peers_conf(content: &str) -> HashMap<String, PeerConfig> {
    let mut peers = HashMap::new();
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
            peers.insert(
                current.clone(),
                PeerConfig {
                    peer_name: current.clone(),
                    ssh_alias: None,
                    dns_name: None,
                },
            );
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if let Some(peer) = peers.get_mut(&current) {
                match k.trim() {
                    "ssh_alias" => peer.ssh_alias = Some(v.trim().to_string()),
                    "dns_name" => peer.dns_name = Some(v.trim().to_string()),
                    _ => {}
                }
            }
        }
    }
    peers
}

#[allow(clippy::too_many_arguments)]
pub fn detect_sync_source(
    target: &str,
    ssh_target: &str,
    local_hostname: &str,
    execution_host: &str,
    worktree: &str,
    plan_status: &str,
    in_progress_count: i64,
    peers: &HashMap<String, PeerConfig>,
) -> SyncSourceInfo {
    let host = execution_host.trim();
    if host.is_empty()
        || host.eq_ignore_ascii_case(local_hostname)
        || host
            .to_lowercase()
            .starts_with(&local_hostname.to_lowercase())
    {
        return SyncSourceInfo {
            source: "coordinator".to_string(),
            ssh_source: None,
            ssh_target: ssh_target.to_string(),
            worktree: worktree.to_string(),
            needs_stop: false,
            needs_stash: false,
        };
    }
    let target_peer = peers.get(target);
    let target_names = [
        Some(target.to_string()),
        target_peer.and_then(|p| p.ssh_alias.clone()),
        target_peer.and_then(|p| p.dns_name.clone()),
    ];
    if target_names
        .into_iter()
        .flatten()
        .any(|v| v.to_lowercase().contains(&host.to_lowercase()))
    {
        return SyncSourceInfo {
            source: "same_node".to_string(),
            ssh_source: Some(ssh_target.to_string()),
            ssh_target: ssh_target.to_string(),
            worktree: worktree.to_string(),
            needs_stop: false,
            needs_stash: false,
        };
    }
    let ssh_source = peers
        .iter()
        .find(|(_, p)| {
            [
                Some(p.peer_name.clone()),
                p.ssh_alias.clone(),
                p.dns_name.clone(),
            ]
            .into_iter()
            .flatten()
            .any(|name| name.to_lowercase().contains(&host.to_lowercase()))
        })
        .map(|(name, p)| p.ssh_alias.clone().unwrap_or_else(|| name.clone()));
    SyncSourceInfo {
        source: format!("worker:{host}"),
        ssh_source,
        ssh_target: ssh_target.to_string(),
        worktree: worktree.to_string(),
        needs_stop: plan_status == "doing" && in_progress_count > 0,
        needs_stash: true,
    }
}

pub fn resolve_cli_command(cli: &str, detections: &HashMap<String, String>) -> Option<String> {
    let map = HashMap::from([
        ("copilot", "copilot --yolo"),
        (
            "claude",
            "claude --dangerously-skip-permissions --model sonnet",
        ),
        ("opencode", "opencode"),
    ]);
    let picked = detections
        .get(cli)
        .cloned()
        .unwrap_or_else(|| "MISSING".to_string());
    if picked != "MISSING" {
        return Some(if picked == "gh-copilot" {
            "gh copilot -p".to_string()
        } else {
            map.get(cli).unwrap_or(&cli).to_string()
        });
    }
    for fb in ["copilot", "claude", "opencode"] {
        let v = detections
            .get(fb)
            .cloned()
            .unwrap_or_else(|| "MISSING".to_string());
        if v != "MISSING" {
            return Some(if v == "gh-copilot" {
                "gh copilot -p".to_string()
            } else {
                map.get(fb).unwrap_or(&fb).to_string()
            });
        }
    }
    None
}

pub fn check_stale_host(
    now_ts: u64,
    last_seen: Option<u64>,
    stale_threshold: u64,
    ssh_reachable: bool,
) -> StaleHostStatus {
    if let Some(ts) = last_seen {
        let age = now_ts.saturating_sub(ts);
        if age < stale_threshold {
            return StaleHostStatus {
                stale: false,
                reason: format!("heartbeat {age}s ago"),
                can_recover: false,
            };
        }
    }
    if ssh_reachable {
        StaleHostStatus {
            stale: true,
            reason: "heartbeat stale but SSH ok".to_string(),
            can_recover: true,
        }
    } else {
        StaleHostStatus {
            stale: true,
            reason: "heartbeat stale and SSH unreachable".to_string(),
            can_recover: false,
        }
    }
}
