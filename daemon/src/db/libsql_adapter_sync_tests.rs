// Sync subcommand routing test extracted from libsql_adapter_tests.rs.
// Why: keep libsql_adapter_tests.rs ≤250 lines per CONSTITUTION Article V.

#[test]
fn cli_sync_commands_point_to_timestamp_adapter() {
    // After crsqlite removal, sync CLI commands should direct users
    // to the timestamp-based sync (libsql_adapter).
    let db = crate::db::PlanDb::open_in_memory().expect("db");
    // Seed minimal schema for subcommand dispatch
    db.connection()
        .execute_batch(
            "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT);
             CREATE TABLE plans (id INTEGER PRIMARY KEY, project_id TEXT, name TEXT, status TEXT, tasks_done INTEGER DEFAULT 0, tasks_total INTEGER DEFAULT 0);
             CREATE TABLE waves (id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id TEXT, name TEXT, status TEXT, tasks_done INTEGER DEFAULT 0, tasks_total INTEGER DEFAULT 0, position INTEGER DEFAULT 0);
             CREATE TABLE tasks (id INTEGER PRIMARY KEY, project_id TEXT, plan_id INTEGER, wave_id_fk INTEGER, wave_id TEXT, task_id TEXT, title TEXT, status TEXT, started_at TEXT, completed_at TEXT, notes TEXT, tokens INTEGER, output_data TEXT, executor_host TEXT, validated_at TEXT, validated_by TEXT, validation_report TEXT);",
        )
        .expect("schema");
    for cmd in &["export-changes", "apply-changes", "sync"] {
        let err = db
            .run_subcommand(&[cmd.to_string()])
            .expect_err(&format!("{cmd} should return error"));
        let msg = err.to_string();
        assert!(
            msg.contains("timestamp-based sync") || msg.contains("libsql_adapter"),
            "{cmd} error should mention timestamp sync, got: {msg}"
        );
    }
}
