// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Project path canonicalization — run once on daemon startup.

use rusqlite::Connection;

/// Normalise any existing project paths stored without canonicalization.
///
/// For each project whose path exists on disk, std::fs::canonicalize resolves symlinks
/// so macOS case-sensitivity bugs (e.g. /Users/foo vs /users/foo on HFS+) are fixed.
/// Paths that no longer exist on disk are left unchanged.
pub fn canonicalize_existing_project_paths(conn: &Connection) {
    // Check projects table exists before querying — may not exist on first boot.
    let table_ok: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='projects'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);
    if !table_ok {
        return;
    }

    let rows: Vec<(String, String)> = {
        let mut stmt = match conn
            .prepare("SELECT id, path FROM projects WHERE path IS NOT NULL AND path != ''")
        {
            Ok(s) => s,
            Err(e) => { eprintln!("[migration] canonicalize projects prepare failed: {e}"); return; }
        };
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .and_then(|mapped| mapped.collect())
            .unwrap_or_default()
    };

    let mut fixed = 0usize;
    for (id, raw_path) in rows {
        if let Ok(canonical) = std::fs::canonicalize(&raw_path) {
            if let Ok(canonical_str) = canonical.into_os_string().into_string() {
                if canonical_str != raw_path {
                    if let Err(e) = conn.execute(
                        "UPDATE projects SET path = ?1 WHERE id = ?2",
                        rusqlite::params![canonical_str, id],
                    ) {
                        eprintln!("[migration] canonicalize path update failed for {id}: {e}");
                    } else {
                        fixed += 1;
                    }
                }
            }
        }
    }
    if fixed > 0 {
        eprintln!("[migration] canonicalized {fixed} project path(s)");
    }
}
