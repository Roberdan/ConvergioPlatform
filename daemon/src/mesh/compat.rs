// Compatibility layer for existing mesh scripts.
// Provides legacy-format loading and backward-compat verification.

use crate::mesh::peers::{PeersError, PeersRegistry};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompatError {
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("missing shared_secret in [mesh] section")]
    MissingSharedSecret,
    #[error("no peers defined")]
    NoPeers,
}

pub struct CompatReport {
    pub peer_count: usize,
    pub has_shared_secret: bool,
    pub coordinator_present: bool,
}

/// Load peers.conf in the legacy INI format used by existing mesh scripts.
///
/// Delegates to `PeersRegistry::load` since the format is identical.
pub fn load_legacy_peers(path: &Path) -> Result<PeersRegistry, CompatError> {
    PeersRegistry::load(path).map_err(|e: PeersError| CompatError::ParseError(e.to_string()))
}

/// Verify that a peers.conf file is backward-compatible with existing mesh scripts.
///
/// Checks:
/// - File can be parsed
/// - [mesh] section has shared_secret (non-empty)
/// - At least one peer is defined
pub fn verify_backward_compat(peers_path: &Path) -> Result<CompatReport, CompatError> {
    let registry = load_legacy_peers(peers_path)?;

    if registry.shared_secret.is_empty() {
        return Err(CompatError::MissingSharedSecret);
    }

    if registry.peers.is_empty() {
        return Err(CompatError::NoPeers);
    }

    let coordinator_present = registry.get_coordinator().is_some();

    Ok(CompatReport {
        peer_count: registry.peers.len(),
        has_shared_secret: true,
        coordinator_present,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    const VALID_CONF: &str = "\
[mesh]
shared_secret=test-secret-v1

[testnode1]
ssh_alias=test1.local
user=testuser
os=macos
tailscale_ip=100.1.2.3
dns_name=test1.tail.ts.net
capabilities=claude,copilot
role=coordinator
status=active
";

    #[test]
    fn test_load_legacy_peers_valid() {
        let f = NamedTempFile::new().unwrap();
        std::fs::write(f.path(), VALID_CONF).unwrap();
        let reg = load_legacy_peers(f.path()).unwrap();
        assert_eq!(reg.shared_secret, "test-secret-v1");
        assert_eq!(reg.peers.len(), 1);
    }

    #[test]
    fn test_verify_backward_compat_valid() {
        let f = NamedTempFile::new().unwrap();
        std::fs::write(f.path(), VALID_CONF).unwrap();
        let report = verify_backward_compat(f.path()).unwrap();
        assert!(report.has_shared_secret);
        assert_eq!(report.peer_count, 1);
        assert!(report.coordinator_present);
    }

    #[test]
    fn test_verify_backward_compat_missing_secret() {
        let conf = "[mesh]\nshared_secret=\n\n[node1]\nssh_alias=a\nuser=u\nos=linux\ntailscale_ip=1.2.3.4\ndns_name=a.ts.net\ncapabilities=claude\nrole=worker\nstatus=active\n";
        let f = NamedTempFile::new().unwrap();
        std::fs::write(f.path(), conf).unwrap();
        let result = verify_backward_compat(f.path());
        assert!(matches!(result, Err(CompatError::MissingSharedSecret)));
    }
}
