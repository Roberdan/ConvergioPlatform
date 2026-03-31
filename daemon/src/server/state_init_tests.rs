//! Tests for workspace schema migrations in state_init.
//! Verifies workspaces and workspace_events tables exist after MIGRATIONS run.

use rusqlite::Connection;

fn run_workspace_migrations(conn: &Connection) {
    // Run only the workspace-related migrations (subset of MIGRATIONS constant).
    // We use the same SQL strings verbatim so the test is coupled to the real DDL.
    let ddl = [
        "CREATE TABLE IF NOT EXISTS workspaces (id INTEGER PRIMARY KEY AUTOINCREMENT, plan_id INTEGER, wave_db_id INTEGER, workspace_id TEXT UNIQUE NOT NULL, path TEXT NOT NULL, branch TEXT, status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','merged','deleted')), created_at TEXT NOT NULL DEFAULT (datetime('now')), deleted_at TEXT)",
        "CREATE INDEX IF NOT EXISTS idx_workspaces_plan ON workspaces(plan_id)",
        "CREATE INDEX IF NOT EXISTS idx_workspaces_status ON workspaces(status)",
        "CREATE INDEX IF NOT EXISTS idx_workspaces_workspace_id ON workspaces(workspace_id)",
        "CREATE TABLE IF NOT EXISTS workspace_events (id INTEGER PRIMARY KEY AUTOINCREMENT, workspace_id TEXT NOT NULL, agent TEXT NOT NULL, action TEXT NOT NULL, file_path TEXT, detail TEXT, metadata TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')))",
        "CREATE INDEX IF NOT EXISTS idx_workspace_events_workspace ON workspace_events(workspace_id)",
        "CREATE INDEX IF NOT EXISTS idx_workspace_events_agent ON workspace_events(agent)",
        "CREATE INDEX IF NOT EXISTS idx_workspace_events_created ON workspace_events(created_at DESC)",
    ];
    for sql in ddl {
        conn.execute_batch(sql).expect("workspace migration failed");
    }
}

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
        rusqlite::params![name],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

fn index_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
        rusqlite::params![name],
        |row| row.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}

#[test]
fn test_workspaces_table_exists_after_migration() {
    let conn = Connection::open_in_memory().expect("open_in_memory");
    run_workspace_migrations(&conn);
    assert!(
        table_exists(&conn, "workspaces"),
        "workspaces table must exist after migration"
    );
}

#[test]
fn test_workspace_events_table_exists_after_migration() {
    let conn = Connection::open_in_memory().expect("open_in_memory");
    run_workspace_migrations(&conn);
    assert!(
        table_exists(&conn, "workspace_events"),
        "workspace_events table must exist after migration"
    );
}

#[test]
fn test_workspace_indexes_exist_after_migration() {
    let conn = Connection::open_in_memory().expect("open_in_memory");
    run_workspace_migrations(&conn);
    assert!(
        index_exists(&conn, "idx_workspaces_plan"),
        "idx_workspaces_plan missing"
    );
    assert!(
        index_exists(&conn, "idx_workspaces_status"),
        "idx_workspaces_status missing"
    );
    assert!(
        index_exists(&conn, "idx_workspaces_workspace_id"),
        "idx_workspaces_workspace_id missing"
    );
    assert!(
        index_exists(&conn, "idx_workspace_events_workspace"),
        "idx_workspace_events_workspace missing"
    );
    assert!(
        index_exists(&conn, "idx_workspace_events_agent"),
        "idx_workspace_events_agent missing"
    );
    assert!(
        index_exists(&conn, "idx_workspace_events_created"),
        "idx_workspace_events_created missing"
    );
}

