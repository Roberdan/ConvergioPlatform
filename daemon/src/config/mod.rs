// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Unified config.toml system — sensible defaults, hot-reload ready.

pub mod defaults;
pub mod night;
pub mod validation;
pub mod watcher;

use serde::Deserialize;
use std::io;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Nested config sections
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NodeConfig {
    /// Auto-detected from hostname when empty.
    pub name: String,
    /// standalone | coordinator | worker
    pub role: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        let name = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".to_string());
        Self {
            name,
            role: "standalone".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TailscaleConfig {
    pub enabled: bool,
    pub auth_key: String,
}

impl Default for TailscaleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auth_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MeshConfig {
    pub transport: String,
    pub discovery: String,
    pub peers: Vec<String>,
    pub tailscale: TailscaleConfig,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            transport: "lan".to_string(),
            discovery: "mdns".to_string(),
            peers: Vec::new(),
            tailscale: TailscaleConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct InferenceConfig {
    pub default_model: String,
    pub api_key_env: String,
    pub fallback: InferenceFallbackConfig,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            default_model: "claude-sonnet-4-6".to_string(),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            fallback: InferenceFallbackConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct InferenceFallbackConfig {
    pub max_attempts: usize,
    pub t1: Vec<String>,
    pub t2: Vec<String>,
    pub t3: Vec<String>,
    pub t4: Vec<String>,
}

impl Default for InferenceFallbackConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            t1: vec!["local".into(), "haiku".into(), "sonnet".into()],
            t2: vec!["haiku".into(), "local".into(), "sonnet".into()],
            t3: vec!["sonnet".into(), "opus".into()],
            t4: vec!["opus".into(), "sonnet".into()],
        }
    }
}

impl PartialEq for InferenceFallbackConfig {
    fn eq(&self, other: &Self) -> bool {
        self.max_attempts == other.max_attempts
            && self.t1 == other.t1
            && self.t2 == other.t2
            && self.t3 == other.t3
            && self.t4 == other.t4
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct KernelConfig {
    pub model: String,
    pub model_path: String,
    pub escalation_model: String,
    pub max_tokens: u32,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            model: "none".to_string(),
            model_path: String::new(),
            escalation_model: String::new(),
            max_tokens: 2048,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub token_keychain: String,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token_keychain: String::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DaemonConfig {
    pub port: u16,
    pub quiet_hours: Option<String>,
    pub timezone: Option<String>,
    pub auto_update: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            port: 8420,
            quiet_hours: None,
            timezone: None,
            auto_update: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct ConvergioConfig {
    pub node: NodeConfig,
    pub daemon: DaemonConfig,
    pub night: night::NightConfig,
    pub mesh: MeshConfig,
    pub inference: InferenceConfig,
    pub kernel: KernelConfig,
    pub telegram: TelegramConfig,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Path to the config file. Overridable via `CONVERGIO_CONFIG` env var.
pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("CONVERGIO_CONFIG") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".convergio/config.toml")
}

/// Load config from disk. Falls back to defaults when the file is missing
/// or empty. Logs a warning on parse errors and returns defaults.
pub fn load_config() -> ConvergioConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) if !contents.trim().is_empty() => {
            match toml::from_str::<ConvergioConfig>(&contents) {
                Ok(cfg) => {
                    tracing::info!(
                        "[config] Loaded from {}",
                        path.display()
                    );
                    cfg
                }
                Err(e) => {
                    tracing::warn!(
                        "[config] Parse error in {}: {e} — using defaults",
                        path.display()
                    );
                    ConvergioConfig::default()
                }
            }
        }
        Ok(_) => {
            // File exists but is empty — use defaults
            tracing::info!(
                "[config] {} is empty, using defaults",
                path.display()
            );
            ConvergioConfig::default()
        }
        Err(_) => {
            tracing::info!(
                "[config] No config.toml found, using defaults"
            );
            ConvergioConfig::default()
        }
    }
}

/// Write a well-commented default config template to the given path.
pub fn write_default_config(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, defaults::DEFAULT_CONFIG_TEMPLATE)
}

#[cfg(test)]
mod tests;
