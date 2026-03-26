use crate::memory::types::MemoryError;
use chrono::Utc;
use rusqlite::{params, Connection};

/// A record granting access to a memory for a specific agent.
#[derive(Debug, Clone)]
pub struct MemoryAccessGrant {
    /// The memory this grant applies to.
    pub memory_id: String,
    /// The agent being granted access.
    pub granted_to: String,
    /// RFC3339 timestamp when the grant was created.
    pub granted_at: String,
}

/// Grant access to `memory_id` for each agent in `target_agent_ids`.
/// Idempotent: duplicate grants are silently ignored (INSERT OR IGNORE).
pub fn grant_access(
    conn: &Connection,
    memory_id: &str,
    target_agent_ids: &[String],
) -> Result<(), MemoryError> {
    let now = Utc::now().to_rfc3339();
    for agent_id in target_agent_ids {
        conn.execute(
            "INSERT OR IGNORE INTO memory_access_grants (memory_id, granted_to, granted_at)
             VALUES (?1, ?2, ?3)",
            params![memory_id, agent_id, now],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
    }
    Ok(())
}

/// Check whether `agent_id` can access `memory_id`.
///
/// Rules:
/// - `Public` → always true
/// - `Private` → only the owning agent (agent_id = owner)
/// - `Shared` → owner OR any agent with an access grant
pub fn check_access(
    conn: &Connection,
    memory_id: &str,
    agent_id: &str,
) -> Result<bool, MemoryError> {
    // Fetch access_level and owner for this memory.
    let result: Option<(String, String)> = conn
        .query_row(
            "SELECT access_level, agent_id FROM agent_memories
             WHERE id = ?1 AND deleted_at IS NULL",
            params![memory_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .ok();

    let (access_level, owner) = match result {
        Some(row) => row,
        None => return Err(MemoryError::NotFound(memory_id.to_string())),
    };

    match access_level.as_str() {
        "Public" => Ok(true),
        "Private" => Ok(owner == agent_id),
        "Shared" => {
            // Owner always has access; otherwise check grant table.
            if owner == agent_id {
                return Ok(true);
            }
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM memory_access_grants
                     WHERE memory_id = ?1 AND granted_to = ?2",
                    params![memory_id, agent_id],
                    |r| r.get(0),
                )
                .map_err(|e| MemoryError::StorageError(e.to_string()))?;
            Ok(count > 0)
        }
        _ => Ok(false),
    }
}
