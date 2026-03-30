/// Schema migrations for execution_runs and supporting infrastructure.
///
/// Idempotent: checks sqlite_master before creating tables/indexes.
/// Safe to call on every startup — skips if already applied.
use rusqlite::Connection;
use std::path::PathBuf;

const CREATE_DOMAIN_SKILL_MAP: &str = "
CREATE TABLE domain_skill_map (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    domain      TEXT    NOT NULL,
    skill_name  TEXT    NOT NULL,
    description TEXT,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(domain, skill_name)
)";

const DOMAIN_SKILL_SEED: &[(&str, &str, &str)] = &[
    (
        "healthcare",
        "research",
        "Medical research and clinical analysis",
    ),
    ("deploy", "release", "Deployment and release management"),
    ("design", "prepare", "Design preparation and setup"),
];

const CREATE_EXECUTION_RUNS: &str = "
CREATE TABLE execution_runs (
    id              INTEGER PRIMARY KEY,
    goal            TEXT    NOT NULL,
    team            TEXT    NOT NULL DEFAULT '[]',
    status          TEXT    NOT NULL DEFAULT 'running'
        CHECK(status IN ('running','completed','failed','cancelled','paused')),
    result          TEXT,
    cost_usd        REAL    NOT NULL DEFAULT 0,
    agents_used     INTEGER NOT NULL DEFAULT 0,
    plan_id         INTEGER,
    started_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    completed_at    TEXT,
    duration_minutes REAL,
    context_path    TEXT,
    paused_at       TEXT,
    paused_context  TEXT
)";

const INDEXES: &[&str] = &[
    "CREATE INDEX idx_execution_runs_status   ON execution_runs(status)",
    "CREATE INDEX idx_execution_runs_plan_id  ON execution_runs(plan_id)",
    "CREATE INDEX idx_execution_runs_started_at ON execution_runs(started_at DESC)",
];

/// Run all startup migrations against `conn`.
///
/// Called once per daemon launch after the connection is established.
/// Each step is guarded by a sqlite_master check or `IF NOT EXISTS`,
/// so repeated calls are safe.
pub fn run(conn: &Connection) -> rusqlite::Result<()> {
    ensure_execution_runs(conn)?;
    ensure_domain_skill_map(conn)?;
    ensure_mesh_sync_stats_columns(conn)?;
    ensure_sync_meta(conn)?;
    ensure_sync_conflicts(conn)?;
    ensure_runs_dir();
    Ok(())
}

fn table_exists(conn: &Connection, name: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        [name],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n > 0)
}

fn index_exists(conn: &Connection, name: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
        [name],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n > 0)
}

fn ensure_execution_runs(conn: &Connection) -> rusqlite::Result<()> {
    if !table_exists(conn, "execution_runs")? {
        conn.execute_batch(CREATE_EXECUTION_RUNS)?;
        eprintln!("[migrations] created execution_runs table");
    }

    // Apply indexes regardless — each is guarded by its own existence check.
    for sql in INDEXES {
        // Extract index name (second token after CREATE INDEX).
        let name = sql.split_whitespace().nth(2).unwrap_or("");
        if !index_exists(conn, name)? {
            conn.execute_batch(sql)?;
            eprintln!("[migrations] created index {name}");
        }
    }

    Ok(())
}

fn ensure_domain_skill_map(conn: &Connection) -> rusqlite::Result<()> {
    if !table_exists(conn, "domain_skill_map")? {
        conn.execute_batch(CREATE_DOMAIN_SKILL_MAP)?;
        eprintln!("[migrations] created domain_skill_map table");

        for (domain, skill_name, description) in DOMAIN_SKILL_SEED {
            conn.execute(
                "INSERT INTO domain_skill_map (domain, skill_name, description) \
                 VALUES (?1, ?2, ?3)",
                [domain, skill_name, description],
            )?;
        }
        eprintln!(
            "[migrations] seeded domain_skill_map ({} rows)",
            DOMAIN_SKILL_SEED.len()
        );
    }
    Ok(())
}

