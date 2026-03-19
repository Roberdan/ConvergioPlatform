// Network utilities for mesh setup.
//
// Wraps OS tools (tailscale, ssh-keygen) via std::process::Command.
// All functions are fail-loud — they return Err on unexpected output rather
// than silently swallowing failures.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

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

// ── SSH key generation ────────────────────────────────────────────────────────

/// Generate an ed25519 keypair at `path` (private) and `path.pub` (public).
/// Returns Err if the key already exists to avoid silent overwrites.
pub fn generate_ssh_keypair(path: &Path) -> Result<(), NetworkError> {
    if path.exists() {
        return Err(NetworkError::Other(format!(
            "key already exists: {}",
            path.display()
        )));
    }

    let path_str = path
        .to_str()
        .ok_or_else(|| NetworkError::Other("key path is not valid UTF-8".to_owned()))?;

    let out = std::process::Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-f", path_str])
        .output()?;

    if !out.status.success() {
        return Err(NetworkError::CommandFailed {
            cmd: "ssh-keygen".to_owned(),
            code: out.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        });
    }

    Ok(())
}

// ── Screen sharing detection ──────────────────────────────────────────────────

#[derive(Debug, PartialEq)]
pub enum ScreenSharingStatus {
    Enabled,
    Disabled,
    Unknown,
}

/// Detect remote-desktop/screen-sharing status without modifying system state.
/// macOS: checks ARD via `launchctl list`.
/// Linux: checks for xrdp or wayvnc via `pgrep`.
/// Windows: returns Unknown (detection not implemented).
pub fn enable_screen_sharing(os: &str) -> Result<ScreenSharingStatus, NetworkError> {
    match os {
        "macos" => {
            let out = std::process::Command::new("launchctl")
                .args(["list", "com.apple.screensharing"])
                .output()?;
            if out.status.success() {
                Ok(ScreenSharingStatus::Enabled)
            } else {
                Ok(ScreenSharingStatus::Disabled)
            }
        }
        "linux" => {
            for svc in ["xrdp", "wayvnc"] {
                let out = std::process::Command::new("pgrep")
                    .arg("-x")
                    .arg(svc)
                    .output()?;
                if out.status.success() {
                    return Ok(ScreenSharingStatus::Enabled);
                }
            }
            Ok(ScreenSharingStatus::Disabled)
        }
        "windows" => Ok(ScreenSharingStatus::Unknown),
        other => Err(NetworkError::UnsupportedOs(other.to_owned())),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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

    #[test]
    fn ssh_keygen_creates_keypair() {
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("id_ed25519");
        generate_ssh_keypair(&key_path).unwrap();
        assert!(key_path.exists(), "private key file must exist");
        let pub_path = dir.path().join("id_ed25519.pub");
        assert!(pub_path.exists(), "public key file must exist");
    }

    #[test]
    fn ssh_keygen_refuses_overwrite() {
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("id_ed25519");
        generate_ssh_keypair(&key_path).unwrap();
        let result = generate_ssh_keypair(&key_path);
        assert!(result.is_err(), "second call must fail — key exists");
    }
}
