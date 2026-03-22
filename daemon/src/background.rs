/// Background pause bridge — polls coordinator_events for pause/resume signals.
///
/// Spawned once at daemon startup. Opens its own SQLite connection so it does not
/// contend with the request path. Runs forever; cancelled implicitly when the
/// process exits.
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{debug, error, warn};

/// Poll interval for coordinator_events.
const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Maximum events fetched per tick to bound memory usage.
const EVENT_LIMIT: i64 = 50;

/// Entry point — call with `tokio::spawn(run_pause_bridge(db_path))`.
///
/// Never returns under normal operation; logs errors and continues on
/// transient DB failures so a single bad tick does not kill the bridge.
pub async fn run_pause_bridge(db_path: PathBuf) {
    let conn = match open_conn(&db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("pause_bridge: cannot open DB {db_path:?}: {e}");
            return;
        }
    };

    let mut seen: HashSet<i64> = HashSet::new();
    let mut ticker = tokio::time::interval(POLL_INTERVAL);
    // Skip the immediate first tick so we do not fire at t=0 before the
    // server has finished binding its sockets.
    ticker.tick().await;

    loop {
        ticker.tick().await;
        if let Err(e) = process_tick(&conn, &mut seen) {
            warn!("pause_bridge tick error: {e}");
        }
    }
}

/// One poll cycle: fetch new pause/resume events and apply DB updates.
pub(crate) fn process_tick(conn: &Connection, seen: &mut HashSet<i64>) -> rusqlite::Result<()> {
    // coordinator_events may not exist yet (e.g. fresh DB before migrations run).
    // Return Ok(()) silently — the bridge will retry on the next tick.
    if !table_exists(conn, "coordinator_events")? {
        debug!("pause_bridge: coordinator_events table not yet present, skipping tick");
        return Ok(());
    }

    let mut stmt = conn.prepare_cached(
        "SELECT id, event_type, payload \
         FROM coordinator_events \
         WHERE event_type IN ('pause_run', 'resume_run') \
         ORDER BY id DESC \
         LIMIT ?1",
    )?;

    let rows: Vec<(i64, String, Option<String>)> = stmt
        .query_map([EVENT_LIMIT], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    for (id, event_type, payload) in rows {
        if seen.contains(&id) {
            continue;
        }
        seen.insert(id);

        let plan_id = match extract_plan_id(payload.as_deref()) {
            Some(pid) => pid,
            None => {
                warn!("pause_bridge: event {id} ({event_type}) missing plan_id in payload");
                continue;
            }
        };

        let result = match event_type.as_str() {
            "pause_run" => apply_pause(conn, plan_id),
            "resume_run" => apply_resume(conn, plan_id),
            _ => Ok(0),
        };

        match result {
            Ok(n) => debug!("pause_bridge: event {id} ({event_type}) plan_id={plan_id} affected={n}"),
            Err(e) => warn!("pause_bridge: event {id} ({event_type}) plan_id={plan_id} error: {e}"),
        }
    }

    Ok(())
}

pub(crate) fn apply_pause(conn: &Connection, plan_id: i64) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE execution_runs \
         SET status='paused', paused_at=datetime('now') \
         WHERE plan_id=?1 AND status='running'",
        [plan_id],
    )
}

pub(crate) fn apply_resume(conn: &Connection, plan_id: i64) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE execution_runs \
         SET status='running', paused_at=NULL \
         WHERE plan_id=?1 AND status='paused'",
        [plan_id],
    )
}

/// Extract `plan_id` from a JSON payload string like `{"plan_id": 42}`.
pub(crate) fn extract_plan_id(payload: Option<&str>) -> Option<i64> {
    let text = payload?;
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    v.get("plan_id")?.as_i64()
}

fn open_conn(path: &PathBuf) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    // WAL mode + busy timeout — shared DB with daemon and server.
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
    Ok(conn)
}

fn table_exists(conn: &Connection, name: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        [name],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n > 0)
}

#[cfg(test)]
#[path = "background_tests.rs"]
mod background_tests;
