// DB migration and schema init extracted from state.rs (250-line split).
// DDL including CREATE TABLE and CREATE INDEX lives in state_init_migrations.rs.
use super::state::ApiError;
use super::state_init_canon::canonicalize_existing_project_paths;
use super::state_init_migrations::MIGRATIONS;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::PathBuf;

const AGENT_ACTIVITY_SCHEMA: &str =
    "CREATE TABLE IF NOT EXISTS agent_activity (id INTEGER PRIMARY KEY AUTOINCREMENT, agent_id TEXT NOT NULL, task_db_id INTEGER, plan_id INTEGER, agent_type TEXT NOT NULL, model TEXT, description TEXT, status TEXT NOT NULL DEFAULT 'running', tokens_in INTEGER DEFAULT 0, tokens_out INTEGER DEFAULT 0, tokens_total INTEGER DEFAULT 0, cost_usd REAL DEFAULT 0, started_at TEXT NOT NULL DEFAULT (datetime('now')), completed_at TEXT, duration_s REAL, host TEXT, region TEXT, metadata TEXT, parent_session TEXT)";

const AGENT_ACTIVITY_COLUMNS: &[(&str, &str)] = &[
    ("agent_type", "TEXT NOT NULL DEFAULT 'legacy'"),
    ("model", "TEXT"),
    ("description", "TEXT"),
    ("status", "TEXT NOT NULL DEFAULT 'completed'"),
    ("tokens_in", "INTEGER DEFAULT 0"),
    ("tokens_out", "INTEGER DEFAULT 0"),
    ("tokens_total", "INTEGER DEFAULT 0"),
    ("cost_usd", "REAL DEFAULT 0"),
    ("started_at", "TEXT"),
    ("completed_at", "TEXT"),
    ("duration_s", "REAL"),
    ("host", "TEXT"),
    ("region", "TEXT"),
    ("metadata", "TEXT"),
    ("parent_session", "TEXT"),
];

fn table_columns(conn: &Connection, table: &str) -> Result<HashSet<String>, ApiError> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|err| ApiError::internal(format!("table info prepare failed: {err}")))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| ApiError::internal(format!("table info query failed: {err}")))?;
    let columns = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|err| ApiError::internal(format!("table info decode failed: {err}")))?;
    Ok(columns.into_iter().collect())
}

pub fn ensure_agent_activity_schema(conn: &Connection) -> Result<(), ApiError> {
    conn.execute_batch(AGENT_ACTIVITY_SCHEMA)
        .map_err(|err| ApiError::internal(format!("agent_activity create failed: {err}")))?;

    let mut columns = table_columns(conn, "agent_activity")?;
    for (name, spec) in AGENT_ACTIVITY_COLUMNS {
        if columns.contains(*name) {
            continue;
        }
        conn.execute_batch(&format!(
            "ALTER TABLE agent_activity ADD COLUMN {name} {spec}"
        ))
        .map_err(|err| ApiError::internal(format!("agent_activity alter failed: {err}")))?;
        columns.insert((*name).to_string());
    }

    if columns.contains("action") {
        conn.execute_batch("UPDATE agent_activity SET agent_type = COALESCE(NULLIF(action,''), NULLIF(agent_type,''), 'legacy') WHERE action IS NOT NULL AND action != ''")
            .map_err(|err| ApiError::internal(format!("agent_activity type backfill failed: {err}")))?;
    }
    if columns.contains("details") {
        conn.execute_batch("UPDATE agent_activity SET description = COALESCE(NULLIF(description,''), NULLIF(details,'')) WHERE (description IS NULL OR description = '') AND details IS NOT NULL")
            .map_err(|err| ApiError::internal(format!("agent_activity description backfill failed: {err}")))?;
    }
    if columns.contains("created_at") {
        conn.execute_batch("UPDATE agent_activity SET started_at = COALESCE(NULLIF(started_at,''), created_at, datetime('now')) WHERE started_at IS NULL OR started_at = ''")
            .map_err(|err| ApiError::internal(format!("agent_activity started_at backfill failed: {err}")))?;
    }
    conn.execute_batch(
        "UPDATE agent_activity SET agent_type = COALESCE(NULLIF(agent_type,''), 'legacy') WHERE COALESCE(agent_type,'') = '';
         UPDATE agent_activity SET model = COALESCE(NULLIF(model,''), agent_type, 'unknown') WHERE COALESCE(model,'') = '';
         UPDATE agent_activity SET status = COALESCE(NULLIF(status,''), 'completed') WHERE COALESCE(status,'') = '';
         UPDATE agent_activity SET region = COALESCE(NULLIF(region,''), 'prefrontal') WHERE COALESCE(region,'') = '';
         UPDATE agent_activity SET started_at = COALESCE(NULLIF(started_at,''), datetime('now')) WHERE started_at IS NULL OR started_at = '';
         DELETE FROM agent_activity WHERE id NOT IN (SELECT MAX(id) FROM agent_activity GROUP BY agent_id);
         CREATE UNIQUE INDEX IF NOT EXISTS uq_agent_activity_agent_id ON agent_activity(agent_id);",
    ).map_err(|err| ApiError::internal(format!("agent_activity repair failed: {err}")))?;

    Ok(())
}

