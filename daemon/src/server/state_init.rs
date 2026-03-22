// DB migration and schema init extracted from state.rs (250-line split).
use super::state::ApiError;
use super::state_init_canon::canonicalize_existing_project_paths;
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
    let mut stmt = conn.prepare(&sql)
        .map_err(|err| ApiError::internal(format!("table info prepare failed: {err}")))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))
        .map_err(|err| ApiError::internal(format!("table info query failed: {err}")))?;
    let columns = rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|err| ApiError::internal(format!("table info decode failed: {err}")))?;
    Ok(columns.into_iter().collect())
}

pub fn ensure_agent_activity_schema(conn: &Connection) -> Result<(), ApiError> {
    conn.execute_batch(AGENT_ACTIVITY_SCHEMA)
        .map_err(|err| ApiError::internal(format!("agent_activity create failed: {err}")))?;

    let mut columns = table_columns(conn, "agent_activity")?;
    for (name, spec) in AGENT_ACTIVITY_COLUMNS {
        if columns.contains(*name) { continue; }
        conn.execute_batch(&format!("ALTER TABLE agent_activity ADD COLUMN {name} {spec}"))
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

const MIGRATIONS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS daemon_config (key TEXT PRIMARY KEY NOT NULL, value TEXT, updated_at TEXT DEFAULT (datetime('now')))",
    "CREATE TABLE IF NOT EXISTS coordinator_events (id INTEGER PRIMARY KEY, event_type TEXT NOT NULL DEFAULT '', payload TEXT, source_node TEXT, handled_at TEXT DEFAULT (datetime('now')))",
    "CREATE TABLE IF NOT EXISTS notification_queue (id INTEGER PRIMARY KEY, severity TEXT DEFAULT 'info', title TEXT NOT NULL DEFAULT '', message TEXT, plan_id INTEGER, link TEXT, status TEXT DEFAULT 'pending', created_at TEXT DEFAULT (datetime('now')), delivered_at TEXT)",
    "CREATE INDEX IF NOT EXISTS idx_notification_queue_status ON notification_queue(status)",
    "CREATE INDEX IF NOT EXISTS idx_coordinator_events_type ON coordinator_events(event_type)",
    "CREATE TABLE IF NOT EXISTS agent_runs (id INTEGER PRIMARY KEY AUTOINCREMENT, plan_id INTEGER, wave_id TEXT, task_id TEXT, agent_name TEXT, agent_role TEXT, model TEXT, peer_name TEXT, status TEXT DEFAULT 'running', started_at TEXT DEFAULT (datetime('now')), last_heartbeat TEXT, current_task TEXT)",
    "CREATE TABLE IF NOT EXISTS nightly_jobs (id INTEGER PRIMARY KEY AUTOINCREMENT, run_id TEXT, job_name TEXT DEFAULT 'guardian', started_at DATETIME DEFAULT CURRENT_TIMESTAMP, finished_at DATETIME, host TEXT, status TEXT NOT NULL CHECK(status IN ('running','ok','action_required','failed')), sentry_unresolved INTEGER DEFAULT 0, github_open_issues INTEGER DEFAULT 0, processed_items INTEGER DEFAULT 0, fixed_items INTEGER DEFAULT 0, branch_name TEXT, pr_url TEXT, summary TEXT, report_json TEXT)",
    "CREATE TABLE IF NOT EXISTS nightly_job_definitions (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, description TEXT, schedule TEXT NOT NULL DEFAULT '0 3 * * *', script_path TEXT NOT NULL, target_host TEXT DEFAULT 'local', enabled INTEGER NOT NULL DEFAULT 1, created_at DATETIME DEFAULT CURRENT_TIMESTAMP)",
    "CREATE TABLE IF NOT EXISTS github_events (id INTEGER PRIMARY KEY AUTOINCREMENT, plan_id INTEGER, event_type TEXT, status TEXT DEFAULT 'pending', created_at TEXT DEFAULT (datetime('now')))",
    "CREATE TABLE IF NOT EXISTS earned_skills (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL UNIQUE, domain TEXT, content TEXT NOT NULL, confidence TEXT DEFAULT 'low', hit_count INTEGER DEFAULT 0, source TEXT DEFAULT 'earned', created_at TEXT DEFAULT (datetime('now')), updated_at TEXT DEFAULT (datetime('now')))",
    "CREATE TABLE IF NOT EXISTS plan_commits (id INTEGER PRIMARY KEY AUTOINCREMENT, plan_id INTEGER, commit_sha TEXT, commit_message TEXT, lines_added INTEGER DEFAULT 0, lines_removed INTEGER DEFAULT 0, files_changed INTEGER DEFAULT 0, authored_at TEXT, created_at TEXT DEFAULT (datetime('now')))",
    "CREATE INDEX IF NOT EXISTS idx_agent_activity_status ON agent_activity(status)",
    "CREATE INDEX IF NOT EXISTS idx_agent_activity_plan ON agent_activity(plan_id)",
    "CREATE INDEX IF NOT EXISTS idx_agent_activity_task ON agent_activity(task_db_id)",
    "CREATE INDEX IF NOT EXISTS idx_agent_activity_started_at ON agent_activity(started_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_agent_activity_status_started ON agent_activity(status, started_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_agent_activity_status_completed ON agent_activity(status, completed_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_agent_activity_model ON agent_activity(model)",
    "CREATE INDEX IF NOT EXISTS idx_agent_runs_started_at ON agent_runs(started_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_agent_runs_status ON agent_runs(status)",
    "CREATE INDEX IF NOT EXISTS idx_agent_runs_peer ON agent_runs(peer_name)",
    "CREATE INDEX IF NOT EXISTS idx_nightly_jobs_started ON nightly_jobs(started_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_mesh_events_created_at ON mesh_events(created_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_mesh_events_status ON mesh_events(status)",
    "CREATE INDEX IF NOT EXISTS idx_token_usage_model ON token_usage(model)",
    "CREATE INDEX IF NOT EXISTS idx_token_usage_created_at ON token_usage(created_at)",
    "CREATE INDEX IF NOT EXISTS idx_github_events_plan_status ON github_events(plan_id, status)",
    "CREATE INDEX IF NOT EXISTS idx_plan_commits_plan_id ON plan_commits(plan_id)",
    "CREATE INDEX IF NOT EXISTS idx_projects_name ON projects(name COLLATE NOCASE)",
    "CREATE TABLE IF NOT EXISTS ideas (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, description TEXT, tags TEXT, priority TEXT DEFAULT 'P2' CHECK(priority IN ('P0','P1','P2','P3')), status TEXT DEFAULT 'draft' CHECK(status IN ('draft','elaborating','ready','promoted','archived')), project_id TEXT REFERENCES projects(id) ON DELETE SET NULL, links TEXT, plan_id INTEGER, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME DEFAULT CURRENT_TIMESTAMP)",
    "CREATE TABLE IF NOT EXISTS idea_notes (id INTEGER PRIMARY KEY AUTOINCREMENT, idea_id INTEGER NOT NULL REFERENCES ideas(id) ON DELETE CASCADE, content TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP)",
    "CREATE INDEX IF NOT EXISTS idx_ideas_status ON ideas(status)",
    "CREATE INDEX IF NOT EXISTS idx_ideas_project ON ideas(project_id)",
    "CREATE INDEX IF NOT EXISTS idx_idea_notes_idea ON idea_notes(idea_id)",
    "ALTER TABLE nightly_jobs ADD COLUMN job_name TEXT DEFAULT 'guardian'",
    "ALTER TABLE nightly_jobs ADD COLUMN log_stdout TEXT",
    "ALTER TABLE nightly_jobs ADD COLUMN log_stderr TEXT",
    "ALTER TABLE nightly_jobs ADD COLUMN log_file_path TEXT",
    "ALTER TABLE nightly_jobs ADD COLUMN duration_sec INTEGER",
    "ALTER TABLE nightly_jobs ADD COLUMN config_snapshot TEXT",
    "ALTER TABLE nightly_jobs ADD COLUMN exit_code INTEGER",
    "ALTER TABLE nightly_jobs ADD COLUMN error_detail TEXT",
    "ALTER TABLE nightly_jobs ADD COLUMN trigger_source TEXT DEFAULT 'scheduled'",
    "ALTER TABLE nightly_jobs ADD COLUMN parent_run_id TEXT",
    "ALTER TABLE nightly_job_definitions ADD COLUMN project_id TEXT DEFAULT 'mirrorbuddy'",
    "ALTER TABLE nightly_job_definitions ADD COLUMN run_fixes INTEGER DEFAULT 1",
    "ALTER TABLE nightly_job_definitions ADD COLUMN timeout_sec INTEGER DEFAULT 5400",
    "ALTER TABLE agent_activity ADD COLUMN parent_session TEXT",
    "ALTER TABLE plans ADD COLUMN waves_total INTEGER DEFAULT 0",
    "ALTER TABLE plans ADD COLUMN waves_merged INTEGER DEFAULT 0",
    "CREATE TABLE IF NOT EXISTS plan_reviews (id INTEGER PRIMARY KEY AUTOINCREMENT, plan_id INTEGER NOT NULL, reviewer_agent TEXT NOT NULL, verdict TEXT NOT NULL, suggestions TEXT, raw_report TEXT, reviewed_at TEXT NOT NULL DEFAULT (datetime('now')))",
    "CREATE TABLE IF NOT EXISTS agent_catalog (name TEXT PRIMARY KEY, category TEXT, description TEXT, model TEXT, tools TEXT, skills TEXT, source_repo TEXT, constitution_version TEXT, version TEXT, created_at DATETIME DEFAULT (datetime('now')), updated_at DATETIME DEFAULT (datetime('now')))",
    // Plan 689 — project metadata columns
    "ALTER TABLE projects ADD COLUMN input_path TEXT DEFAULT NULL",
    "ALTER TABLE projects ADD COLUMN output_path TEXT DEFAULT NULL",
    "ALTER TABLE projects ADD COLUMN github_url TEXT DEFAULT NULL",
    "ALTER TABLE projects ADD COLUMN icon_path TEXT DEFAULT NULL",
    // Plan 689 — deliverables table
    "CREATE TABLE IF NOT EXISTS deliverables (id INTEGER PRIMARY KEY, task_id INTEGER REFERENCES tasks(id), project_id TEXT NOT NULL, name TEXT NOT NULL, output_path TEXT, version INTEGER DEFAULT 1, status TEXT DEFAULT 'pending' CHECK(status IN ('pending','in_progress','ready','approved','rejected')), output_type TEXT NOT NULL, metadata_json TEXT DEFAULT '{}', created_at DATETIME DEFAULT CURRENT_TIMESTAMP, approved_at DATETIME DEFAULT NULL, approved_by TEXT DEFAULT NULL, updated_at DATETIME DEFAULT NULL)",
    "CREATE INDEX IF NOT EXISTS idx_deliverables_project ON deliverables(project_id)",
    "CREATE INDEX IF NOT EXISTS idx_deliverables_task ON deliverables(task_id)",
    // Plan 689 — solve_sessions table
    "CREATE TABLE IF NOT EXISTS solve_sessions (id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp TEXT NOT NULL DEFAULT (datetime('now')), user_input TEXT NOT NULL, constitution_check TEXT, triage_level TEXT CHECK(triage_level IN ('light','standard','full')), clarification_rounds TEXT, research_findings TEXT, problem_statement TEXT, requirements_json TEXT, acceptance_invariants TEXT, routed_to TEXT, decision_audit TEXT, plan_id INTEGER REFERENCES plans(id), project_id TEXT DEFAULT NULL)",
    "CREATE INDEX IF NOT EXISTS idx_solve_sessions_project ON solve_sessions(project_id)",
];

/// Run DB migrations and return a connection pool for `db_path`.
/// Called once during ServerState::new.
pub fn init_db_and_pool(
    db_path: &PathBuf,
    crsqlite_path: &Option<String>,
) -> Pool<SqliteConnectionManager> {
    if let Ok(conn) = Connection::open(db_path) {
        let _ = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;");
        if let Some(ref ext) = crsqlite_path {
            if let Err(e) = crate::db::crdt::load_crsqlite(&conn, ext) {
                eprintln!("[migration] crsqlite load failed: {e}");
            }
        }
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
                        eprintln!("[migration] ERROR on '{}'...: {e}", &sql.chars().take(50).collect::<String>());
                    }
                }
            }
        }
        let check = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='agent_activity'",
        );
        let exists = check.map(|mut s| s.exists([])).unwrap_or(Ok(false)).unwrap_or(false);
        if !exists {
            eprintln!("[migration] CRITICAL: agent_activity table missing after migration!");
        }
        eprintln!("[migration] {ok} applied, {skip} skipped (already exist), agent_activity={exists}");
    }
    let manager = SqliteConnectionManager::file(db_path).with_init(|conn| {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
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
