use std::io::{Error as IoError, ErrorKind, Write};
use std::process::{Command, Stdio};

use rusqlite::params;

use crate::db::PlanDb;

use super::{CrdtChange, SyncSummary};

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
            applied += 1;
        }
        Ok(applied)
    }

    pub(crate) fn sync_with_peer(&self, peer: &str) -> rusqlite::Result<SyncSummary> {
        let local = self.export_changes()?;
        let remote = self.fetch_remote_changes(peer).map_err(io_as_sql_error)?;
        let applied = self.apply_changes(&remote)?;
        self.send_local_changes(peer, &local)
            .map_err(io_as_sql_error)?;
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
        if let Some(ext) = &self.crsqlite_extension {
            cmd.arg("--crsqlite-path").arg(ext);
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
        if let Some(ext) = &self.crsqlite_extension {
            cmd.arg("--crsqlite-path").arg(ext);
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

pub fn io_as_sql_error(err: std::io::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(err))
}
