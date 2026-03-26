pub mod types;

pub use types::{AccessLevel, Attestation, Memory, MemoryError, MemoryType, RecallQuery};

/// Persistent storage interface for agent memory.
///
/// Implementations must be `Send + Sync` so they can be shared across
/// async task boundaries in the tokio runtime.
pub trait MemoryStore: Send + Sync {
    /// Persist a new memory entry and return its assigned ID.
    fn remember(&self, memory: Memory) -> Result<String, MemoryError>;

    /// Retrieve memories matching the given query parameters.
    fn recall(&self, query: RecallQuery) -> Result<Vec<Memory>, MemoryError>;

    /// Permanently delete a memory by ID.
    fn forget(&self, memory_id: &str) -> Result<(), MemoryError>;

    /// Share a memory with additional agents by granting read access.
    fn share(&self, memory_id: &str, target_agent_ids: &[String]) -> Result<(), MemoryError>;

    /// Record an attestation from another agent on an existing memory.
    fn attest(&self, memory_id: &str, attestation: Attestation) -> Result<(), MemoryError>;
}

#[cfg(test)]
mod types_tests;
