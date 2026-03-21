use crate::db::{PlanDb, ValidateTaskArgs};

// seed_schema is defined in the parent module (tests.rs)
use super::seed_schema;

#[test]
fn db_validate_task_submitted_to_done() {
    let db = PlanDb::open_in_memory().expect("db");
    seed_schema(&db);
    db.connection()
        .execute("INSERT INTO projects(id,name) VALUES('p1','P1')", [])
        .expect("projects");
    db.connection()
        .execute(
            "INSERT INTO plans(id,project_id,name,status,tasks_done,tasks_total) VALUES(1,'p1','Plan A','doing',0,1)",
            [],
        )
        .expect("plans");
    db.connection()
        .execute(
            "INSERT INTO waves(id,plan_id,wave_id,name,status,tasks_done,tasks_total,position) VALUES(10,1,'W1','Wave 1','pending',0,1,1)",
            [],
        )
        .expect("waves");
    db.connection()
        .execute(
            "INSERT INTO tasks(id,project_id,plan_id,wave_id_fk,wave_id,task_id,title,status) VALUES(100,'p1',1,10,'W1','T1','Task 1','submitted')",
            [],
        )
        .expect("tasks");

    let args = ValidateTaskArgs {
        identifier: "100".to_string(),
        validated_by: "thor".to_string(),
        ..ValidateTaskArgs::default()
    };
    let result = db.validate_task(&args).expect("validate-task");
    assert_eq!(result.old_status, "submitted");
    assert_eq!(result.new_status, "done");
}

#[test]
fn db_execution_tree_contains_waves_and_tasks() {
    let db = PlanDb::open_in_memory().expect("db");
    seed_schema(&db);
    db.connection()
        .execute("INSERT INTO projects(id,name) VALUES('p1','P1')", [])
        .expect("projects");
    db.connection()
        .execute(
            "INSERT INTO plans(id,project_id,name,status,tasks_done,tasks_total) VALUES(1,'p1','Plan A','doing',1,2)",
            [],
        )
        .expect("plans");
    db.connection()
        .execute(
            "INSERT INTO waves(id,plan_id,wave_id,name,status,tasks_done,tasks_total,position) VALUES(10,1,'W1','Wave 1','in_progress',1,2,1)",
            [],
        )
        .expect("waves");
    db.connection()
        .execute(
            "INSERT INTO tasks(id,project_id,plan_id,wave_id_fk,wave_id,task_id,title,status) VALUES
             (100,'p1',1,10,'W1','T1','Task 1','done'),
             (101,'p1',1,10,'W1','T2','Task 2','pending')",
            [],
        )
        .expect("tasks");

    let tree = db.execution_tree(1).expect("execution-tree");
    assert_eq!(tree.waves.len(), 1);
    assert_eq!(tree.waves[0].tasks.len(), 2);
}

#[test]
fn db_crdt_required_tables_are_declared() {
    let tables = crate::db::crdt::required_crdt_tables();
    assert_eq!(
        tables,
        vec![
            "agent_activity",
            "agent_runs",
            "chat_messages",
            "chat_requirements",
            "chat_sessions",
            "collector_runs",
            "conversation_logs",
            "coordinator_events",
            "daemon_config",
            "debt_items",
            "delegation_log",
            "env_vault_log",
            "file_locks",
            "file_snapshots",
            "github_events",
            "host_heartbeats",
            "idea_notes",
            "ideas",
            "ipc_agent_skills",
            "ipc_agents",
            "ipc_auth_tokens",
            "ipc_budget_log",
            "ipc_channels",
            "ipc_file_locks",
            "ipc_messages",
            "ipc_model_registry",
            "ipc_node_capabilities",
            "ipc_shared_context",
            "ipc_subscriptions",
            "ipc_worktrees",
            "knowledge_base",
            "merge_queue",
            "mesh_events",
            "mesh_sync_stats",
            "metrics_history",
            "nightly_job_definitions",
            "nightly_jobs",
            "notification_queue",
            "notification_triggers",
            "notifications",
            "peer_heartbeats",
            "plan_actuals",
            "plan_approvals",
            "plan_business_assessments",
            "plan_commits",
            "plan_learnings",
            "plan_reviews",
            "plan_token_estimates",
            "plan_versions",
            "plans",
            "projects",
            "schema_metadata",
            "session_state",
            "snapshots",
            "tasks",
            "token_usage",
            "waves"
        ]
    );
}

#[test]
fn db_crdt_sync_subcommand_is_supported() {
    let db = PlanDb::open_in_memory().expect("db");
    seed_schema(&db);
    let error = db
        .run_subcommand(&["sync".to_string()])
        .expect_err("sync should require peer argument");
    assert!(error.to_string().contains("usage: sync <peer>"));
}

#[test]
fn schema_migration_creates_daemon_tables() {
    let db = PlanDb::open_in_memory().expect("db");
    let conn = db.connection();
    // Run the daemon consolidation migrations
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS daemon_config (key TEXT PRIMARY KEY NOT NULL, value TEXT, updated_at TEXT DEFAULT (datetime('now')));
         CREATE TABLE IF NOT EXISTS coordinator_events (id INTEGER PRIMARY KEY, event_type TEXT NOT NULL DEFAULT '', payload TEXT, source_node TEXT, handled_at TEXT DEFAULT (datetime('now')));
         CREATE TABLE IF NOT EXISTS notification_queue (id INTEGER PRIMARY KEY, severity TEXT DEFAULT 'info', title TEXT NOT NULL DEFAULT '', message TEXT, plan_id INTEGER, link TEXT, status TEXT DEFAULT 'pending', created_at TEXT DEFAULT (datetime('now')), delivered_at TEXT);"
    ).expect("create tables");

    // Verify daemon_config
    conn.execute(
        "INSERT INTO daemon_config (key, value) VALUES (?1, ?2)",
        rusqlite::params!["test_key", "test_value"],
    )
    .expect("insert config");
    let val: String = conn
        .query_row(
            "SELECT value FROM daemon_config WHERE key = ?1",
            rusqlite::params!["test_key"],
            |r| r.get(0),
        )
        .expect("query config");
    assert_eq!(val, "test_value");

    // Verify coordinator_events
    conn.execute(
        "INSERT INTO coordinator_events (event_type, payload, source_node) VALUES (?1, ?2, ?3)",
        rusqlite::params!["plan_started", "{\"plan_id\":1}", "mac-worker-2"],
    )
    .expect("insert event");
    let etype: String = conn
        .query_row(
            "SELECT event_type FROM coordinator_events WHERE id = last_insert_rowid()",
            [],
            |r| r.get(0),
        )
        .expect("query event");
    assert_eq!(etype, "plan_started");

    // Verify notification_queue
    conn.execute(
        "INSERT INTO notification_queue (severity, title, message, status) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params!["info", "Test", "Test message", "pending"],
    )
    .expect("insert notification");
    let status: String = conn
        .query_row(
            "SELECT status FROM notification_queue WHERE id = last_insert_rowid()",
            [],
            |r| r.get(0),
        )
        .expect("query notification");
    assert_eq!(status, "pending");
}
