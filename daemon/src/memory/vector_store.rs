use super::embeddings::{
    cosine_similarity, embedding_from_bytes, embedding_to_bytes, generate_embedding,
};
use super::types::MemoryError;
use rusqlite::{params, Connection};
use std::sync::Mutex;

const VECTOR_SCHEMA: &str = "
PRAGMA busy_timeout=5000;
CREATE TABLE IF NOT EXISTS memory_vectors (
  memory_id TEXT PRIMARY KEY,
  embedding BLOB NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
";

/// SQLite-backed vector store for memory embeddings.
pub struct VectorStore {
    conn: Mutex<Connection>,
}

/// A search result with memory ID and similarity score.
#[derive(Debug, Clone)]
pub struct VectorMatch {
    pub memory_id: String,
    pub score: f32,
}

impl VectorStore {
    pub fn new(db_path: &str) -> Result<Self, MemoryError> {
        let conn = Connection::open(db_path)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute_batch(VECTOR_SCHEMA)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Construct from an existing connection (for sharing DB with SqliteMemoryStore).
    pub fn from_connection(conn: Connection) -> Result<Self, MemoryError> {
        conn.execute_batch(VECTOR_SCHEMA)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Store an embedding for a memory. Generates embedding from content.
    pub fn store(&self, memory_id: &str, content: &str) -> Result<(), MemoryError> {
        let embedding = generate_embedding(content);
        let bytes = embedding_to_bytes(&embedding);
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO memory_vectors (memory_id, embedding) VALUES (?1, ?2)",
            params![memory_id, bytes],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }

    /// Search for similar embeddings using cosine similarity.
    /// Returns up to `limit` results above `threshold`.
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<VectorMatch>, MemoryError> {
        let query_emb = generate_embedding(query);
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT memory_id, embedding FROM memory_vectors")
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;

        let mut matches: Vec<VectorMatch> = stmt
            .query_map([], |row| {
                let mid: String = row.get(0)?;
                let bytes: Vec<u8> = row.get(1)?;
                Ok((mid, bytes))
            })
            .map_err(|e| MemoryError::StorageError(e.to_string()))?
            .filter_map(|r| r.ok())
            .filter_map(|(mid, bytes)| {
                let emb = embedding_from_bytes(&bytes);
                let score = cosine_similarity(&query_emb, &emb);
                if score >= threshold {
                    Some(VectorMatch { memory_id: mid, score })
                } else {
                    None
                }
            })
            .collect();

        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        matches.truncate(limit);
        Ok(matches)
    }

    /// Delete the embedding for a memory.
    pub fn delete(&self, memory_id: &str) -> Result<(), MemoryError> {
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute(
            "DELETE FROM memory_vectors WHERE memory_id = ?1",
            params![memory_id],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }

    /// Return the number of stored vectors.
    pub fn count(&self) -> Result<usize, MemoryError> {
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM memory_vectors", [], |r| r.get(0))
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(count as usize)
    }
}

#[cfg(test)]
#[path = "vector_store_tests.rs"]
mod tests;
