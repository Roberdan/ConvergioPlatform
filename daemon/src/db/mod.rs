mod cli;
pub mod libsql_adapter;
pub mod migrations;
mod models;
pub mod plan_hierarchy;
mod queries;
pub mod seed_agents;
mod service;

pub use models::{
    ActivePlan, ExecutionTaskNode, ExecutionTree, ExecutionWaveNode, InProgressTask, StatusView,
    TaskStatus, UpdateTaskArgs, UpdateTaskResult, ValidateTaskArgs, ValidateTaskResult,
};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub struct PlanDb {
    conn: Connection,
    db_path: Option<PathBuf>,
}

/// Retry a SQLite write operation up to `max_attempts` times when `SQLITE_BUSY` is returned.
///
/// Delegates to `resilience::retry::retry_sync` with exponential backoff.
/// Non-BUSY errors are returned immediately without retrying.
pub fn with_retry<T, F>(max_attempts: u32, f: F) -> rusqlite::Result<T>
where
    F: FnMut() -> rusqlite::Result<T>,
{
    use crate::resilience::retry::{RetryConfig, retry_sync};
    use std::time::Duration;

    let retries = max_attempts.saturating_sub(1);
    retry_sync(
        f,
        RetryConfig {
            max_retries: retries,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(2000),
            backoff_factor: 5.0,
            jitter: false,
        },
        is_busy_error,
    )
}

/// Returns true if the rusqlite error is SQLITE_BUSY or SQLITE_LOCKED.
fn is_busy_error(e: &rusqlite::Error) -> bool {
    match e {
        rusqlite::Error::SqliteFailure(err, _) => {
            matches!(
                err.code,
                rusqlite::ffi::ErrorCode::DatabaseBusy
                    | rusqlite::ffi::ErrorCode::DatabaseLocked
            )
        }
        _ => false,
    }
}

impl PlanDb {
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        Ok(Self {
            conn: Connection::open_in_memory()?,
            db_path: None,
        })
    }

    /// Open a database at the given path with standard pragmas.
    ///
    /// crsqlite is no longer loaded. Sync is handled by the timestamp-based
    /// adapter in `libsql_adapter` module.
    pub fn open_path(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
        Ok(Self {
            conn,
            db_path: Some(path.to_path_buf()),
        })
    }

    pub fn open_sqlite_path(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-8000;
             PRAGMA mmap_size=67108864;
             PRAGMA temp_store=MEMORY;",
        )?;
        Ok(Self {
            conn,
            db_path: Some(path.to_path_buf()),
        })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests;
