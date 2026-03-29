// crsqlite-dependent sync methods: export/apply/sync via crsql_changes table.
// When crsqlite feature is disabled, sync is handled by libsql_adapter instead.

#[cfg(feature = "crsqlite")]
mod crsqlite_sync {
    use std::io::{Error as IoError, ErrorKind, Write};
    use std::process::{Command, Stdio};

    use rusqlite::params;
    use rusqlite::OptionalExtension;

    use crate::db::crdt::{CrdtChange, SyncSummary};
    use crate::db::PlanDb;

    impl PlanDb {
        pub(crate) fn export_changes(&self) -> rusqlite::Result<Vec<CrdtChange>> {
            let mut stmt = self.conn.prepare(
                r#"SELECT "table", pk, cid, CAST(val AS TEXT), col_version, db_version, site_id, cl, seq
                   FROM crsql_changes"#,
            )?;
            let rows = stmt.query_map([], |row| {
                Ok(CrdtChange {
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

        pub(crate) fn apply_changes(&self, changes: &[CrdtChange]) -> rusqlite::Result<usize> {
            let mut applied = 0usize;
            for change in changes {
                // Idempotency guard: skip if a row with the same (table, pk, cid,
                // site_id) already has an equal or higher col_version.
                let existing_version: Option<i64> = self
                    .conn
                    .query_row(
                        r#"SELECT col_version FROM crsql_changes
                           WHERE "table" = ?1 AND pk = ?2 AND cid = ?3 AND site_id = ?4
                           LIMIT 1"#,
                        params![change.table_name, change.pk, change.cid, change.site_id],
                        |row| row.get(0),
                    )
                    .optional()?;

                if let Some(existing) = existing_version {
                    if existing >= change.col_version {
                        continue;
                    }
                    self.conn.execute(
                        r#"UPDATE crsql_changes
                           SET val = ?4, col_version = ?5, db_version = ?6, cl = ?7, seq = ?8
                           WHERE "table" = ?1 AND pk = ?2 AND cid = ?3 AND site_id = ?9"#,
                        params![
                            change.table_name,
                            change.pk,
                            change.cid,
                            change.val,
                            change.col_version,
                            change.db_version,
                            change.cl,
                            change.seq,
                            change.site_id,
                        ],
                    )?;
                } else {
                    self.conn.execute(
                        r#"INSERT INTO crsql_changes ("table", pk, cid, val, col_version, db_version, site_id, cl, seq)
                           VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
                        params![
                            change.table_name,
                            change.pk,
                            change.cid,
                            change.val,
                            change.col_version,
                            change.db_version,
                            change.site_id,
                            change.cl,
                            change.seq
                        ],
                    )?;
                }
                applied += 1;
            }
            Ok(applied)
        }

        pub(crate) fn sync_with_peer(&self, peer: &str) -> rusqlite::Result<SyncSummary> {
            let local = self.export_changes()?;
            let remote = self.fetch_remote_changes(peer).map_err(super::io_as_sql_error)?;
            let applied = self.apply_changes(&remote)?;
            self.send_local_changes(peer, &local)
                .map_err(super::io_as_sql_error)?;
            Ok(SyncSummary {
                peer: peer.to_string(),
                sent: local.len(),
                received: remote.len(),
                applied,
            })
        }

        fn fetch_remote_changes(&self, peer: &str) -> std::io::Result<Vec<CrdtChange>> {
            let mut cmd = Command::new("ssh");
            cmd.arg(peer)
                .arg("claude-core")
                .arg("db")
                .arg("export-changes");
            if let Some(path) = &self.db_path {
                cmd.arg("--db-path").arg(path);
            }
            let output = cmd.output()?;
            if !output.status.success() {
                return Err(IoError::other(format!(
                    "remote export failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }
            serde_json::from_slice::<Vec<CrdtChange>>(&output.stdout)
                .map_err(|err| IoError::new(ErrorKind::InvalidData, err))
        }

        fn send_local_changes(&self, peer: &str, changes: &[CrdtChange]) -> std::io::Result<()> {
            let payload = serde_json::to_vec(changes)
                .map_err(|err| IoError::new(ErrorKind::InvalidData, err.to_string()))?;
            let mut cmd = Command::new("ssh");
            cmd.arg(peer)
                .arg("claude-core")
                .arg("db")
                .arg("apply-changes");
            if let Some(path) = &self.db_path {
                cmd.arg("--db-path").arg(path);
            }
            cmd.stdin(Stdio::piped());
            let mut child = cmd.spawn()?;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(&payload)?;
            }
            let status = child.wait()?;
            if status.success() {
                Ok(())
            } else {
                Err(IoError::other("remote apply failed"))
            }
        }
    }
}

pub fn io_as_sql_error(err: std::io::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(err))
}

/// Increment consecutive_failures for a peer.
///
/// When the counter reaches 3, the peer is marked 'unreachable' so the
/// background sync loop (background_sync.rs) stops attempting to contact it.
// Used by mesh background sync loop and tests; not yet called by lib code.
#[allow(dead_code)]
pub fn record_sync_failure(conn: &rusqlite::Connection, peer: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE mesh_sync_stats
            SET consecutive_failures = consecutive_failures + 1,
                status = CASE WHEN consecutive_failures + 1 >= 3 THEN 'unreachable' ELSE status END
          WHERE peer_name = ?1",
        rusqlite::params![peer],
    )?;
    Ok(())
}

/// Reset consecutive_failures to 0 and mark a peer 'online' after a successful sync.
// Used by mesh background sync loop and tests; not yet called by lib code.
#[allow(dead_code)]
pub fn record_sync_success(conn: &rusqlite::Connection, peer: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE mesh_sync_stats
            SET consecutive_failures = 0,
                status = 'online'
          WHERE peer_name = ?1",
        rusqlite::params![peer],
    )?;
    Ok(())
}
