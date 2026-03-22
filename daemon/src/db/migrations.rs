/// Schema migrations for execution_runs and supporting infrastructure.
///
/// Idempotent: checks sqlite_master before creating tables/indexes.
/// Safe to call on every startup — skips if already applied.
use rusqlite::Connection;
use std::path::PathBuf;

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
        let name = sql
            .split_whitespace()
            .nth(2)
            .unwrap_or("");
        if !index_exists(conn, name)? {
            conn.execute_batch(sql)?;
            eprintln!("[migrations] created index {name}");
        }
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
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn in_memory() -> Connection {
        Connection::open_in_memory().expect("in-memory db")
    }

    #[test]
    fn run_is_idempotent() {
        let conn = in_memory();
        run(&conn).expect("first run");
        run(&conn).expect("second run — must be idempotent");
    }

    #[test]
    fn execution_runs_schema_is_correct() {
        let conn = in_memory();
        run(&conn).expect("migration");

        // Table must exist
        assert!(table_exists(&conn, "execution_runs").unwrap());

        // Insert a minimal row and verify it round-trips
        conn.execute(
            "INSERT INTO execution_runs (goal) VALUES (?1)",
            ["verify schema"],
        )
        .expect("insert");

        let status: String = conn
            .query_row(
                "SELECT status FROM execution_runs WHERE goal='verify schema'",
                [],
                |r| r.get(0),
            )
            .expect("select");
        assert_eq!(status, "running");
    }

    #[test]
    fn execution_runs_status_constraint_rejects_invalid() {
        let conn = in_memory();
        run(&conn).expect("migration");

        let result = conn.execute(
            "INSERT INTO execution_runs (goal, status) VALUES (?1, ?2)",
            ["test goal", "invalid_status"],
        );
        assert!(result.is_err(), "CHECK constraint must reject invalid status");
    }

    #[test]
    fn indexes_exist_after_migration() {
        let conn = in_memory();
        run(&conn).expect("migration");

        for name in &[
            "idx_execution_runs_status",
            "idx_execution_runs_plan_id",
            "idx_execution_runs_started_at",
        ] {
            assert!(
                index_exists(&conn, name).unwrap(),
                "index {name} must exist"
            );
        }
    }
}
