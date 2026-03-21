// Coordinator types: migration state, peer snapshots, error variants.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MigrationState {
    pub old_coordinator: String,
    pub new_coordinator: String,
    /// peers.conf backup per node — used for rollback.
    pub snapshots: Vec<PeerSnapshot>,
    pub started_at: String,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeerSnapshot {
    pub peer_name: String,
    /// Full content of peers.conf on that node before migration.
    pub peers_conf_backup: String,
}

#[derive(Debug, Error)]
pub enum CoordinatorError {
    #[error("peer '{0}' not found")]
    PeerNotFound(String),
    #[error("SSH command failed on '{peer}': {reason}")]
    Ssh { peer: String, reason: String },
    #[error("SCP transfer failed: {0}")]
    Scp(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialisation error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("rollback error: {0}")]
    Rollback(String),
}