/// Run DB migrations and return a connection pool for `db_path`.
/// Called once during ServerState::new.
pub fn init_db_and_pool(
    db_path: &PathBuf,
    crsqlite_path: &Option<String>,
) -> Pool<SqliteConnectionManager> {
    if let Ok(conn) = Connection::open(db_path) {
        if let Err(e) = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;") {
            eprintln!("[migration] PRAGMA init failed: {e}");
        }
        #[cfg(feature = "crsqlite")]
        if let Some(ref ext) = crsqlite_path {
            if let Err(e) = crate::db::crdt::load_crsqlite(&conn, ext) {
                eprintln!("[migration] crsqlite load failed: {e}");
            }
        }
        #[cfg(not(feature = "crsqlite"))]
        let _ = &crsqlite_path; // suppress unused warning
        if let Err(err) = ensure_agent_activity_schema(&conn) {
            eprintln!("[migration] agent_activity schema repair failed: {err:?}");
        }
        if let Err(err) = crate::db::migrations::run(&conn) {
            eprintln!("[migration] execution_runs migration failed: {err:?}");
        }
        canonicalize_existing_project_paths(&conn);
        let mut ok = 0;
        let mut skip = 0;
        for sql in MIGRATIONS {
            match conn.execute_batch(sql) {
                Ok(_) => ok += 1,
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("duplicate column") || msg.contains("already exists") {
                        skip += 1;
                    } else {
                        eprintln!(
                            "[migration] ERROR on '{}'...: {e}",
                            &sql.chars().take(50).collect::<String>()
                        );
                    }
                }
            }
        }
        let check = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='agent_activity'");
        let exists = check
            .map(|mut s| s.exists([]))
            .unwrap_or(Ok(false))
            .unwrap_or(false);
        if !exists {
            eprintln!("[migration] CRITICAL: agent_activity table missing after migration!");
        }
        eprintln!(
            "[migration] {ok} applied, {skip} skipped (already exist), agent_activity={exists}"
        );
    }
    let manager = SqliteConnectionManager::file(db_path).with_init(|conn| {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA busy_timeout=5000;
             PRAGMA cache_size=-8000;
             PRAGMA mmap_size=67108864;
             PRAGMA temp_store=MEMORY;",
        )?;
        Ok(())
    });
    Pool::builder()
        .max_size(8)
        .min_idle(Some(2))
        .build(manager)
        .expect("failed to create sqlite connection pool")
}

/// Convenience alias so callers can use the pool type without importing r2d2 directly.
pub type ConnPool = Pool<SqliteConnectionManager>;
pub type PooledConn = PooledConnection<SqliteConnectionManager>;
