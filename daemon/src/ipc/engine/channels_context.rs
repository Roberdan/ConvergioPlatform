use super::super::protocol::{ChannelInfo, ContextEntry, IpcResponse};
use super::core::IpcEngine;

impl IpcEngine {
    // ── Channels ─────────────────────────────────────

    pub fn channel_create(
        &self,
        name: &str,
        description: Option<&str>,
        created_by: &str,
    ) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO ipc_channels (name, description, created_by) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, description, created_by],
        )?;
        Ok(IpcResponse::Ok {
            message: format!("channel '{name}' created"),
        })
    }

    pub fn channel_list(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let mut stmt = conn.prepare(
            "SELECT name, description, created_by, created_at FROM ipc_channels ORDER BY name",
        )?;
        let channels: Vec<ChannelInfo> = stmt
            .query_map([], |row| {
                Ok(ChannelInfo {
                    name: row.get(0)?,
                    description: row.get(1)?,
                    created_by: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(IpcResponse::ChannelList { channels })
    }

    // ── Shared Context (LWW) ─────────────────────────

    pub fn context_get(&self, key: &str) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let result = conn.query_row(
            "SELECT key, value, version, set_by, updated_at FROM ipc_shared_context WHERE key = ?1",
            rusqlite::params![key],
            |row| {
                Ok(IpcResponse::Context {
                    key: row.get(0)?,
                    value: row.get(1)?,
                    version: row.get(2)?,
                    set_by: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        );
        match result {
            Ok(resp) => Ok(resp),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(IpcResponse::Error {
                code: 404,
                message: format!("key '{key}' not found"),
            }),
            Err(e) => Err(e),
        }
    }

    pub fn context_set(
        &self,
        key: &str,
        value: &str,
        set_by: &str,
    ) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        conn.execute(
            "INSERT INTO ipc_shared_context (key, value, version, set_by, updated_at)
             VALUES (?1, ?2, 1, ?3, strftime('%Y-%m-%dT%H:%M:%f','now'))
             ON CONFLICT(key) DO UPDATE SET
               value = excluded.value,
               version = ipc_shared_context.version + 1,
               set_by = excluded.set_by,
               updated_at = strftime('%Y-%m-%dT%H:%M:%f','now')",
            rusqlite::params![key, value, set_by],
        )?;
        Ok(IpcResponse::Ok {
            message: format!("context '{key}' set"),
        })
    }

    pub fn context_list(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let mut stmt = conn.prepare(
            "SELECT key, value, version, set_by, updated_at FROM ipc_shared_context ORDER BY key",
        )?;
        let entries: Vec<ContextEntry> = stmt
            .query_map([], |row| {
                Ok(ContextEntry {
                    key: row.get(0)?,
                    value: row.get(1)?,
                    version: row.get(2)?,
                    set_by: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(IpcResponse::ContextList { entries })
    }

    pub fn context_delete(&self, key: &str) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let deleted = conn.execute(
            "DELETE FROM ipc_shared_context WHERE key = ?1",
            rusqlite::params![key],
        )?;
        if deleted > 0 {
            Ok(IpcResponse::Ok {
                message: format!("context '{key}' deleted"),
            })
        } else {
            Ok(IpcResponse::Error {
                code: 404,
                message: format!("key '{key}' not found"),
            })
        }
    }

    // ── DB Maintenance ───────────────────────────────

    pub fn db_stats(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        let agents: u64 = conn.query_row("SELECT count(*) FROM ipc_agents", [], |r| r.get(0))?;
        let messages: u64 =
            conn.query_row("SELECT count(*) FROM ipc_messages", [], |r| r.get(0))?;
        let channels: u64 =
            conn.query_row("SELECT count(*) FROM ipc_channels", [], |r| r.get(0))?;
        let context_keys: u64 =
            conn.query_row("SELECT count(*) FROM ipc_shared_context", [], |r| r.get(0))?;

        let db_size_bytes = std::fs::metadata(&self.db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(IpcResponse::Stats {
            agents,
            messages,
            channels,
            context_keys,
            db_size_bytes,
        })
    }

    pub fn db_cleanup(&self, older_than_days: u32) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        // Use concatenation inside SQLite so older_than_days is a bound parameter,
        // not interpolated into the SQL string (prevents SQL injection).
        let deleted = conn.execute(
            "DELETE FROM ipc_messages WHERE created_at < datetime('now', '-' || ?1 || ' days')",
            rusqlite::params![older_than_days],
        )?;
        Ok(IpcResponse::Ok {
            message: format!("cleaned up {deleted} message(s)"),
        })
    }

    pub fn db_vacuum(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")?;
        Ok(IpcResponse::Ok {
            message: "vacuum complete".into(),
        })
    }

    pub fn db_reset(&self) -> rusqlite::Result<IpcResponse> {
        let conn = self.open_conn()?;
        // Explicit match on the IPC_TABLES constant values for defense-in-depth:
        // table names are compile-time constants, but using format!() would open
        // the door to injection if the constant source ever changes.
        for &table in &super::super::schema::IPC_TABLES {
            let sql = match table {
                "ipc_agents" => "DELETE FROM ipc_agents",
                "ipc_messages" => "DELETE FROM ipc_messages",
                "ipc_channels" => "DELETE FROM ipc_channels",
                "ipc_shared_context" => "DELETE FROM ipc_shared_context",
                "ipc_file_locks" => "DELETE FROM ipc_file_locks",
                "ipc_worktrees" => "DELETE FROM ipc_worktrees",
                other => {
                    tracing::warn!("db_reset: unknown table '{other}' skipped");
                    continue;
                }
            };
            conn.execute(sql, [])?;
        }
        Ok(IpcResponse::Ok {
            message: "all IPC tables cleared".into(),
        })
    }
}
