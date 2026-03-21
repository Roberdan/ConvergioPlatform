// Mesh sync operations: collect changes, apply delta frames, record stats.

use rusqlite::{params, Connection};
use std::path::Path;

use super::conn::{ensure_sync_schema, now_ms, open_sync_conn, validate_peer_name};
use super::ops_apply::{apply_changes_to_conn, read_local_changes_since};
use super::types::{ApplySummary, DeltaChange};

pub fn collect_changes_since(
    db_path: &Path,
    crsqlite_ext: Option<&str>,
    last_db_version: i64,
) -> Result<(Vec<DeltaChange>, i64), String> {
    let conn = open_sync_conn(db_path, crsqlite_ext)?;
    ensure_sync_schema(&conn).map_err(|e| e.to_string())?;
    let changes = read_local_changes_since(&conn, last_db_version).map_err(|e| e.to_string())?;
    let max_db_version = changes
        .iter()
        .map(|c| c.db_version)
        .max()
        .unwrap_or(last_db_version);
    Ok((changes, max_db_version))
}

/// Get the current max db_version — used to initialize cursor on startup
pub fn current_db_version(db_path: &Path, crsqlite_ext: Option<&str>) -> Result<i64, String> {
    let conn = open_sync_conn(db_path, crsqlite_ext)?;
    conn.query_row(
        "SELECT COALESCE(MAX(db_version), 0) FROM crsql_changes",
        [],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

pub fn apply_delta_frame(
    db_path: &Path,
    crsqlite_ext: Option<&str>,
    peer_name: &str,
    sent_at_ms: u64,
    changes: &[DeltaChange],
) -> Result<ApplySummary, String> {
    validate_peer_name(peer_name)?;
    let conn = open_sync_conn(db_path, crsqlite_ext)?;
    ensure_sync_schema(&conn).map_err(|e| e.to_string())?;
    let applied = apply_changes_to_conn(&conn, changes).map_err(|e| e.to_string())?;
    let latency = now_ms().saturating_sub(sent_at_ms);
    let last_db_version = changes.iter().map(|c| c.db_version).max().unwrap_or(0);
    conn.execute(
        "INSERT INTO mesh_sync_stats(peer_name,total_received,total_applied,last_sync_at,last_latency_ms,last_db_version,last_error)
         VALUES(?1, ?2, ?3, strftime('%s','now'), ?4, ?5, NULL)
         ON CONFLICT(peer_name) DO UPDATE SET
           total_received = total_received + excluded.total_received,
           total_applied = total_applied + excluded.total_applied,
           last_sync_at = excluded.last_sync_at,
           last_latency_ms = excluded.last_latency_ms,
           last_db_version = MAX(mesh_sync_stats.last_db_version, excluded.last_db_version),
           last_error = NULL",
        params![peer_name, changes.len() as i64, applied as i64, latency as i64, last_db_version],
    )
    .map_err(|e| e.to_string())?;
    Ok(ApplySummary {
        applied,
        latency_ms: latency,
        last_db_version,
    })
}

pub fn record_sent_stats(
    db_path: &Path,
    crsqlite_ext: Option<&str>,
    peer_name: &str,
    sent_count: usize,
    last_db_version: i64,
) -> Result<(), String> {
    validate_peer_name(peer_name)?;
    let conn = open_sync_conn(db_path, crsqlite_ext)?;
    ensure_sync_schema(&conn).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO mesh_sync_stats(peer_name,total_sent,last_sent_at,last_db_version,last_error)
         VALUES(?1, ?2, strftime('%s','now'), ?3, NULL)
         ON CONFLICT(peer_name) DO UPDATE SET
           total_sent = total_sent + excluded.total_sent,
           last_sent_at = excluded.last_sent_at,
           last_db_version = MAX(mesh_sync_stats.last_db_version, excluded.last_db_version),
           last_error = NULL",
        params![peer_name, sent_count as i64, last_db_version],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn record_sync_error(
    db_path: &Path,
    crsqlite_ext: Option<&str>,
    peer_name: &str,
    error: &str,
) -> Result<(), String> {
    validate_peer_name(peer_name)?;
    let conn = open_sync_conn(db_path, crsqlite_ext)?;
    ensure_sync_schema(&conn).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO mesh_sync_stats(peer_name,last_error,last_sync_at)
         VALUES(?1, ?2, strftime('%s','now'))
         ON CONFLICT(peer_name) DO UPDATE SET
           last_error = excluded.last_error,
           last_sync_at = excluded.last_sync_at",
        params![peer_name, error],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Collect local changes using an existing connection (avoids opening new one per tick)
pub fn collect_changes_with_conn(
    conn: &Connection,
    last_db_version: i64,
) -> Result<(Vec<DeltaChange>, i64), String> {
    let changes = read_local_changes_since(conn, last_db_version).map_err(|e| e.to_string())?;
    let max_db_version = changes
        .iter()
        .map(|c| c.db_version)
        .max()
        .unwrap_or(last_db_version);
    Ok((changes, max_db_version))
}

/// Record sent stats using an existing connection
pub fn record_sent_stats_with_conn(
    conn: &Connection,
    peer_name: &str,
    sent_count: usize,
    last_db_version: i64,
) -> Result<(), String> {
    validate_peer_name(peer_name)?;
    ensure_sync_schema(conn).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO mesh_sync_stats(peer_name,total_sent,last_sent_at,last_db_version,last_error)
         VALUES(?1, ?2, strftime('%s','now'), ?3, NULL)
         ON CONFLICT(peer_name) DO UPDATE SET
           total_sent = total_sent + excluded.total_sent,
           last_sent_at = excluded.last_sent_at,
           last_db_version = MAX(mesh_sync_stats.last_db_version, excluded.last_db_version),
           last_error = NULL",
        params![peer_name, sent_count as i64, last_db_version],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn current_db_version_with_conn(conn: &Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(db_version), 0) FROM crsql_changes",
        [],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

pub use super::ops_apply::read_changes_since_from_conn;
