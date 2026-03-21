use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpcFileLock {
    pub file_pattern: String,
    pub agent: String,
    pub host: String,
    pub pid: i64,
    pub locked_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquireResult {
    Acquired,
    Rejected(IpcFileLock),
}

pub struct IpcLockStore {
    conn: Connection,
}

impl IpcLockStore {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.init_schema()?;
        Ok(store)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn acquire_lock(
        &mut self,
        file_pattern: &str,
        agent: &str,
        host: &str,
        pid: i64,
    ) -> rusqlite::Result<AcquireResult> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let existing: Option<IpcFileLock> = tx
            .query_row(
                "SELECT file_pattern, agent, host, pid, locked_at
                 FROM ipc_file_locks WHERE file_pattern = ?1",
                params![file_pattern],
                map_lock_row,
            )
            .optional()?;

        if let Some(lock) = existing {
            if lock.agent == agent && lock.host == host {
                tx.execute(
                    "UPDATE ipc_file_locks SET pid = ?1, locked_at = datetime('now')
                     WHERE file_pattern = ?2 AND agent = ?3 AND host = ?4",
                    params![pid, file_pattern, agent, host],
                )?;
                tx.commit()?;
                return Ok(AcquireResult::Acquired);
            }
            tx.commit()?;
            return Ok(AcquireResult::Rejected(lock));
        }

        tx.execute(
            "INSERT INTO ipc_file_locks(file_pattern, agent, host, pid)
             VALUES (?1, ?2, ?3, ?4)",
            params![file_pattern, agent, host, pid],
        )?;
        tx.commit()?;
        Ok(AcquireResult::Acquired)
    }

    pub fn release_lock(
        &self,
        file_pattern: &str,
        agent: &str,
        host: &str,
    ) -> rusqlite::Result<usize> {
        self.conn.execute(
            "DELETE FROM ipc_file_locks
             WHERE file_pattern = ?1 AND agent = ?2 AND host = ?3",
            params![file_pattern, agent, host],
        )
    }

    pub fn list_locks(&self) -> rusqlite::Result<Vec<IpcFileLock>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_pattern, agent, host, pid, locked_at
             FROM ipc_file_locks ORDER BY locked_at DESC",
        )?;
        let rows = stmt.query_map([], map_lock_row)?;
        rows.collect()
    }

    pub fn prune_dead(&self) -> rusqlite::Result<usize> {
        let locks = self.list_locks()?;
        let mut pruned = 0;
        for lock in &locks {
            if !is_pid_alive(lock.pid, &lock.host) {
                self.conn.execute(
                    "DELETE FROM ipc_file_locks
                     WHERE file_pattern = ?1 AND agent = ?2 AND host = ?3",
                    params![lock.file_pattern, lock.agent, lock.host],
                )?;
                pruned += 1;
            }
        }
        Ok(pruned)
    }

    fn init_schema(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ipc_file_locks (
                file_pattern TEXT NOT NULL,
                agent TEXT NOT NULL,
                host TEXT NOT NULL,
                pid INTEGER NOT NULL,
                locked_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (file_pattern, agent, host)
            );
            CREATE INDEX IF NOT EXISTS idx_ipc_file_locks_pattern
                ON ipc_file_locks(file_pattern);",
        )
    }
}

fn is_pid_alive(pid: i64, host: &str) -> bool {
    let hostname = gethostname();
    if host != hostname {
        return true; // remote host — assume alive
    }
    #[cfg(unix)]
    {
        // kill(pid, 0) checks process existence without sending a signal
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

pub fn gethostname() -> String {
    hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

pub fn map_lock_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IpcFileLock> {
    Ok(IpcFileLock {
        file_pattern: row.get(0)?,
        agent: row.get(1)?,
        host: row.get(2)?,
        pid: row.get(3)?,
        locked_at: row.get(4)?,
    })
}

#[cfg(test)]
#[path = "manager_tests.rs"]
mod manager_tests;
