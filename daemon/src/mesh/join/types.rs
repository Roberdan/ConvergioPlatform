// Join protocol public types and error definitions

use crate::mesh::token;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinConfig {
    pub token: String,
    pub admin_password: String,
    pub profiles: Vec<String>,
    /// When true: emit JoinProgress JSON lines to stdout for GUI consumption.
    pub interactive: bool,
    pub selections: JoinSelections,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JoinSelections {
    pub network: bool,
    pub brew: bool,
    pub apps: bool,
    pub repos: bool,
    pub shell: bool,
    pub auth: bool,
    pub macos_tweaks: bool,
    pub coordinator_migration: bool,
    pub runners: bool,
}

impl JoinSelections {
    /// All components selected (default for non-interactive use).
    pub fn all() -> Self {
        Self {
            network: true,
            brew: true,
            apps: true,
            repos: true,
            shell: true,
            auth: true,
            macos_tweaks: true,
            coordinator_migration: true,
            runners: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JoinProgress {
    pub step: u8,
    pub total_steps: u8,
    pub current: String,
    pub status: StepStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    Running,
    Done,
    Skipped,
    Failed(String),
}

#[derive(Debug, Error)]
pub enum JoinError {
    #[error("token error: {0}")]
    Token(#[from] token::TokenError),
    #[error("network error: {0}")]
    Network(String),
    #[error("bundle download failed: {0}")]
    BundleDownload(String),
    #[error("auth import failed: {0}")]
    AuthImport(String),
    #[error("coordinator error: {0}")]
    Coordinator(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("preflight failed: {0}")]
    Preflight(String),
}
