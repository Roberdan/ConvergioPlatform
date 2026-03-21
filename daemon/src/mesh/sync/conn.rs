// Database connection setup and schema management for mesh sync.

use rusqlite::Connection;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::types::MAX_PEER_NAME_LEN;

/// Open a new sync connection (WAL mode, cr-sqlite extension if available).
pub fn open_sync_conn(db_path: &Path, crsqlite_ext: Option<&str>) -> Result<Connection, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA busy_timeout=5000;
         PRAGMA cache_size=-2000;",
    )
    .map_err(|e| e.to_string())?;
    if let Some(ext) = crsqlite_ext {
        match (|| -> Result<(), String> {
            unsafe { conn.load_extension_enable() }.map_err(|e| e.to_string())?;
            unsafe { conn.load_extension(ext, None::<&str>) }.map_err(|e| e.to_string())?;
            crate::db::crdt::mark_required_tables(&conn).map_err(|e| e.to_string())?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(e) => {
                eprintln!("[warn] crsqlite failed (SQLite {ext} vs system mismatch?): {e}");
                eprintln!(
                    "[warn] daemon running WITHOUT CRDT replication — heartbeat/sync still active"
                );
            }
        }
    }
    Ok(conn)
}

/// Open a long-lived sync connection (call once, reuse across ticks).
pub fn open_persistent_sync_conn(
    db_path: &Path,
    crsqlite_ext: Option<&str>,
) -> Result<Connection, String> {
    open_sync_conn(db_path, crsqlite_ext)
}

pub fn ensure_sync_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS mesh_sync_stats (
            peer_name TEXT PRIMARY KEY,
            total_sent INTEGER NOT NULL DEFAULT 0,
            total_received INTEGER NOT NULL DEFAULT 0,
            total_applied INTEGER NOT NULL DEFAULT 0,
            last_sent_at INTEGER,
            last_sync_at INTEGER,
            last_latency_ms INTEGER,
            last_db_version INTEGER NOT NULL DEFAULT 0,
            last_error TEXT
        );",
    )
}

pub fn ensure_sync_schema_pub(conn: &Connection) -> rusqlite::Result<()> {
    ensure_sync_schema(conn)
}

pub fn validate_peer_name(peer_name: &str) -> Result<(), String> {
    if peer_name.is_empty() || peer_name.len() > MAX_PEER_NAME_LEN {
        return Err(format!("invalid peer name length: {}", peer_name.len()));
    }
    Ok(())
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
