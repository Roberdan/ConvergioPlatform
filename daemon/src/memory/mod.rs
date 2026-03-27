pub mod attestation;
pub mod embeddings;
pub mod sharing;
pub mod sqlite_store;
pub mod types;
pub mod vector_store;

pub use types::{AccessLevel, Attestation, Memory, MemoryError, MemoryType, RecallQuery};
pub use vector_store::{VectorMatch, VectorStore};

pub trait MemoryStore: Send + Sync {
    fn remember(&self, memory: Memory) -> Result<String, MemoryError>;
    fn recall(&self, query: RecallQuery) -> Result<Vec<Memory>, MemoryError>;
    fn forget(&self, memory_id: &str) -> Result<(), MemoryError>;
    fn share(&self, memory_id: &str, target_agent_ids: &[String]) -> Result<(), MemoryError>;
    fn attest(&self, memory_id: &str, attestation: Attestation) -> Result<(), MemoryError>;
}

#[cfg(test)]
mod types_tests;

#[cfg(test)]
#[path = "sharing_tests.rs"]
mod sharing_tests;

#[cfg(test)]
#[path = "attestation_tests.rs"]
mod attestation_tests;
