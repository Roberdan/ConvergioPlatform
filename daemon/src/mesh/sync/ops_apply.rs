// Mesh sync apply operations: read changes, apply delta frames, CRR allowlist.

use rusqlite::{params, Connection};
use std::collections::HashSet;

use super::types::DeltaChange;

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

/// Read only LOCAL changes (filtered by this node's site_id) — prevents re-broadcast loops
pub(super) fn read_local_changes_since(
    conn: &Connection,
    last_db_version: i64,
) -> rusqlite::Result<Vec<DeltaChange>> {
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

pub(super) fn apply_changes_to_conn(
    conn: &Connection,
    changes: &[DeltaChange],
) -> rusqlite::Result<usize> {
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
        "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '%\\_\\_crsql\\_clock' ESCAPE '\\'",
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