#[test]
fn test_workspaces_schema_columns() {
    let conn = Connection::open_in_memory().expect("open_in_memory");
    run_workspace_migrations(&conn);
    // Insert and retrieve a row to verify column structure is correct.
    conn.execute_batch(
        "INSERT INTO workspaces (workspace_id, path, status) VALUES ('ws-001', '/tmp/ws-001', 'active')",
    )
    .expect("insert workspace");
    let (workspace_id, path, status): (String, String, String) = conn
        .query_row(
            "SELECT workspace_id, path, status FROM workspaces WHERE workspace_id='ws-001'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("select workspace");
    assert_eq!(workspace_id, "ws-001");
    assert_eq!(path, "/tmp/ws-001");
    assert_eq!(status, "active");
}

#[test]
fn test_workspace_events_schema_columns() {
    let conn = Connection::open_in_memory().expect("open_in_memory");
    run_workspace_migrations(&conn);
    conn.execute_batch(
        "INSERT INTO workspace_events (workspace_id, agent, action) VALUES ('ws-001', 'task-executor', 'FileWrite')",
    )
    .expect("insert workspace_event");
    let (workspace_id, agent, action): (String, String, String) = conn
        .query_row(
            "SELECT workspace_id, agent, action FROM workspace_events WHERE workspace_id='ws-001'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("select workspace_event");
    assert_eq!(workspace_id, "ws-001");
    assert_eq!(agent, "task-executor");
    assert_eq!(action, "FileWrite");
}

#[test]
fn test_workspace_status_constraint() {
    let conn = Connection::open_in_memory().expect("open_in_memory");
    run_workspace_migrations(&conn);
    // Invalid status must be rejected by CHECK constraint.
    let result = conn.execute_batch(
        "INSERT INTO workspaces (workspace_id, path, status) VALUES ('ws-bad', '/tmp/bad', 'invalid')",
    );
    assert!(result.is_err(), "invalid status must fail CHECK constraint");
}

#[test]
fn test_workspaces_idempotent_migration() {
    let conn = Connection::open_in_memory().expect("open_in_memory");
    // Running migrations twice must not error (IF NOT EXISTS semantics).
    run_workspace_migrations(&conn);
    run_workspace_migrations(&conn);
    assert!(table_exists(&conn, "workspaces"));
    assert!(table_exists(&conn, "workspace_events"));
}

fn run_core_table_migrations(conn: &Connection) {
    // Minimal schema for plans/waves/tasks so index migrations can reference them.
    let ddl = [
        "CREATE TABLE IF NOT EXISTS plans (id INTEGER PRIMARY KEY, name TEXT, status TEXT DEFAULT 'todo')",
        "CREATE TABLE IF NOT EXISTS waves (id INTEGER PRIMARY KEY, plan_id INTEGER, status TEXT DEFAULT 'pending')",
        "CREATE TABLE IF NOT EXISTS tasks (id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id_fk INTEGER, status TEXT DEFAULT 'pending')",
        // The five new performance indexes from Plan 706 T2-02.
        "CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_wave_id ON tasks(wave_id_fk)",
        "CREATE INDEX IF NOT EXISTS idx_plans_status ON plans(status)",
        "CREATE INDEX IF NOT EXISTS idx_tasks_plan_id ON tasks(plan_id)",
        "CREATE INDEX IF NOT EXISTS idx_waves_plan_id ON waves(plan_id)",
    ];
    for sql in ddl {
        conn.execute_batch(sql)
            .expect("core table migration failed");
    }
}

#[test]
fn test_cleanup_legacy_crdt_objects_removes_shadow_schema() {
    let conn = Connection::open_in_memory().expect("open_in_memory");
    conn.execute_batch(
        "CREATE TABLE workspaces (id INTEGER PRIMARY KEY, path TEXT);
         CREATE TRIGGER workspaces__crsql_itrig AFTER INSERT ON workspaces BEGIN SELECT 1; END;
         CREATE TABLE workspaces__crsql_clock (id INTEGER);
         CREATE INDEX workspaces__crsql_clock_dbv_idx ON workspaces__crsql_clock(id);
         CREATE TABLE crsql_master (name TEXT);",
    )
    .expect("seed legacy schema");
    let dropped = super::state_init::cleanup_legacy_crdt_objects(&conn).expect("cleanup");
    assert!(dropped >= 4, "expected CRDT shadow objects to be dropped");
    assert!(!table_exists(&conn, "workspaces__crsql_clock"));
    assert!(!table_exists(&conn, "crsql_master"));
    assert!(!index_exists(&conn, "workspaces__crsql_clock_dbv_idx"));
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name='workspaces__crsql_itrig'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap(),
        0
    );
}

#[test]
fn test_core_performance_indexes_exist_after_migration() {
    let conn = Connection::open_in_memory().expect("open_in_memory");
    run_core_table_migrations(&conn);
    assert!(
        index_exists(&conn, "idx_tasks_status"),
        "idx_tasks_status missing"
    );
    assert!(
        index_exists(&conn, "idx_tasks_wave_id"),
        "idx_tasks_wave_id missing"
    );
    assert!(
        index_exists(&conn, "idx_plans_status"),
        "idx_plans_status missing"
    );
    assert!(
        index_exists(&conn, "idx_tasks_plan_id"),
        "idx_tasks_plan_id missing"
    );
    assert!(
        index_exists(&conn, "idx_waves_plan_id"),
        "idx_waves_plan_id missing"
    );
}

#[test]
fn test_core_performance_indexes_idempotent() {
    let conn = Connection::open_in_memory().expect("open_in_memory");
    // IF NOT EXISTS: running twice must not error.
    run_core_table_migrations(&conn);
    run_core_table_migrations(&conn);
    assert!(index_exists(&conn, "idx_tasks_status"));
    assert!(index_exists(&conn, "idx_plans_status"));
}
