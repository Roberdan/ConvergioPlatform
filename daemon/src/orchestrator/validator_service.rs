// Thor validator service — durable queue + persistent verdicts.
// Why: inline validation has no retry, no persistence, no timeout handling.

use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};

// ── Migrations ──────────────────────────────────────────────────────────────

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS validation_queue (
            id          INTEGER PRIMARY KEY,
            task_id     INTEGER,
            wave_id     INTEGER,
            plan_id     INTEGER,
            status      TEXT    NOT NULL DEFAULT 'pending',
            created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
            started_at  TEXT,
            completed_at TEXT
        );
        CREATE TABLE IF NOT EXISTS validation_verdicts (
            id          INTEGER PRIMARY KEY,
            queue_id    INTEGER NOT NULL,
            verdict     TEXT    NOT NULL,
            report      TEXT,
            validator   TEXT,
            created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS audit_log (
            id          INTEGER PRIMARY KEY,
            action      TEXT    NOT NULL,
            entity_type TEXT,
            entity_id   INTEGER,
            actor       TEXT,
            details     TEXT,
            created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
        );",
    )
}

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub id: i64,
    pub task_id: Option<i64>,
    pub wave_id: Option<i64>,
    pub plan_id: Option<i64>,
    pub status: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub id: i64,
    pub queue_id: i64,
    pub verdict: String,
    pub report: Option<String>,
    pub validator: Option<String>,
    pub created_at: String,
}

// ── Queue operations ─────────────────────────────────────────────────────────

/// Enqueue a validation request. Returns the new queue row id.
/// Idempotent: if task_id already has a pending/running entry, returns that id.
pub fn enqueue_validation(
    conn: &Connection,
    task_id: Option<i64>,
    wave_id: Option<i64>,
    plan_id: Option<i64>,
) -> Result<i64> {
    if let Some(tid) = task_id {
        if let Ok(existing) = conn.query_row(
            "SELECT id FROM validation_queue
             WHERE task_id=?1 AND status IN ('pending','running')
             LIMIT 1",
            params![tid],
            |r| r.get::<_, i64>(0),
        ) {
            return Ok(existing);
        }
    }
    conn.execute(
        "INSERT INTO validation_queue (task_id, wave_id, plan_id)
         VALUES (?1, ?2, ?3)",
        params![task_id, wave_id, plan_id],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Return all pending queue entries.
pub fn get_pending(conn: &Connection) -> Result<Vec<QueueEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, wave_id, plan_id, status, created_at, started_at, completed_at
         FROM validation_queue WHERE status='pending' ORDER BY id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(QueueEntry {
            id: r.get(0)?,
            task_id: r.get(1)?,
            wave_id: r.get(2)?,
            plan_id: r.get(3)?,
            status: r.get(4)?,
            created_at: r.get(5)?,
            started_at: r.get(6)?,
            completed_at: r.get(7)?,
        })
    })?;
    rows.collect()
}

/// Return all queue entries (any status) ordered by id desc.
pub fn list_queue(conn: &Connection) -> Result<Vec<QueueEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, wave_id, plan_id, status, created_at, started_at, completed_at
         FROM validation_queue ORDER BY id DESC LIMIT 200",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(QueueEntry {
            id: r.get(0)?,
            task_id: r.get(1)?,
            wave_id: r.get(2)?,
            plan_id: r.get(3)?,
            status: r.get(4)?,
            created_at: r.get(5)?,
            started_at: r.get(6)?,
            completed_at: r.get(7)?,
        })
    })?;
    rows.collect()
}

/// Persist a verdict and mark the queue entry as completed.
pub fn record_verdict(
    conn: &Connection,
    queue_id: i64,
    verdict: &str,
    report: Option<&str>,
    validator: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT INTO validation_verdicts (queue_id, verdict, report, validator)
         VALUES (?1, ?2, ?3, ?4)",
        params![queue_id, verdict, report, validator],
    )?;
    conn.execute(
        "UPDATE validation_queue
         SET status='completed', completed_at=datetime('now')
         WHERE id=?1",
        params![queue_id],
    )?;
    Ok(())
}

/// Idempotent verdict lookup — returns existing verdict if task was already validated.
pub fn get_verdict(conn: &Connection, task_id: i64) -> Result<Option<Verdict>> {
    let result = conn.query_row(
        "SELECT v.id, v.queue_id, v.verdict, v.report, v.validator, v.created_at
         FROM validation_verdicts v
         JOIN validation_queue q ON v.queue_id = q.id
         WHERE q.task_id = ?1
         ORDER BY v.id DESC
         LIMIT 1",
        params![task_id],
        |r| {
            Ok(Verdict {
                id: r.get(0)?,
                queue_id: r.get(1)?,
                verdict: r.get(2)?,
                report: r.get(3)?,
                validator: r.get(4)?,
                created_at: r.get(5)?,
            })
        },
    );
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Mark stale pending/running entries as failed with a 'timeout' verdict.
/// `max_age_secs`: entries older than this are considered stale.
pub fn timeout_stale(conn: &Connection, max_age_secs: u64) -> Result<usize> {
    // Collect stale queue ids before updating so we can insert verdicts.
    let mut stmt = conn.prepare(
        "SELECT id FROM validation_queue
         WHERE status IN ('pending','running')
         AND created_at < datetime('now', ?1)",
    )?;
    let interval = format!("-{max_age_secs} seconds");
    let ids: Vec<i64> = stmt
        .query_map(params![interval], |r| r.get(0))?
        .collect::<Result<Vec<_>>>()?;

    let count = ids.len();
    for id in &ids {
        conn.execute(
            "UPDATE validation_queue
             SET status='failed', completed_at=datetime('now')
             WHERE id=?1",
            params![id],
        )?;
        conn.execute(
            "INSERT INTO validation_verdicts (queue_id, verdict, report, validator)
             VALUES (?1, 'timeout', 'Timed out waiting for validator', 'system')",
            params![id],
        )?;
    }
    Ok(count)
}

#[cfg(test)]
#[path = "validator_service_tests.rs"]
mod tests;
