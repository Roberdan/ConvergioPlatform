// Progress tracking for delegation pipeline stages.
// Writes to delegation_progress table so CLI/API can poll status.

use std::path::Path;
use tracing::{debug, warn};

/// Record a pipeline stage into the delegation_progress table.
/// Uses UPSERT so each delegation_id has exactly one row that gets updated.
/// `step` goes into current_task, `status` must be running|blocked|done.
pub fn record_step(
    db_path: &Path,
    delegation_id: &str,
    step: &str,
    status: &str,
    summary: Option<&str>,
) {
    let conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            warn!("delegate progress: cannot open db: {e}");
            return;
        }
    };
    if let Err(e) = conn.execute_batch("PRAGMA busy_timeout = 5000;") {
        warn!("delegate progress: pragma failed: {e}");
        return;
    }
    let sql = "INSERT INTO delegation_progress
         (delegation_id, status, current_task, output_summary, updated_at)
     VALUES (?1, ?2, ?3, ?4, datetime('now'))
     ON CONFLICT(delegation_id) DO UPDATE SET
         status         = excluded.status,
         current_task   = excluded.current_task,
         output_summary = COALESCE(excluded.output_summary, output_summary),
         updated_at     = excluded.updated_at";
    match conn.execute(sql, rusqlite::params![delegation_id, status, step, summary]) {
        Ok(_) => debug!(delegation_id, step, status, "progress recorded"),
        Err(e) => warn!(delegation_id, step, "progress insert failed: {e}"),
    }
}
