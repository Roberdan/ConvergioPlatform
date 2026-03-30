/// Timestamp-based sync adapter replacing crsqlite CRDT.
///
/// ADR: The libsql crate (v0.9) is async-only, requiring a tokio runtime for
/// all database operations. Since the daemon uses synchronous rusqlite in 80+
/// files, migrating to libsql would require rewriting the entire data layer.
/// Instead, we keep rusqlite and implement timestamp-based sync: each row has
/// an `updated_at` column; peers exchange rows newer than their last sync
/// checkpoint. This achieves the same eventual consistency goal with ~200 lines
/// of synchronous Rust instead of an external C extension (crsqlite).
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use super::libsql_adapter_helpers::{get_column_names, row_to_change};

/// Metadata tracking the last successful sync point per peer per table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncMeta {
    pub peer: String,
    pub table_name: String,
    pub last_sync_at: String,
}

/// A single row change to be replicated between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncChange {
    pub table_name: String,
    pub pk: i64,
    pub data: serde_json::Value,
}

/// Upsert sync metadata for a peer+table pair.
pub fn upsert_sync_meta(conn: &Connection, meta: &SyncMeta) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO _sync_meta (peer, table_name, last_sync_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT (peer, table_name)
         DO UPDATE SET last_sync_at = excluded.last_sync_at",
        params![meta.peer, meta.table_name, meta.last_sync_at],
    )?;
    Ok(())
}

/// Retrieve sync metadata for a peer+table pair. Returns None if no record.
pub fn get_sync_meta(
    conn: &Connection,
    peer: &str,
    table_name: &str,
) -> rusqlite::Result<Option<SyncMeta>> {
    conn.query_row(
        "SELECT peer, table_name, last_sync_at
         FROM _sync_meta
         WHERE peer = ?1 AND table_name = ?2",
        params![peer, table_name],
        |row| {
            Ok(SyncMeta {
                peer: row.get(0)?,
                table_name: row.get(1)?,
                last_sync_at: row.get(2)?,
            })
        },
    )
    .optional()
}

/// Export rows from `table_name` where `updated_at > since`.
///
/// When `since` is None, exports all rows (initial sync).
/// Requires the table to have `id INTEGER PRIMARY KEY` and `updated_at TEXT`.
pub fn export_changes_since(
    conn: &Connection,
    table_name: &str,
    since: Option<&str>,
) -> rusqlite::Result<Vec<SyncChange>> {
    // Validate table_name is alphanumeric+underscore to prevent injection
    if !table_name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(rusqlite::Error::InvalidParameterName(
            "invalid table name".to_string(),
        ));
    }

    let mut changes = Vec::new();
    let columns = get_column_names(conn, table_name)?;
    if columns.is_empty() {
        return Ok(changes);
    }

    let col_list = columns.join(", ");
    let query = if let Some(since_ts) = since {
        // Normalize: SQLite datetime('now') uses space separator, RFC3339 uses T.
        // Replace T with space so string comparison works consistently.
        let normalized = since_ts.replace('T', " ");
        let sql = format!(
            "SELECT id, {col_list} FROM \"{table_name}\" WHERE REPLACE(updated_at,'T',' ') > ?1 ORDER BY id"
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![normalized], |row| {
            row_to_change(row, table_name, &columns)
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    } else {
        let sql =
            format!("SELECT id, {col_list} FROM \"{table_name}\" ORDER BY id");
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            row_to_change(row, table_name, &columns)
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    changes.extend(query);
    Ok(changes)
}

// apply_changes → libsql_adapter_apply.rs
pub use super::libsql_adapter_apply::apply_changes;

#[cfg(test)]
#[path = "libsql_adapter_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "libsql_adapter_sync_tests.rs"]
mod sync_tests;
