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

/// Resolve the crsqlite extension path by searching standard locations.
///
/// Search order (first existing file wins):
///   1. Caller-supplied explicit path (when `hint` is `Some`)
///   2. `~/lib/crsqlite.{dylib|so}` — user-local install (e.g. M1 Pro)
///   3. `~/.claude/lib/crsqlite/crsqlite.{dylib|so}` — mesh-provisioned location
///   4. Bare name `"crsqlite"` — fall back to OS dynamic-linker search path
pub fn resolve_crsqlite_path(hint: Option<String>) -> String {
    if let Some(p) = hint {
        return p;
    }
    let ext = if cfg!(target_os = "macos") { "dylib" } else { "so" };
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let candidates = [
        format!("{home}/lib/crsqlite.{ext}"),
        format!("{home}/.claude/lib/crsqlite/crsqlite.{ext}"),
    ];
    for candidate in &candidates {
        if std::path::Path::new(candidate).exists() {
            return candidate.clone();
        }
    }
    // Fall back to bare name; OS linker (DYLD_LIBRARY_PATH / LD_LIBRARY_PATH) will resolve.
    "crsqlite".to_string()
}

pub struct PlanDb {
    conn: Connection,
    db_path: Option<PathBuf>,
    crsqlite_extension: Option<String>,
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

    // Replicate previous behavior: 100ms → 500ms → 2000ms ≈ factor 4-5
    // RetryConfig: max_retries = max_attempts - 1 (first attempt is free)
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
            crsqlite_extension: None,
        })
    }

    pub fn open_path(path: &Path, crsqlite_extension: Option<String>) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        let extension = resolve_crsqlite_path(crsqlite_extension);
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
