// INI-format parser/writer compatible with ~/.claude/config/peers.conf
//
// Format:
//   [section_name]
//   key=value
//
// Sections: [mesh] for global config, [peer_name] for each peer.
// Comment lines start with '#'. Blank lines are ignored.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PeersError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("missing required field '{field}' in peer '{peer}'")]
    MissingField { peer: String, field: String },
    #[error("peer '{0}' not found")]
    NotFound(String),
    #[error("parse error at line {line}: {msg}")]
    Parse { line: usize, msg: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeerConfig {
    pub ssh_alias: String,
    pub user: String,
    pub os: String,
    pub tailscale_ip: String,
    pub dns_name: String,
    pub capabilities: Vec<String>,
    pub role: String,
    pub status: String,
    pub mac_address: Option<String>,
    pub gh_account: Option<String>,
    pub runners: Option<u32>,
    pub runner_paths: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PeersRegistry {
    pub shared_secret: String,
    pub peers: BTreeMap<String, PeerConfig>,
}

// ── Parsing helpers ──────────────────────────────────────────────────────────

fn require(map: &BTreeMap<String, String>, key: &str, peer: &str) -> Result<String, PeersError> {
    map.get(key)
        .cloned()
        .ok_or_else(|| PeersError::MissingField {
            peer: peer.to_owned(),
            field: key.to_owned(),
        })
}

fn parse_capabilities(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

fn build_peer(name: &str, kv: &BTreeMap<String, String>) -> Result<PeerConfig, PeersError> {
    Ok(PeerConfig {
        ssh_alias: require(kv, "ssh_alias", name)?,
        user: require(kv, "user", name)?,
        os: require(kv, "os", name)?,
        tailscale_ip: require(kv, "tailscale_ip", name)?,
        dns_name: require(kv, "dns_name", name)?,
        capabilities: parse_capabilities(&require(kv, "capabilities", name)?),
        role: require(kv, "role", name)?,
        status: kv
            .get("status")
            .cloned()
            .unwrap_or_else(|| "active".to_owned()),
        mac_address: kv.get("mac_address").cloned(),
        gh_account: kv.get("gh_account").cloned(),
        runners: kv.get("runners").and_then(|v| v.parse::<u32>().ok()),
        runner_paths: kv.get("runner_paths").cloned(),
    })
}

// ── INI parser ────────────────────────────────────────────────────────────────

fn flush_section(
    section: &Option<String>,
    kv: &BTreeMap<String, String>,
    secret: &mut String,
    peers: &mut BTreeMap<String, PeerConfig>,
) -> Result<(), PeersError> {
    if let Some(name) = section {
        if name == "mesh" {
            if let Some(s) = kv.get("shared_secret") {
                *secret = s.clone();
            }
        } else {
            let cfg = build_peer(name, kv)?;
            peers.insert(name.clone(), cfg);
        }
    }
    Ok(())
}

fn parse_ini(text: &str) -> Result<(String, BTreeMap<String, PeerConfig>), PeersError> {
    let mut shared_secret = String::new();
    let mut peers: BTreeMap<String, PeerConfig> = BTreeMap::new();
    let mut current_section: Option<String> = None;
    let mut current_kv: BTreeMap<String, String> = BTreeMap::new();

    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            flush_section(
                &current_section,
                &current_kv,
                &mut shared_secret,
                &mut peers,
            )?;
            current_section = Some(line[1..line.len() - 1].to_owned());
            current_kv = BTreeMap::new();
        } else if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_owned();
            let val = line[eq + 1..].trim().to_owned();
            current_kv.insert(key, val);
        } else {
            return Err(PeersError::Parse {
                line: lineno + 1,
                msg: format!("unexpected content: {line}"),
            });
        }
    }
    flush_section(
        &current_section,
        &current_kv,
        &mut shared_secret,
        &mut peers,
    )?;
    Ok((shared_secret, peers))
}

// ── Serialiser ────────────────────────────────────────────────────────────────

fn caps_str(caps: &[String]) -> String {
    caps.join(",")
}

