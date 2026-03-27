use super::types::ArtifactError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Convergio deployment blueprint.
/// Declares daemon config, active channels, inference providers,
/// security policies, ports, and data paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub name: String,
    pub version: String,
    pub daemon: DaemonConfig,
    pub channels: Vec<ChannelConfig>,
    pub inference: Vec<InferenceProvider>,
    pub security: SecurityConfig,
    pub data: DataConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub port: u16,
    pub host: String,
    pub log_level: String,
    pub max_agents: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub name: String,
    pub r#type: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceProvider {
    pub name: String,
    pub provider: String,
    pub model: String,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_acl: bool,
    pub enable_sandbox: bool,
    pub enable_audit: bool,
    pub enable_egress_firewall: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    pub db_path: String,
    pub blobs_path: String,
    pub memory_dir: String,
}

impl Blueprint {
    /// Load blueprint from YAML file.
    pub fn from_file(path: &Path) -> Result<Self, ArtifactError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ArtifactError::ScanError(format!("read blueprint: {e}")))?;
        serde_yaml::from_str(&content)
            .map_err(|e| ArtifactError::ScanError(format!("parse blueprint: {e}")))
    }

    /// Generate a plan summary of what would be deployed.
    pub fn plan_summary(&self) -> String {
        let mut s = format!("Blueprint: {} v{}\n", self.name, self.version);
        s.push_str(&format!("  Daemon: {}:{}\n", self.daemon.host, self.daemon.port));
        s.push_str(&format!("  Channels: {}\n", self.channels.len()));
        s.push_str(&format!("  Inference: {}\n", self.inference.len()));
        s.push_str(&format!("  Security: ACL={} Sandbox={} Audit={} Egress={}\n",
            self.security.enable_acl, self.security.enable_sandbox,
            self.security.enable_audit, self.security.enable_egress_firewall));
        s
    }
}

impl Default for Blueprint {
    fn default() -> Self {
        Self {
            name: "convergio".to_string(),
            version: "1.0.0".to_string(),
            daemon: DaemonConfig {
                port: 8420, host: "127.0.0.1".to_string(),
                log_level: "info".to_string(), max_agents: 20,
            },
            channels: vec![
                ChannelConfig { name: "rest".to_string(), r#type: "http".to_string(), enabled: true },
                ChannelConfig { name: "websocket".to_string(), r#type: "ws".to_string(), enabled: true },
            ],
            inference: vec![InferenceProvider {
                name: "primary".to_string(), provider: "anthropic".to_string(),
                model: "claude-sonnet-4-6".to_string(), priority: 1,
            }],
            security: SecurityConfig {
                enable_acl: true, enable_sandbox: true,
                enable_audit: true, enable_egress_firewall: true,
            },
            data: DataConfig {
                db_path: "data/dashboard.db".to_string(),
                blobs_path: "data/blobs".to_string(),
                memory_dir: "data/memory".to_string(),
            },
        }
    }
}
