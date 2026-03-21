// Network utilities for mesh setup.
//
// Wraps OS tools (tailscale, ssh-keygen) via std::process::Command.
// All functions are fail-loud — they return Err on unexpected output rather
// than silently swallowing failures.

pub mod ssh_keys;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use ssh_keys::{enable_screen_sharing, generate_ssh_keypair, ScreenSharingStatus};

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("command '{cmd}' failed (exit {code}): {stderr}")]
    CommandFailed {
        cmd: String,
        code: i32,
        stderr: String,
    },
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported OS: {0}")]
    UnsupportedOs(String),
    #[error("{0}")]
    Other(String),
}

// ── Tailscale ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TailscaleStatus {
    pub self_ip: String,
    pub self_name: String,
    pub peers: Vec<TailscalePeer>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TailscalePeer {
    pub hostname: String,
    pub ip: String,
    pub online: bool,
    pub os: String,
}

// Raw shape returned by `tailscale status --json`.
#[derive(Debug, Deserialize)]
struct TailscaleRaw {
    #[serde(rename = "Self")]
    self_node: TailscaleSelf,
    #[serde(rename = "Peer")]
    peer_map: Option<std::collections::HashMap<String, TailscalePeerRaw>>,
}

#[derive(Debug, Deserialize)]
struct TailscaleSelf {
    #[serde(rename = "TailscaleIPs")]
    tailscale_ips: Vec<String>,
    #[serde(rename = "HostName")]
    host_name: String,
}

#[derive(Debug, Deserialize)]
struct TailscalePeerRaw {
    #[serde(rename = "HostName")]
    host_name: String,
    #[serde(rename = "TailscaleIPs")]
    tailscale_ips: Vec<String>,
    #[serde(rename = "Online")]
    online: bool,
    #[serde(rename = "OS")]
    os: String,
}

/// Run `tailscale status --json` and return a normalised `TailscaleStatus`.
pub fn tailscale_status() -> Result<TailscaleStatus, NetworkError> {
    let out = std::process::Command::new("tailscale")
        .args(["status", "--json"])
        .output()?;

    if !out.status.success() {
        return Err(NetworkError::CommandFailed {
            cmd: "tailscale status --json".to_owned(),
            code: out.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        });
    }

    parse_tailscale_json(&out.stdout)
}

/// Parse raw tailscale JSON bytes — separated for unit testing with mock data.
pub fn parse_tailscale_json(json: &[u8]) -> Result<TailscaleStatus, NetworkError> {
    let raw: TailscaleRaw = serde_json::from_slice(json)?;

    let self_ip = raw
        .self_node
        .tailscale_ips
        .first()
        .cloned()
        .unwrap_or_default();

    let peers = raw
        .peer_map
        .unwrap_or_default()
        .into_values()
        .map(|p| TailscalePeer {
            hostname: p.host_name,
            ip: p.tailscale_ips.into_iter().next().unwrap_or_default(),
            online: p.online,
            os: p.os,
        })
        .collect();

    Ok(TailscaleStatus {
        self_ip,
        self_name: raw.self_node.host_name,
        peers,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const TAILSCALE_JSON: &str = r#"{
      "Self": {
        "HostName": "coordinator-1",
        "TailscaleIPs": ["100.64.0.10", "fd7a:115c:a1e0::1"],
        "OS": "macos"
      },
      "Peer": {
        "abc123": {
          "HostName": "linux-worker",
          "TailscaleIPs": ["100.64.0.2"],
          "Online": true,
          "OS": "linux"
        },
        "def456": {
          "HostName": "mac-worker-1",
          "TailscaleIPs": ["100.64.0.1"],
          "Online": false,
          "OS": "macos"
        }
      }
    }"#;

    #[test]
    fn parse_tailscale_self_fields() {
        let status = parse_tailscale_json(TAILSCALE_JSON.as_bytes()).unwrap();
        assert_eq!(status.self_ip, "100.64.0.10");
        assert_eq!(status.self_name, "coordinator-1");
    }

    #[test]
    fn parse_tailscale_peer_count() {
        let status = parse_tailscale_json(TAILSCALE_JSON.as_bytes()).unwrap();
        assert_eq!(status.peers.len(), 2);
    }

    #[test]
    fn parse_tailscale_peer_online_flags() {
        let status = parse_tailscale_json(TAILSCALE_JSON.as_bytes()).unwrap();

        let linux_worker = status
            .peers
            .iter()
            .find(|p| p.hostname == "linux-worker")
            .unwrap();
        assert!(linux_worker.online);
        assert_eq!(linux_worker.ip, "100.64.0.2");
        assert_eq!(linux_worker.os, "linux");

        let m1 = status
            .peers
            .iter()
            .find(|p| p.hostname == "mac-worker-1")
            .unwrap();
        assert!(!m1.online);
    }

    #[test]
    fn parse_tailscale_no_peers() {
        let json = r#"{"Self":{"HostName":"solo","TailscaleIPs":["100.1.1.1"],"OS":"linux"}}"#;
        let status = parse_tailscale_json(json.as_bytes()).unwrap();
        assert_eq!(status.peers.len(), 0);
    }

    #[test]
    fn parse_tailscale_invalid_json_errors() {
        let result = parse_tailscale_json(b"not json");
        assert!(result.is_err());
    }
}
