<<<<<<< HEAD
mod helpers;

use super::types::{Attestation, Memory, MemoryError, RecallQuery};
use super::MemoryStore;
use helpers::{encode_access, encode_type, recall_conditions, row_to_memory};
use rusqlite::{params, Connection};
use std::sync::Mutex;
use uuid::Uuid;

const SCHEMA_SQL: &str = "
PRAGMA journal_mode=WAL;
PRAGMA busy_timeout=5000;
CREATE TABLE IF NOT EXISTS agent_memories (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL,
  memory_type TEXT NOT NULL,
  content TEXT NOT NULL,
  tags TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  expires_at TEXT,
  access_level TEXT NOT NULL DEFAULT 'Private',
  attestations TEXT NOT NULL DEFAULT '[]',
  deleted_at TEXT,
  shared_with TEXT NOT NULL DEFAULT '[]'
);
CREATE VIRTUAL TABLE IF NOT EXISTS agent_memories_fts USING fts5(
  content, tags, content=agent_memories, content_rowid=rowid
);
CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON agent_memories BEGIN
  INSERT INTO agent_memories_fts(rowid, content, tags)
  VALUES (new.rowid, new.content, new.tags);
END;
CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON agent_memories BEGIN
  INSERT INTO agent_memories_fts(agent_memories_fts, rowid, content, tags)
  VALUES('delete', old.rowid, old.content, old.tags);
END;
CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON agent_memories BEGIN
  INSERT INTO agent_memories_fts(agent_memories_fts, rowid, content, tags)
  VALUES('delete', old.rowid, old.content, old.tags);
  INSERT INTO agent_memories_fts(rowid, content, tags) VALUES (new.rowid, new.content, new.tags);
END;
";

pub struct SqliteMemoryStore {
    conn: Mutex<Connection>,
}

