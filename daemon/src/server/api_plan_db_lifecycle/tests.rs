use crate::db::PlanDb;

fn setup_db() -> PlanDb {
    let db = PlanDb::open_in_memory().expect("db");
    db.connection()
        .execute_batch(
            "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE plans (
                 id INTEGER PRIMARY KEY, project_id TEXT NOT NULL,
                 name TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'draft',
                 source_file TEXT, description TEXT, human_summary TEXT,
                 tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0,
                 execution_host TEXT, worktree_path TEXT, parallel_mode TEXT,
                 created_at TEXT, started_at TEXT, completed_at TEXT,
                 updated_at TEXT, cancelled_at TEXT, cancelled_reason TEXT,
                 constraints_json TEXT
             );
             CREATE TABLE waves (
                 id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id TEXT,
                 name TEXT, status TEXT DEFAULT 'pending',
                 tasks_done INTEGER DEFAULT 0, tasks_total INTEGER DEFAULT 0,
                 position INTEGER DEFAULT 0, worktree_path TEXT,
                 cancelled_at TEXT, cancelled_reason TEXT, project_id TEXT
             );
             CREATE TABLE tasks (
                 id INTEGER PRIMARY KEY, project_id TEXT, plan_id INTEGER,
                 wave_id_fk INTEGER, wave_id TEXT, task_id TEXT,
                 title TEXT, status TEXT DEFAULT 'pending',
                 started_at TEXT, completed_at TEXT, notes TEXT,
                 tokens INTEGER DEFAULT 0, description TEXT,
                 type TEXT, priority TEXT, assignee TEXT,
                 test_criteria TEXT, output_data TEXT, executor_host TEXT,
                 validated_at TEXT, validated_by TEXT, validation_report TEXT
             );
             INSERT INTO projects (id, name) VALUES ('test', 'Test');",
        )
        .expect("schema");
    db
}

#[test]
fn plan_db_lifecycle_create_start_complete() {
    let db = setup_db();
    let conn = db.connection();

    // Create plan
    conn.execute(
        "INSERT INTO plans (project_id, name, status) VALUES ('test', 'Plan A', 'draft')",
        [],
    )
    .expect("create");
    let plan_id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .unwrap();

    // Start
    let changed = conn
        .execute(
            "UPDATE plans SET status = 'doing', \
             started_at = datetime('now') \
             WHERE id = ?1 AND status IN ('draft', 'approved', 'todo')",
            rusqlite::params![plan_id],
        )
        .unwrap();
    assert_eq!(changed, 1);

    // Complete (no tasks, should succeed)
    let pending: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE plan_id = ?1 \
             AND status NOT IN ('done', 'cancelled', 'skipped')",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(pending, 0);

    let changed = conn
        .execute(
            "UPDATE plans SET status = 'completed', completed_at = datetime('now') \
             WHERE id = ?1 AND status IN ('doing', 'approved')",
            rusqlite::params![plan_id],
        )
        .unwrap();
    assert_eq!(changed, 1);
}

#[test]
fn plan_db_lifecycle_cancel_cascades() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO plans (project_id, name, status) VALUES ('test', 'Plan B', 'doing')",
        [],
    )
    .unwrap();
    let plan_id: i64 = conn
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .unwrap();

    conn.execute(
        "INSERT INTO waves (plan_id, wave_id, name, status, project_id) \
         VALUES (?1, 'W1', 'Wave 1', 'pending', 'test')",
        rusqlite::params![plan_id],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO tasks (plan_id, wave_id, task_id, title, status, project_id, wave_id_fk) \
         VALUES (?1, 'W1', 'T1', 'Task 1', 'pending', 'test', 1), \
                (?1, 'W1', 'T2', 'Task 2', 'in_progress', 'test', 1)",
        rusqlite::params![plan_id],
    )
    .unwrap();

    // Cancel cascades to tasks
    let tasks_cancelled = conn
        .execute(
            "UPDATE tasks SET status = 'cancelled' \
             WHERE plan_id = ?1 AND status IN ('pending', 'in_progress')",
            rusqlite::params![plan_id],
        )
        .unwrap();
    assert_eq!(tasks_cancelled, 2);

    conn.execute(
        "UPDATE plans SET status = 'cancelled' WHERE id = ?1",
        rusqlite::params![plan_id],
    )
    .unwrap();

    let status: String = conn
        .query_row(
            "SELECT status FROM plans WHERE id = ?1",
            rusqlite::params![plan_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(status, "cancelled");
}
