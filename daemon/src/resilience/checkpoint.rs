// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
//! Plan-level checkpoint persistence: save/restore wave execution state to DB.
//!
//! Allows long-running plan executions to restart without data loss
//! (Article XI — Checkpoint/restart).
//!
//! Table: `checkpoints (id, plan_id, wave_id, state JSON, created_at)`.
//! Migration is appended to `state_init_migrations.rs`.

use rusqlite::{Connection, params};
use serde_json::Value;
use thiserror::Error;

/// Snapshot of a plan's execution state at a point in time.
#[derive(Debug, Clone)]
pub struct CheckpointState {
    pub plan_id: i64,
    pub wave_id: String,
    pub state: Value,
    pub created_at: String,
}

#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Ensure the `checkpoints` table exists. Called at startup or on first use.
pub fn ensure_table(conn: &Connection) -> Result<(), CheckpointError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS checkpoints (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            plan_id    INTEGER NOT NULL,
            wave_id    TEXT    NOT NULL DEFAULT '',
            state      TEXT    NOT NULL DEFAULT '{}',
            created_at TEXT    NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_checkpoints_plan_id ON checkpoints(plan_id);",
    )?;
    Ok(())
}

/// Persist a checkpoint. Replaces any existing checkpoint for (plan_id, wave_id).
///
/// Uses INSERT OR REPLACE so callers can call `save_checkpoint` repeatedly
/// without accumulating unbounded rows for the same wave.
pub fn save_checkpoint(
    conn: &Connection,
    plan_id: i64,
    wave_id: &str,
    state: &Value,
) -> Result<(), CheckpointError> {
    ensure_table(conn)?;
    let state_json = serde_json::to_string(state)?;

    // Delete previous snapshot for this (plan_id, wave_id) pair then insert fresh.
    conn.execute(
        "DELETE FROM checkpoints WHERE plan_id = ?1 AND wave_id = ?2",
        params![plan_id, wave_id],
    )?;
    conn.execute(
        "INSERT INTO checkpoints (plan_id, wave_id, state) VALUES (?1, ?2, ?3)",
        params![plan_id, wave_id, state_json],
    )?;
    Ok(())
}

/// Return the most recent checkpoint for `plan_id`, or `None` if none exists.
pub fn restore_checkpoint(
    conn: &Connection,
    plan_id: i64,
) -> Result<Option<CheckpointState>, CheckpointError> {
    ensure_table(conn)?;

    let mut stmt = conn.prepare(
        "SELECT plan_id, wave_id, state, created_at
           FROM checkpoints
          WHERE plan_id = ?1
          ORDER BY id DESC
          LIMIT 1",
    )?;

    let result = stmt.query_row(params![plan_id], |row| {
        let state_json: String = row.get(2)?;
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            state_json,
            row.get::<_, String>(3)?,
        ))
    });

    match result {
        Ok((pid, wave_id, state_json, created_at)) => {
            let state: Value = serde_json::from_str(&state_json).unwrap_or(Value::Null);
            Ok(Some(CheckpointState {
                plan_id: pid,
                wave_id,
                state,
                created_at,
            }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(CheckpointError::Db(e)),
    }
}

/// Delete all checkpoints for a plan (call after successful plan completion).
pub fn clear_checkpoints(conn: &Connection, plan_id: i64) -> Result<usize, CheckpointError> {
    ensure_table(conn)?;
    let n = conn.execute(
        "DELETE FROM checkpoints WHERE plan_id = ?1",
        params![plan_id],
    )?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use serde_json::json;

    fn open_mem() -> Connection {
        Connection::open_in_memory().expect("in-memory db")
    }

    #[test]
    fn save_and_restore_roundtrip() {
        let conn = open_mem();
        let state = json!({"wave": "W1", "tasks_done": 3});
        save_checkpoint(&conn, 100, "W1", &state).unwrap();
        let restored = restore_checkpoint(&conn, 100).unwrap().unwrap();
        assert_eq!(restored.plan_id, 100);
        assert_eq!(restored.wave_id, "W1");
        assert_eq!(restored.state["tasks_done"], 3);
    }

    #[test]
    fn restore_missing_plan_returns_none() {
        let conn = open_mem();
        let result = restore_checkpoint(&conn, 9999).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn save_replaces_existing_checkpoint() {
        let conn = open_mem();
        save_checkpoint(&conn, 200, "W1", &json!({"v": 1})).unwrap();
        save_checkpoint(&conn, 200, "W1", &json!({"v": 2})).unwrap();
        let restored = restore_checkpoint(&conn, 200).unwrap().unwrap();
        assert_eq!(restored.state["v"], 2, "second save should overwrite first");
    }

    #[test]
    fn save_multiple_waves_restores_latest() {
        let conn = open_mem();
        save_checkpoint(&conn, 300, "W1", &json!({"x": 1})).unwrap();
        save_checkpoint(&conn, 300, "W2", &json!({"x": 2})).unwrap();
        let restored = restore_checkpoint(&conn, 300).unwrap().unwrap();
        // Most recent by id — W2 was inserted last
        assert_eq!(restored.wave_id, "W2");
    }

    #[test]
    fn clear_checkpoints_removes_all_for_plan() {
        let conn = open_mem();
        save_checkpoint(&conn, 400, "W1", &json!({})).unwrap();
        save_checkpoint(&conn, 400, "W2", &json!({})).unwrap();
        let deleted = clear_checkpoints(&conn, 400).unwrap();
        assert_eq!(deleted, 2);
        let after = restore_checkpoint(&conn, 400).unwrap();
        assert!(after.is_none());
    }

    #[test]
    fn clear_does_not_affect_other_plans() {
        let conn = open_mem();
        save_checkpoint(&conn, 500, "W1", &json!({"keep": true})).unwrap();
        save_checkpoint(&conn, 501, "W1", &json!({"other": true})).unwrap();
        clear_checkpoints(&conn, 501).unwrap();
        let still_there = restore_checkpoint(&conn, 500).unwrap();
        assert!(still_there.is_some());
    }

    #[test]
    fn ensure_table_is_idempotent() {
        let conn = open_mem();
        ensure_table(&conn).unwrap();
        ensure_table(&conn).unwrap(); // second call must not error
    }

    #[test]
    fn checkpoint_state_has_created_at() {
        let conn = open_mem();
        save_checkpoint(&conn, 600, "W3", &json!({})).unwrap();
        let cp = restore_checkpoint(&conn, 600).unwrap().unwrap();
        assert!(!cp.created_at.is_empty());
    }
}