fn peer_to_ini(name: &str, p: &PeerConfig) -> String {
    let mut out = format!(
        "[{name}]\nssh_alias={}\nuser={}\nos={}\ntailscale_ip={}\ndns_name={}\ncapabilities={}\nrole={}\nstatus={}\n",
        p.ssh_alias,
        p.user,
        p.os,
        p.tailscale_ip,
        p.dns_name,
        caps_str(&p.capabilities),
        p.role,
        p.status,
    );
    if let Some(ref mac) = p.mac_address {
        out.push_str(&format!("mac_address={mac}\n"));
    }
    if let Some(ref gh) = p.gh_account {
        out.push_str(&format!("gh_account={gh}\n"));
    }
    if let Some(r) = p.runners {
        out.push_str(&format!("runners={r}\n"));
    }
    if let Some(ref rp) = p.runner_paths {
        out.push_str(&format!("runner_paths={rp}\n"));
    }
    out
}

// ── PeersRegistry impl ────────────────────────────────────────────────────────

impl PeersRegistry {
    pub fn load(path: &Path) -> Result<Self, PeersError> {
        let text = std::fs::read_to_string(path)?;
        let (shared_secret, peers) = parse_ini(&text)?;
        Ok(Self {
            shared_secret,
            peers,
        })
    }

    pub fn save(&self, path: &Path) -> Result<(), PeersError> {
        let mut out = String::new();
        out.push_str("[mesh]\n");
        out.push_str(&format!("shared_secret={}\n", self.shared_secret));
        for (name, cfg) in &self.peers {
            out.push('\n');
            out.push_str(&peer_to_ini(name, cfg));
        }
        std::fs::write(path, out)?;
        Ok(())
    }

    pub fn add_peer(&mut self, name: &str, config: PeerConfig) {
        self.peers.insert(name.to_owned(), config);
    }

    pub fn remove_peer(&mut self, name: &str) -> Option<PeerConfig> {
        self.peers.remove(name)
    }

    pub fn update_role(&mut self, name: &str, role: &str) -> Result<(), PeersError> {
        self.peers
            .get_mut(name)
            .ok_or_else(|| PeersError::NotFound(name.to_owned()))
            .map(|p| p.role = role.to_owned())
    }

    pub fn get_coordinator(&self) -> Option<(&str, &PeerConfig)> {
        self.peers
            .iter()
            .find(|(_, p)| p.role == "coordinator")
            .map(|(n, p)| (n.as_str(), p))
    }

