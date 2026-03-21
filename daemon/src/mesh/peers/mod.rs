// INI-format parser/writer compatible with ~/.claude/config/peers.conf

mod parser;
mod registry;
mod types;

pub use types::{PeerConfig, PeersError, PeersRegistry};

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
