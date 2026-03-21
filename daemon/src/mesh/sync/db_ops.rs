use rusqlite::{params, Connection};
use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::types::{ApplySummary, DeltaChange, MAX_PEER_NAME_LEN};

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

/// Read only LOCAL changes (filtered by this node's site_id) — prevents re-broadcast loops
fn read_local_changes_since(
    conn: &Connection,
    last_db_version: i64,
) -> rusqlite::Result<Vec<DeltaChange>> {
    // Get local site_id to filter — only send OUR writes, not re-broadcast received changes
    let local_site_id: Option<Vec<u8>> = conn
        .query_row("SELECT site_id FROM crsql_site_id LIMIT 1", [], |r| {
            r.get(0)
        })
        .ok();
    let mut stmt = conn.prepare(
        r#"SELECT "table", pk, cid, CAST(val AS TEXT), col_version, db_version, site_id, cl, seq
           FROM crsql_changes
           WHERE db_version > ?1 AND (?2 IS NULL OR site_id = ?2)
           ORDER BY db_version ASC, seq ASC
           LIMIT 1000"#,
    )?;
    let rows = stmt.query_map(params![last_db_version, local_site_id], |row| {
        Ok(DeltaChange {
            table_name: row.get(0)?,
            pk: row.get(1)?,
            cid: row.get(2)?,
            val: row.get(3)?,
            col_version: row.get(4)?,
            db_version: row.get(5)?,
            site_id: row.get(6)?,
            cl: row.get(7)?,
            seq: row.get(8)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
}

pub fn read_changes_since_from_conn(
    conn: &Connection,
    last_db_version: i64,
) -> rusqlite::Result<Vec<DeltaChange>> {
    let mut stmt = conn.prepare(
        r#"SELECT "table", pk, cid, CAST(val AS TEXT), col_version, db_version, site_id, cl, seq
           FROM crsql_changes
           WHERE db_version > ?1
           ORDER BY db_version ASC, seq ASC"#,
    )?;
    let rows = stmt.query_map([last_db_version], |row| {
        Ok(DeltaChange {
            table_name: row.get(0)?,
            pk: row.get(1)?,
            cid: row.get(2)?,
            val: row.get(3)?,
            col_version: row.get(4)?,
            db_version: row.get(5)?,
            site_id: row.get(6)?,
            cl: row.get(7)?,
            seq: row.get(8)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
}

fn apply_changes_to_conn(conn: &Connection, changes: &[DeltaChange]) -> rusqlite::Result<usize> {
    if changes.is_empty() {
        return Ok(0);
    }
    // T1-08: CRDT table allowlist — only apply changes for known CRR tables
    let allowed = get_crr_table_allowlist(conn);
    let valid: Vec<&DeltaChange> = changes
        .iter()
        .filter(|c| allowed.contains(&c.table_name))
        .collect();
    if valid.is_empty() {
        return Ok(0);
    }
    conn.execute_batch("BEGIN")?;
    let mut applied = 0;
    let result = (|| -> rusqlite::Result<usize> {
        let mut stmt = conn.prepare_cached(
            r#"INSERT INTO crsql_changes ("table", pk, cid, val, col_version, db_version, site_id, cl, seq)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
        )?;
        for change in &valid {
            stmt.execute(params![
                change.table_name,
                change.pk,
                change.cid,
                change.val,
                change.col_version,
                change.db_version,
                change.site_id,
                change.cl,
                change.seq
            ])?;
            applied += 1;
        }
        Ok(applied)
    })();
    match result {
        Ok(count) => {
            conn.execute_batch("COMMIT")?;
            Ok(count)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

/// Query local DB for CRR-tracked tables (tables with __crsql_clock counterpart)
fn get_crr_table_allowlist(conn: &Connection) -> HashSet<String> {
    let mut set = HashSet::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '%\\_\\_crsql\\_clock' ESCAPE '\\'"
    ) {
        if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
            for name in rows.flatten() {
                if let Some(table) = name.strip_suffix("__crsql_clock") {
                    set.insert(table.to_string());
                }
            }
        }
    }
    set
}

/// Open a long-lived sync connection (call once, reuse across ticks)
pub fn open_persistent_sync_conn(
    db_path: &Path,
    crsqlite_ext: Option<&str>,
) -> Result<Connection, String> {
    open_sync_conn(db_path, crsqlite_ext)
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

pub fn ensure_sync_schema_pub(conn: &Connection) -> rusqlite::Result<()> {
    ensure_sync_schema(conn)
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

fn validate_peer_name(peer_name: &str) -> Result<(), String> {
    if peer_name.is_empty() || peer_name.len() > MAX_PEER_NAME_LEN {
        return Err(format!("invalid peer name length: {}", peer_name.len()));
    }
    Ok(())
}

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

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
