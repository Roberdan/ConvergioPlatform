use super::types::SecurityError;
use std::collections::HashSet;
use std::path::PathBuf;

/// Sandbox configuration for an agent execution environment.
/// Restricts filesystem, network, and subprocess access.
pub struct SandboxConfig {
    /// Allowed filesystem paths (read+write).
    pub allowed_paths: Vec<PathBuf>,
    /// Allowed network destinations (host:port or host:*).
    pub allowed_network: HashSet<String>,
    /// Whether subprocess spawning is allowed.
    pub allow_subprocess: bool,
    /// Maximum memory usage in bytes.
    pub max_memory_bytes: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            allowed_paths: vec![],
            allowed_network: HashSet::new(),
            allow_subprocess: false,
            max_memory_bytes: 512 * 1024 * 1024, // 512 MiB
        }
    }
}

/// Sandbox enforcer — checks operations against sandbox rules.
pub struct SandboxEnforcer {
    config: SandboxConfig,
    violations: Vec<SandboxViolation>,
}

#[derive(Debug, Clone)]
pub struct SandboxViolation {
    pub agent_id: String,
    pub violation_type: String,
    pub detail: String,
    pub timestamp: String,
}

impl SandboxEnforcer {
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            violations: Vec::new(),
        }
    }

    /// Check filesystem access against sandbox rules.
    pub fn check_filesystem(&mut self, agent_id: &str, path: &str) -> Result<(), SecurityError> {
        let target = PathBuf::from(path);
        let allowed = self.config.allowed_paths.iter().any(|p| target.starts_with(p));
        if allowed {
            Ok(())
        } else {
            self.record_violation(agent_id, "filesystem", path);
            Err(SecurityError::SandboxViolation(format!(
                "filesystem access denied: {path}"
            )))
        }
    }

    /// Check network access against sandbox rules.
    pub fn check_network(&mut self, agent_id: &str, destination: &str) -> Result<(), SecurityError> {
        let allowed = self.config.allowed_network.contains(destination)
            || self.config.allowed_network.contains("*");
        if allowed {
            Ok(())
        } else {
            self.record_violation(agent_id, "network", destination);
            Err(SecurityError::SandboxViolation(format!(
                "network access denied: {destination}"
            )))
        }
    }

    /// Check subprocess spawning permission.
    pub fn check_subprocess(&mut self, agent_id: &str, cmd: &str) -> Result<(), SecurityError> {
        if self.config.allow_subprocess {
            Ok(())
        } else {
            self.record_violation(agent_id, "subprocess", cmd);
            Err(SecurityError::SandboxViolation(format!(
                "subprocess spawning denied: {cmd}"
            )))
        }
    }

    pub fn violations(&self) -> &[SandboxViolation] {
        &self.violations
    }

    fn record_violation(&mut self, agent_id: &str, vtype: &str, detail: &str) {
        self.violations.push(SandboxViolation {
            agent_id: agent_id.to_string(),
            violation_type: vtype.to_string(),
            detail: detail.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }
}