impl SqliteMemoryStore {
    pub fn new(db_path: &str) -> Result<Self, MemoryError> {
        let conn = Connection::open(db_path)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Hard-delete memories whose expires_at has passed.
    pub fn reap_expired(&self) -> Result<(), MemoryError> {
        // expires_at stored as RFC3339; compare directly as lexicographic ISO strings.
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute(
            "DELETE FROM agent_memories WHERE expires_at IS NOT NULL AND expires_at <= ?1",
            params![now],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }
}

impl MemoryStore for SqliteMemoryStore {
    fn remember(&self, memory: Memory) -> Result<String, MemoryError> {
        let id = if memory.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            memory.id.clone()
        };
        let tags_json = serde_json::to_string(&memory.tags)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let attestations_json = serde_json::to_string(&memory.attestations)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute(
            "INSERT INTO agent_memories
             (id, agent_id, memory_type, content, tags, created_at, expires_at,
              access_level, attestations)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                id,
                memory.agent_id,
                encode_type(&memory.memory_type),
                memory.content,
                tags_json,
                memory.created_at.to_rfc3339(),
                memory.expires_at.map(|dt| dt.to_rfc3339()),
                encode_access(&memory.access_level),
                attestations_json,
            ],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(id)
    }

    fn recall(&self, query: RecallQuery) -> Result<Vec<Memory>, MemoryError> {
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let limit = if query.limit == 0 { 100 } else { query.limit };
        let tr = query.time_range.as_ref().map(|(a, b)| (a, b));
        let conds = recall_conditions(
            query.memory_type.as_ref(),
            query.agent_id.as_deref(),
            tr,
            query.text_search.as_deref(),
            query.tags.as_deref(),
        );
        let mut sql = String::from(
            "SELECT id,agent_id,memory_type,content,tags,created_at,expires_at,\
             access_level,attestations FROM agent_memories WHERE deleted_at IS NULL",
        );
        for c in &conds {
            sql.push_str(" AND ");
            sql.push_str(c);
        }
        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {limit}"));

        let mut stmt = conn.prepare(&sql).map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map([], row_to_memory)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let mut memories: Vec<Memory> = Vec::new();
        for row in rows {
            memories.push(row.map_err(|e| MemoryError::StorageError(e.to_string()))?);
        }
        Ok(memories)
    }

    fn forget(&self, memory_id: &str) -> Result<(), MemoryError> {
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let n = conn
            .execute(
                "UPDATE agent_memories SET deleted_at = datetime('now')
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![memory_id],
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        if n == 0 {
            return Err(MemoryError::NotFound(memory_id.to_string()));
        }
        Ok(())
    }

    fn share(&self, memory_id: &str, target_agent_ids: &[String]) -> Result<(), MemoryError> {
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let current: String = conn
            .query_row(
                "SELECT shared_with FROM agent_memories WHERE id=?1 AND deleted_at IS NULL",
                params![memory_id],
                |r| r.get(0),
            )
            .map_err(|_| MemoryError::NotFound(memory_id.to_string()))?;
        let mut list: Vec<String> = serde_json::from_str(&current).unwrap_or_default();
        for id in target_agent_ids {
            if !list.contains(id) {
                list.push(id.clone());
            }
        }
        let json =
            serde_json::to_string(&list).map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute(
            "UPDATE agent_memories SET shared_with=?1 WHERE id=?2",
            params![json, memory_id],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn attest(&self, memory_id: &str, attestation: Attestation) -> Result<(), MemoryError> {
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let current: String = conn
            .query_row(
                "SELECT attestations FROM agent_memories WHERE id=?1 AND deleted_at IS NULL",
                params![memory_id],
                |r| r.get(0),
            )
            .map_err(|_| MemoryError::NotFound(memory_id.to_string()))?;
        let mut list: Vec<Attestation> = serde_json::from_str(&current).unwrap_or_default();
        list.push(attestation);
        let json =
            serde_json::to_string(&list).map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute(
            "UPDATE agent_memories SET attestations=?1 WHERE id=?2",
            params![json, memory_id],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "sqlite_store_tests.rs"]
mod tests;
||||||| parent of 703864c (feat(daemon): Plan 715 T4A-03 — memory sharing and attestation)
=======
mod helpers;

use super::sharing::{check_access, grant_access};
use super::types::{Attestation, Memory, MemoryError, RecallQuery};
use super::MemoryStore;
use helpers::{encode_access, encode_type, recall_conditions, row_to_memory};
use rusqlite::{params, Connection};
use std::sync::Mutex;
use uuid::Uuid;

const SCHEMA_SQL: &str = "
PRAGMA journal_mode=WAL;
PRAGMA busy_timeout=5000;
CREATE TABLE IF NOT EXISTS agent_memories (
  id TEXT PRIMARY KEY,
  agent_id TEXT NOT NULL,
  memory_type TEXT NOT NULL,
  content TEXT NOT NULL,
  tags TEXT NOT NULL DEFAULT '[]',
  created_at TEXT NOT NULL,
  expires_at TEXT,
  access_level TEXT NOT NULL DEFAULT 'Private',
  attestations TEXT NOT NULL DEFAULT '[]',
  deleted_at TEXT,
  shared_with TEXT NOT NULL DEFAULT '[]'
);
CREATE TABLE IF NOT EXISTS memory_access_grants (
  memory_id TEXT NOT NULL,
  granted_to TEXT NOT NULL,
  granted_at TEXT NOT NULL,
  PRIMARY KEY (memory_id, granted_to)
);
CREATE VIRTUAL TABLE IF NOT EXISTS agent_memories_fts USING fts5(
  content, tags, content=agent_memories, content_rowid=rowid
);
CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON agent_memories BEGIN
  INSERT INTO agent_memories_fts(rowid, content, tags)
  VALUES (new.rowid, new.content, new.tags);
END;
CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON agent_memories BEGIN
  INSERT INTO agent_memories_fts(agent_memories_fts, rowid, content, tags)
  VALUES('delete', old.rowid, old.content, old.tags);
END;
CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON agent_memories BEGIN
  INSERT INTO agent_memories_fts(agent_memories_fts, rowid, content, tags)
  VALUES('delete', old.rowid, old.content, old.tags);
  INSERT INTO agent_memories_fts(rowid, content, tags) VALUES (new.rowid, new.content, new.tags);
END;
";

pub struct SqliteMemoryStore {
    conn: Mutex<Connection>,
}

impl SqliteMemoryStore {
    pub fn new(db_path: &str) -> Result<Self, MemoryError> {
        let conn = Connection::open(db_path)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Hard-delete memories whose expires_at has passed.
    pub fn reap_expired(&self) -> Result<(), MemoryError> {
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute(
            "DELETE FROM agent_memories WHERE expires_at IS NOT NULL AND expires_at <= ?1",
            params![now],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }
}

impl MemoryStore for SqliteMemoryStore {
    fn remember(&self, memory: Memory) -> Result<String, MemoryError> {
        let id = if memory.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            memory.id.clone()
        };
        let tags_json = serde_json::to_string(&memory.tags)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let attestations_json = serde_json::to_string(&memory.attestations)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        conn.execute(
            "INSERT INTO agent_memories
             (id, agent_id, memory_type, content, tags, created_at, expires_at,
              access_level, attestations)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                id,
                memory.agent_id,
                encode_type(&memory.memory_type),
                memory.content,
                tags_json,
                memory.created_at.to_rfc3339(),
                memory.expires_at.map(|dt| dt.to_rfc3339()),
                encode_access(&memory.access_level),
                attestations_json,
            ],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(id)
    }

    fn recall(&self, query: RecallQuery) -> Result<Vec<Memory>, MemoryError> {
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let limit = if query.limit == 0 { 100 } else { query.limit };
        let tr = query.time_range.as_ref().map(|(a, b)| (a, b));
        let conds = recall_conditions(
            query.memory_type.as_ref(),
            query.agent_id.as_deref(),
            tr,
            query.text_search.as_deref(),
            query.tags.as_deref(),
        );
        let mut sql = String::from(
            "SELECT id,agent_id,memory_type,content,tags,created_at,expires_at,\
             access_level,attestations FROM agent_memories WHERE deleted_at IS NULL",
        );
        for c in &conds {
            sql.push_str(" AND ");
            sql.push_str(c);
        }
        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {limit}"));

        let mut stmt = conn.prepare(&sql).map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let rows = stmt
            .query_map([], row_to_memory)
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let mut memories: Vec<Memory> = Vec::new();
        for row in rows {
            memories.push(row.map_err(|e| MemoryError::StorageError(e.to_string()))?);
        }

        // Apply access filtering when a querying agent is specified.
        if let Some(ref qid) = query.querying_agent_id {
            let mut accessible = Vec::with_capacity(memories.len());
            for mem in memories {
                let allowed = check_access(&conn, &mem.id, qid)
                    .map_err(|e| MemoryError::StorageError(e.to_string()))?;
                if allowed {
                    accessible.push(mem);
                }
            }
            return Ok(accessible);
        }

        Ok(memories)
    }

    fn forget(&self, memory_id: &str) -> Result<(), MemoryError> {
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        let n = conn
            .execute(
                "UPDATE agent_memories SET deleted_at = datetime('now')
                 WHERE id = ?1 AND deleted_at IS NULL",
                params![memory_id],
            )
            .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        if n == 0 {
            return Err(MemoryError::NotFound(memory_id.to_string()));
        }
        Ok(())
    }

    fn share(&self, memory_id: &str, target_agent_ids: &[String]) -> Result<(), MemoryError> {
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        grant_access(&conn, memory_id, target_agent_ids)?;
        // Also upgrade the access_level to Shared so check_access works correctly.
        conn.execute(
            "UPDATE agent_memories SET access_level = 'Shared', \
             shared_with = '[]' WHERE id = ?1 AND access_level = 'Private'",
            params![memory_id],
        )
        .map_err(|e| MemoryError::StorageError(e.to_string()))?;
        Ok(())
    }

    fn attest(&self, memory_id: &str, attestation: Attestation) -> Result<(), MemoryError> {
        use super::attestation::add_attestation;
        let conn = self.conn.lock().map_err(|e| MemoryError::StorageError(e.to_string()))?;
        add_attestation(&conn, memory_id, attestation)
    }
}

#[cfg(test)]
#[path = "sqlite_store_tests.rs"]
mod tests;
>>>>>>> 703864c (feat(daemon): Plan 715 T4A-03 — memory sharing and attestation)
