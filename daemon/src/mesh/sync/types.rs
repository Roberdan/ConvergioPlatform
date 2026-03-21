use serde::{Deserialize, Serialize};

pub const MAX_FRAME_BYTES: u32 = 16 * 1024 * 1024;
pub const MAX_PENDING_PEER_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_PEER_NAME_LEN: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeltaChange {
    pub table_name: String,
    #[serde(with = "serde_bytes")]
    pub pk: Vec<u8>,
    pub cid: String,
    pub val: Option<String>,
    pub col_version: i64,
    pub db_version: i64,
    #[serde(with = "serde_bytes")]
    pub site_id: Vec<u8>,
    pub cl: i64,
    pub seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshSyncFrame {
    Heartbeat {
        node: String,
        ts: u64,
    },
    Delta {
        node: String,
        sent_at_ms: u64,
        last_db_version: i64,
        changes: Vec<DeltaChange>,
    },
    Ack {
        node: String,
        applied: usize,
        latency_ms: u64,
        last_db_version: i64,
    },
    /// T1-09: Authentication frames — challenge-response with HMAC-SHA256
    AuthChallenge {
        nonce: Vec<u8>,
        node: String,
    },
    AuthResponse {
        hmac: Vec<u8>,
        node: String,
    },
    AuthResult {
        ok: bool,
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplySummary {
    pub applied: usize,
    pub latency_ms: u64,
    pub last_db_version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramedMeshSyncFrame {
    pub frame: MeshSyncFrame,
    pub payload_len: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerQuota {
    pending_bytes: usize,
    max_pending_bytes: usize,
}

impl PeerQuota {
    pub fn new() -> Self {
        Self {
            pending_bytes: 0,
            max_pending_bytes: MAX_PENDING_PEER_BYTES,
        }
    }

    pub fn with_limit(max_pending_bytes: usize) -> Self {
        Self {
            pending_bytes: 0,
            max_pending_bytes,
        }
    }

    pub fn pending_bytes(&self) -> usize {
        self.pending_bytes
    }

    pub fn release(&mut self, bytes: usize) {
        self.pending_bytes = self.pending_bytes.saturating_sub(bytes);
    }

    pub(super) fn reserve(&mut self, bytes: usize) -> Result<(), String> {
        let next = self
            .pending_bytes
            .checked_add(bytes)
            .ok_or_else(|| "mesh peer pending bytes overflow".to_string())?;
        if next > self.max_pending_bytes {
            return Err(format!(
                "mesh peer pending bytes exceeded: {next} > {}",
                self.max_pending_bytes
            ));
        }
        self.pending_bytes = next;
        Ok(())
    }
}

impl Default for PeerQuota {
    fn default() -> Self {
        Self::new()
    }
}
