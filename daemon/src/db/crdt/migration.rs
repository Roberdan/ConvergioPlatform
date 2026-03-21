use rusqlite::{params, Connection};

use super::required_crdt_tables;
use super::migration_helpers::{drop_sql_object_if_exists, rebuild_crr_compatible};

pub fn mark_required_tables(conn: &Connection) -> rusqlite::Result<()> {
    // Clean up any leftover temp tables from failed migrations
    let temps: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE '_crr_rebuild_%'",
        )?;
        let v: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(0))?.filter_map(|r| r.ok()).collect();
        v
    };
    for tmp in &temps {
        let _ = drop_sql_object_if_exists(conn, "TABLE", tmp);
    }
    let needs_migration: bool = required_crdt_tables().iter().any(|table| {
        let clock = format!("{table}__crsql_clock");
        let already: bool = conn
            .query_row(
                "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
                [&clock],
                |r| r.get(0),
            )
            .unwrap_or(false);
        !already
    });
    if !needs_migration {
        return Ok(());
    }
    // Save and drop ALL views and user triggers before rebuilding tables.
    // Views/triggers reference tables and crsqlite validates schema —
    // temporarily dropped tables cause errors during rebuild.
    let views: Vec<(String, String)> = {
        let mut stmt = conn.prepare("SELECT name, sql FROM sqlite_master WHERE type='view'")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
        rows.filter_map(|r| r.ok()).collect()
    };
    let triggers: Vec<(String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT name, sql FROM sqlite_master WHERE type='trigger' AND name NOT LIKE '%__crsql_%' AND sql IS NOT NULL"
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
        rows.filter_map(|r| r.ok()).collect()
    };
    for (name, _) in &views {
        let _ = drop_sql_object_if_exists(conn, "VIEW", name);
    }
    for (name, _) in &triggers {
        let _ = drop_sql_object_if_exists(conn, "TRIGGER", name);
    }
    for table in required_crdt_tables() {
        let clock_table = format!("{table}__crsql_clock");
        let already: bool = conn.query_row(
            "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
            [&clock_table],
            |r| r.get(0),
        )?;
        if already {
            continue;
        }
        let exists: bool = conn.query_row(
            "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |r| r.get(0),
        )?;
        if !exists {
            continue;
        }
        if conn
            .query_row("SELECT crsql_as_crr(?1)", params![table], |_| Ok(()))
            .is_ok()
        {
            continue;
        }
        drop_unique_indices(conn, table)?;
        if conn
            .query_row("SELECT crsql_as_crr(?1)", params![table], |_| Ok(()))
            .is_ok()
        {
            continue;
        }
        rebuild_crr_compatible(conn, table)?;
        conn.query_row("SELECT crsql_as_crr(?1)", params![table], |_| Ok(()))?;
    }
    // Restore views and triggers
    for (_, sql) in &views {
        let _ = conn.execute_batch(sql);
    }
    for (_, sql) in &triggers {
        let _ = conn.execute_batch(sql);
    }
    Ok(())
}

fn drop_unique_indices(conn: &Connection, table: &str) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='index' AND tbl_name=?1 AND sql LIKE '%UNIQUE%'"
    )?;
    let indices: Vec<String> = stmt.query_map([table], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok()).collect();
    for idx in &indices {
        drop_sql_object_if_exists(conn, "INDEX", idx)?;
    }
    Ok(())
}

