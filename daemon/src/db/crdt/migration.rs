use rusqlite::{params, Connection};

use super::required_crdt_tables;

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

pub fn quote_identifier(conn: &Connection, ident: &str) -> rusqlite::Result<String> {
    conn.query_row("SELECT printf('\"%w\"', ?1)", params![ident], |row| {
        row.get(0)
    })
}

pub fn drop_sql_object_if_exists(
    conn: &Connection,
    object_type: &str,
    object_name: &str,
) -> rusqlite::Result<()> {
    let quoted_name = quote_identifier(conn, object_name)?;
    let drop_sql: String = conn.query_row(
        "SELECT printf('DROP %s IF EXISTS %s', ?1, ?2)",
        params![object_type, quoted_name],
        |row| row.get(0),
    )?;
    conn.execute_batch(&drop_sql)
}

/// Rebuild table to be CRR-compatible:
/// 1. Remove UNIQUE constraints
/// 2. Add DEFAULT values to NOT NULL columns (crsqlite requires this)
pub fn rebuild_crr_compatible(conn: &Connection, table: &str) -> rusqlite::Result<()> {
    // Get column info
    let mut cols: Vec<(String, String, bool, Option<String>, bool)> = Vec::new();
    {
        let mut stmt = conn.prepare(&format!(
            "SELECT name, type, \"notnull\", dflt_value, pk FROM pragma_table_info('{}')",
            table.replace('\'', "''")
        ))?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, bool>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })?;
        let v: Vec<_> = rows.collect();
        for row in v {
            cols.push(row?);
        }
    }
    // Get FK info
    let mut fks: Vec<(String, String, String)> = Vec::new();
    {
        let mut stmt =
            conn.prepare("SELECT \"table\", \"from\", \"to\" FROM pragma_foreign_key_list(?1)")?;
        let rows = stmt.query_map(params![table], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let v: Vec<_> = rows.collect();
        for row in v {
            fks.push(row?);
        }
    }
    // Get CHECK constraints from original SQL
    let original_sql: String = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
        [table],
        |r| r.get(0),
    )?;
    // Build new CREATE TABLE
    let tmp = format!("_crr_rebuild_{table}");
    let mut col_defs: Vec<String> = Vec::new();
    for (name, typ, notnull, dflt, pk) in &cols {
        let mut def = format!("\"{}\" {}", name, typ);
        if *pk {
            def.push_str(" PRIMARY KEY");
            // NOTE: AUTOINCREMENT is intentionally NOT added for CRR tables.
            // crsqlite requires coordinated PKs; AUTOINCREMENT causes ID conflicts
            // between nodes. Bare INTEGER PRIMARY KEY still auto-assigns rowid.
            def.push_str(" NOT NULL");
        }
        if *notnull && !pk {
            def.push_str(" NOT NULL");
            if dflt.is_none() {
                let default = default_for_type(typ);
                def.push_str(&format!(" DEFAULT {default}"));
            }
        }
        if let Some(d) = dflt {
            if !pk {
                // Expression defaults (containing function calls) need parentheses
                if d.contains('(') {
                    def.push_str(&format!(" DEFAULT ({d})"));
                } else {
                    def.push_str(&format!(" DEFAULT {d}"));
                }
            }
        }
        // Extract CHECK constraint for this column from original SQL
        let upper_orig = original_sql.to_uppercase();
        let check_needle = format!("\"{}\"", name.to_uppercase());
        if let Some(pos) = upper_orig.find(&check_needle) {
            let rest = &original_sql[pos..];
            if let Some(check_start) = rest.to_uppercase().find("CHECK(") {
                let check_rest = &rest[check_start..];
                if let Some(end) = find_matching_paren(check_rest, 5) {
                    def.push_str(&format!(" {}", &check_rest[..=end]));
                }
            }
        }
        col_defs.push(def);
    }
    // NOTE: Foreign keys are intentionally NOT added for CRR tables.
    // crsqlite does not allow checked FK constraints in CRR tables because
    // replication can temporarily violate referential integrity.
    let _ = fks; // consumed above intentionally
    let create = format!("CREATE TABLE \"{}\" ({})", tmp, col_defs.join(", "));
    // Use SAVEPOINT for atomicity — if any step fails, rollback all changes
    match conn.execute_batch(&format!(
        "SAVEPOINT crr_rebuild; {}; INSERT INTO \"{}\" SELECT * FROM \"{}\"; DROP TABLE \"{}\"; ALTER TABLE \"{}\" RENAME TO \"{}\"; RELEASE crr_rebuild;",
        create, tmp, table, table, tmp, table
    )) {
        Ok(()) => {},
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK TO crr_rebuild; RELEASE crr_rebuild;");
            let _ = conn.execute_batch(&format!("DROP TABLE IF EXISTS \"{}\"", tmp));
            return Err(e);
        }
    };
    Ok(())
}

fn default_for_type(typ: &str) -> &'static str {
    let upper = typ.to_uppercase();
    if upper.contains("INT") {
        "'0'"
    } else if upper.contains("REAL") || upper.contains("FLOAT") || upper.contains("DOUBLE") {
        "'0.0'"
    } else if upper.contains("BOOL") {
        "'0'"
    } else {
        "''"
    } // TEXT, BLOB, JSON, DATETIME, etc.
}

fn find_matching_paren(s: &str, open_pos: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0;
    for (i, &b) in bytes.iter().enumerate().skip(open_pos) {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}