/// Add peer-health columns to mesh_sync_stats if the table exists but lacks them.
///
/// Idempotent: uses PRAGMA table_info to check before issuing ALTER TABLE.
/// Called every startup so fresh nodes that already have the columns skip cleanly.
fn ensure_mesh_sync_stats_columns(conn: &Connection) -> rusqlite::Result<()> {
    if !table_exists(conn, "mesh_sync_stats")? {
        return Ok(());
    }

    let columns: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(mesh_sync_stats)")?;
        let names: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(|r| match r { Ok(v) => Some(v), Err(e) => { tracing::warn!("skipping column name row: {e}"); None } })
            .collect();
        names
    };

    if !columns.iter().any(|c| c == "consecutive_failures") {
        conn.execute_batch(
            "ALTER TABLE mesh_sync_stats ADD COLUMN consecutive_failures INTEGER DEFAULT 0",
        )?;
        eprintln!("[migrations] added mesh_sync_stats.consecutive_failures");
    }

    if !columns.iter().any(|c| c == "status") {
        conn.execute_batch("ALTER TABLE mesh_sync_stats ADD COLUMN status TEXT DEFAULT 'online'")?;
        eprintln!("[migrations] added mesh_sync_stats.status");
    }

    Ok(())
}

/// Create the `_sync_meta` table used by the timestamp-based sync adapter.
///
/// Tracks the last sync timestamp per peer+table pair so background_sync
/// only transfers rows modified since the last successful sync.
/// Idempotent: skips if the table already exists.
fn ensure_sync_meta(conn: &Connection) -> rusqlite::Result<()> {
    if !table_exists(conn, "_sync_meta")? {
        conn.execute_batch(
            "CREATE TABLE _sync_meta (
                peer       TEXT NOT NULL,
                table_name TEXT NOT NULL,
                last_sync_at TEXT NOT NULL,
                PRIMARY KEY (peer, table_name)
            )",
        )?;
        eprintln!("[migrations] created _sync_meta table");
    }
    Ok(())
}

/// Create the `_sync_conflicts` table for CRDT conflict visibility.
///
/// Records conflicts detected during CRDT merge so operators can inspect
/// and resolve them via GET /api/sync/conflicts. Idempotent.
fn ensure_sync_conflicts(conn: &Connection) -> rusqlite::Result<()> {
    if !table_exists(conn, "_sync_conflicts")? {
        conn.execute_batch(
            "CREATE TABLE _sync_conflicts (
                id INTEGER PRIMARY KEY,
                table_name TEXT NOT NULL,
                pk INTEGER,
                local_data TEXT,
                remote_data TEXT,
                source_node TEXT,
                resolved BOOLEAN DEFAULT 0,
                created_at TEXT DEFAULT (datetime('now'))
            )",
        )?;
        eprintln!("[migrations] created _sync_conflicts table");
    }
    Ok(())
}

/// Ensure data/runs/ exists relative to the executable's project root.
///
/// Uses the `DASHBOARD_DB` env var to locate the project root (parent of
/// the `data/` directory).  Falls back to `$HOME/.claude/data/runs/`.
fn ensure_runs_dir() {
    let runs_dir = runs_dir_path();
    if let Err(e) = std::fs::create_dir_all(&runs_dir) {
        eprintln!("[migrations] warn: could not create {runs_dir:?}: {e}");
    } else {
        eprintln!("[migrations] runs dir ready: {runs_dir:?}");
    }
}

fn runs_dir_path() -> PathBuf {
    // Prefer sibling to the active DB file so runs/ lives next to dashboard.db.
    if let Ok(db_path) = std::env::var("DASHBOARD_DB") {
        if let Some(parent) = PathBuf::from(&db_path).parent() {
            return parent.join("runs");
        }
    }
    // Fallback: ~/.claude/data/runs/
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude/data/runs")
}

#[cfg(test)]
#[path = "migrations_tests.rs"]
mod tests;
