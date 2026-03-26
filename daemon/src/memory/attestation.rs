use crate::memory::types::{Attestation, MemoryError};
use rusqlite::{params, Connection};

/// Append `attestation` to the attestations JSON array of `memory_id`.
/// Returns `MemoryError::NotFound` if the memory does not exist (or is deleted).
pub fn add_attestation(
    conn: &Connection,
    memory_id: &str,
    attestation: Attestation,
) -> Result<(), MemoryError> {
    // Read existing attestations, confirm record exists.
    let current: String = conn
        .query_row(
            "SELECT attestations FROM agent_memories WHERE id = ?1 AND deleted_at IS NULL",
            params![memory_id],
            |r| r.get(0),
        )
        .map_err(|_| MemoryError::NotFound(memory_id.to_string()))?;

    let mut list: Vec<Attestation> = serde_json::from_str(&current).unwrap_or_default();
    list.push(attestation);

    let json =
        serde_json::to_string(&list).map_err(|e| MemoryError::StorageError(e.to_string()))?;
    conn.execute(
        "UPDATE agent_memories SET attestations = ?1 WHERE id = ?2",
        params![json, memory_id],
    )
    .map_err(|e| MemoryError::StorageError(e.to_string()))?;
    Ok(())
}

/// Return all attestations for `memory_id` in insertion order.
/// Returns `MemoryError::NotFound` if the memory does not exist (or is deleted).
pub fn get_attestation_chain(
    conn: &Connection,
    memory_id: &str,
) -> Result<Vec<Attestation>, MemoryError> {
    let json: String = conn
        .query_row(
            "SELECT attestations FROM agent_memories WHERE id = ?1 AND deleted_at IS NULL",
            params![memory_id],
            |r| r.get(0),
        )
        .map_err(|_| MemoryError::NotFound(memory_id.to_string()))?;

    let list: Vec<Attestation> = serde_json::from_str(&json).unwrap_or_default();
    Ok(list)
}

/// Compute average confidence of all attestations.
/// Returns `0.0` for an empty slice.
pub fn trust_score(attestations: &[Attestation]) -> f64 {
    if attestations.is_empty() {
        return 0.0;
    }
    let sum: f64 = attestations.iter().map(|a| a.confidence).sum();
    sum / attestations.len() as f64
}
