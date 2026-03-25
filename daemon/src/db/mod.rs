mod cli;
pub mod crdt;
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
    crsqlite_extension: Option<String>,
}

/// Retry a SQLite write operation up to `max_attempts` times when `SQLITE_BUSY` is returned.
///
/// Delays: 100ms, 500ms, 2000ms (with a small fixed jitter slice per attempt).
/// Non-BUSY errors are returned immediately without retrying.
pub fn with_retry<T, F>(max_attempts: u32, mut f: F) -> rusqlite::Result<T>
where
    F: FnMut() -> rusqlite::Result<T>,
{
    // Delay sequence in milliseconds for attempts 1, 2, 3+
    const DELAYS_MS: &[u64] = &[100, 500, 2000];
    let mut last_err = None;

    for attempt in 0..max_attempts {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => {
                if is_busy_error(&e) {
                    last_err = Some(e);
                    // Wait before retry — skip sleep on last attempt
                    if attempt + 1 < max_attempts {
                        let idx = (attempt as usize).min(DELAYS_MS.len() - 1);
                        let delay = DELAYS_MS[idx];
                        std::thread::sleep(std::time::Duration::from_millis(delay));
                    }
                } else {
                    // Non-BUSY errors propagate immediately
                    return Err(e);
                }
            }
        }
    }

    Err(last_err.expect("at least one attempt was made"))
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
            crsqlite_extension: None,
        })
    }

    pub fn open_path(path: &Path, crsqlite_extension: Option<String>) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        let extension = crsqlite_extension.unwrap_or_else(|| "crsqlite".to_string());
        let mut loaded_ext = None;
        match crdt::load_crsqlite(&conn, &extension) {
            Ok(()) => match crdt::mark_required_tables(&conn) {
                Ok(()) => {
                    loaded_ext = Some(extension);
                }
                Err(e) => {
                    eprintln!("[warn] crsqlite loaded but CRR setup failed (SQLite version mismatch?): {e}");
                    eprintln!("[warn] daemon running WITHOUT CRDT replication — heartbeat/sync still active");
                }
            },
            Err(e) => {
                eprintln!("[warn] crsqlite extension failed to load: {e}");
                eprintln!(
                    "[warn] daemon running WITHOUT CRDT replication — heartbeat/sync still active"
                );
            }
        }
        // Apply standard pragmas even without crsqlite
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
        Ok(Self {
            conn,
            db_path: Some(path.to_path_buf()),
            crsqlite_extension: loaded_ext,
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
            crsqlite_extension: None,
        })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests;
