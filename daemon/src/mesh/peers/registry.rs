// PeersRegistry: load, save, and mutation operations.

use std::path::Path;

use super::parser::{parse_ini, peer_to_ini};
use super::types::{PeerConfig, PeersError, PeersRegistry};

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
