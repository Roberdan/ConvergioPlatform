use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Categories of memory an agent can store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MemoryType {
    /// Objective fact about the world or system state.
    Fact,
    /// A decision that was made, with associated rationale.
    Decision,
    /// A recurring preference or behavioural tendency.
    Preference,
    /// An observation made at a point in time.
    Observation,
}

/// Who can read a memory entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AccessLevel {
    /// Visible only to the owning agent.
    Private,
    /// Visible to explicitly named agents via `share`.
    Shared,
    /// Visible to all agents in the swarm.
    Public,
}

/// A signed endorsement from another agent confirming a memory's accuracy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    /// ID of the agent providing the attestation.
    pub attesting_agent_id: String,
    /// When the attestation was created.
    pub timestamp: DateTime<Utc>,
    /// Confidence score in range [0.0, 1.0].
    pub confidence: f64,
}

/// A single memory entry stored by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// UUID identifying this memory.
    pub id: String,
    /// Agent that created the memory.
    pub agent_id: String,
    /// Classification of the memory.
    pub memory_type: MemoryType,
    /// Free-text content of the memory.
    pub content: String,
    /// Searchable tags for recall filtering.
    pub tags: Vec<String>,
    /// When the memory was created.
    pub created_at: DateTime<Utc>,
    /// Optional expiry — `None` means the memory never expires.
    pub expires_at: Option<DateTime<Utc>>,
    /// Who can access this memory.
    pub access_level: AccessLevel,
    /// Third-party attestations endorsing this memory.
    pub attestations: Vec<Attestation>,
}

/// Filter parameters for recalling memories.
#[derive(Debug, Clone)]
pub struct RecallQuery {
    /// Restrict to a specific memory type.
    pub memory_type: Option<MemoryType>,
    /// Restrict to memories that carry all of these tags.
    pub tags: Option<Vec<String>>,
    /// Restrict to memories created within this time window.
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Full-text search against memory content.
    pub text_search: Option<String>,
    /// Semantic similarity search query. When set, results include vector
    /// similarity scores and are ranked by a hybrid FTS5+vector score.
    pub semantic_query: Option<String>,
    /// Restrict to memories owned by a specific agent.
    pub agent_id: Option<String>,
    /// Maximum number of results to return.
    pub limit: usize,
    /// Agent performing the query — used for access control.
    /// `None` skips access filtering (internal/admin queries only).
    pub querying_agent_id: Option<String>,
    /// Weight for FTS5 score in hybrid ranking (0.0-1.0). Default 0.5.
    pub fts_weight: f32,
}

impl Default for RecallQuery {
    fn default() -> Self {
        Self {
            memory_type: None,
            tags: None,
            time_range: None,
            text_search: None,
            semantic_query: None,
            agent_id: None,
            limit: 100,
            querying_agent_id: None,
            fts_weight: 0.5,
        }
    }
}

/// Errors returned by a `MemoryStore` implementation.
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("memory not found: {0}")]
    NotFound(String),
    #[error("access denied: {0}")]
    AccessDenied(String),
    #[error("storage error: {0}")]
    StorageError(String),
    #[error("memory expired: {0}")]
    Expired(String),
}