    pub fn list_active(&self) -> Vec<(&str, &PeerConfig)> {
        self.peers
            .iter()
            .filter(|(_, p)| p.status == "active")
            .map(|(n, p)| (n.as_str(), p))
            .collect()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    /// Exact content mirroring ~/.claude/config/peers.conf (minus comment header).
    /// BTreeMap sorts sections alphabetically: mac-worker-1, mac-worker-2, linux_worker.
    const PEERS_INI: &str = "\
[mesh]
shared_secret=test-shared-secret-for-unit-tests

[mac-worker-1]
ssh_alias=mac-dev-ts
user=testuser
os=macos
tailscale_ip=100.64.0.1
dns_name=worker-1.example.ts.net
capabilities=claude,copilot
role=worker
status=active
mac_address=AA:BB:CC:DD:EE:FF
gh_account=Roberdan
runners=3
runner_paths=/Users/testuser/actions-runner,/Users/testuser/actions-runner-2,/Users/testuser/actions-runner-3

[mac-worker-2]
ssh_alias=worker-2.example.ts.net
user=roberdan
os=macos
tailscale_ip=100.64.0.10
dns_name=worker-2.example.ts.net
capabilities=claude,copilot,ollama
role=coordinator
status=active
gh_account=Roberdan

[linux-worker]
ssh_alias=linux-worker-ts
user=roberdan
os=linux
tailscale_ip=100.64.0.2
dns_name=linux_worker.example.ts.net
capabilities=claude,copilot
role=worker
status=active
mac_address=9c:b6:d0:e9:68:07
gh_account=Roberdan
";

    fn load_from_str(s: &str) -> PeersRegistry {
        let f = NamedTempFile::new().unwrap();
        std::fs::write(f.path(), s).unwrap();
        PeersRegistry::load(f.path()).unwrap()
    }

    #[test]
    fn parse_shared_secret() {
        let reg = load_from_str(PEERS_INI);
        assert_eq!(reg.shared_secret, "test-shared-secret-for-unit-tests");
    }

    #[test]
    fn parse_peer_count() {
        let reg = load_from_str(PEERS_INI);
        assert_eq!(reg.peers.len(), 3);
    }

    #[test]
    fn parse_coordinator() {
        let reg = load_from_str(PEERS_INI);
        let (name, peer) = reg.get_coordinator().unwrap();
        assert_eq!(name, "mac-worker-2");
        assert_eq!(peer.role, "coordinator");
        assert_eq!(peer.tailscale_ip, "100.64.0.10");
    }

    #[test]
    fn parse_capabilities_split() {
        let reg = load_from_str(PEERS_INI);
        let mac_worker_2 = reg.peers.get("mac-worker-2").unwrap();
        assert_eq!(
            mac_worker_2.capabilities,
            vec!["claude", "copilot", "ollama"]
        );
    }

    #[test]
    fn parse_optional_fields() {
        let reg = load_from_str(PEERS_INI);
        let mac_worker_1 = reg.peers.get("mac-worker-1").unwrap();
        assert_eq!(
            mac_worker_1.mac_address.as_deref(),
            Some("AA:BB:CC:DD:EE:FF")
        );
        assert_eq!(mac_worker_1.runners, Some(3));
        assert_eq!(
            mac_worker_1.runner_paths.as_deref(),
            Some("/Users/testuser/actions-runner,/Users/testuser/actions-runner-2,/Users/testuser/actions-runner-3")
        );
        let mac_worker_2 = reg.peers.get("mac-worker-2").unwrap();
        assert!(mac_worker_2.mac_address.is_none());
        assert!(mac_worker_2.runners.is_none());
    }

    #[test]
    fn list_active_returns_all_active() {
        let reg = load_from_str(PEERS_INI);
        assert_eq!(reg.list_active().len(), 3);
    }

    #[test]
    fn roundtrip_parse_save_parse() {
        let reg = load_from_str(PEERS_INI);
        let tmp = NamedTempFile::new().unwrap();
        reg.save(tmp.path()).unwrap();

        let reg2 = PeersRegistry::load(tmp.path()).unwrap();
        assert_eq!(reg2.shared_secret, reg.shared_secret);
        assert_eq!(reg2.peers.len(), reg.peers.len());
        for (name, orig) in &reg.peers {
            let saved = reg2.peers.get(name).unwrap();
            assert_eq!(saved, orig, "peer '{name}' changed after roundtrip");
        }
    }

    #[test]
    fn add_and_remove_peer() {
        let mut reg = load_from_str(PEERS_INI);
        let new_peer = PeerConfig {
            ssh_alias: "test-node".to_owned(),
            user: "alice".to_owned(),
            os: "linux".to_owned(),
            tailscale_ip: "100.1.2.3".to_owned(),
            dns_name: "test-node.ts.net".to_owned(),
            capabilities: vec!["claude".to_owned()],
            role: "worker".to_owned(),
            status: "active".to_owned(),
            mac_address: None,
            gh_account: None,
            runners: None,
            runner_paths: None,
        };
        reg.add_peer("testnode", new_peer);
        assert_eq!(reg.peers.len(), 4);

        let removed = reg.remove_peer("testnode").unwrap();
        assert_eq!(removed.ssh_alias, "test-node");
        assert_eq!(reg.peers.len(), 3);
    }

    #[test]
    fn update_role_ok_and_not_found() {
        let mut reg = load_from_str(PEERS_INI);
        reg.update_role("linux-worker", "hybrid").unwrap();
        assert_eq!(reg.peers["linux-worker"].role, "hybrid");
        assert!(reg.update_role("nonexistent", "worker").is_err());
    }
}
