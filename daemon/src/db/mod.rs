mod cli;
pub mod crdt;
mod models;
mod queries;
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
            Ok(()) => {
                match crdt::mark_required_tables(&conn) {
                    Ok(()) => { loaded_ext = Some(extension); }
                    Err(e) => {
                        eprintln!("[warn] crsqlite loaded but CRR setup failed (SQLite version mismatch?): {e}");
                        eprintln!("[warn] daemon running WITHOUT CRDT replication — heartbeat/sync still active");
                    }
                }
            }
            Err(e) => {
                eprintln!("[warn] crsqlite extension failed to load: {e}");
                eprintln!("[warn] daemon running WITHOUT CRDT replication — heartbeat/sync still active");
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
