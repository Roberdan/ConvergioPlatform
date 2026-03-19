// Delegation types: error, result, and status enums.

use super::peers::PeersError;
// Re-exported by delegate.rs for external consumers.
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DelegateError {
    #[error("peer not found: {0}")]
    PeerNotFound(String),
    #[error("peer '{0}' is not active (status: {1})")]
    PeerInactive(String, String),
    #[error("SSH connection failed: {0}")]
    SshConnect(String),
    #[error("remote daemon not healthy on {0}: {1}")]
    DaemonUnhealthy(String, String),
    #[error("worktree creation failed: {0}")]
    WorktreeCreate(String),
    #[error("agent spawn failed: {0}")]
    AgentSpawn(String),
    #[error("task timed out after {0:?}")]
    Timeout(Duration),
    #[error("peers config error: {0}")]
    PeersConfig(#[from] PeersError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateResult {
    pub status: DelegateStatus,
    pub output: String,
    pub tokens_used: u64,
    pub duration: Duration,
    pub peer_name: String,
    pub worktree_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegateStatus {
    Success,
    Failed,
    TimedOut,
    Cancelled,
}
