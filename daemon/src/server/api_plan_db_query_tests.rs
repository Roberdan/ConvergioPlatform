//! Unit tests for api_plan_db_query: list, drift-check, validate-task queries.

use crate::db::PlanDb;
use crate::server::state::{query_one, query_rows};

fn setup_db() -> PlanDb {
    let db = PlanDb::open_in_memory().expect("db");
    db.connection()
        .execute_batch(
            "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE plans (
                 id INTEGER PRIMARY KEY, project_id TEXT NOT NULL, name TEXT NOT NULL,
                 status TEXT DEFAULT 'todo', execution_host TEXT, worktree_path TEXT,
                 description TEXT, human_summary TEXT, parallel_mode TEXT,
                 tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0,
                 created_at TEXT, started_at TEXT, updated_at TEXT,
                 waves_total INTEGER DEFAULT 0, waves_merged INTEGER DEFAULT 0);
             CREATE TABLE waves (
                 id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id TEXT, name TEXT,
                 status TEXT DEFAULT 'pending', tasks_done INTEGER DEFAULT 0,
                 tasks_total INTEGER DEFAULT 0, position INTEGER DEFAULT 0,
                 depends_on TEXT, worktree_path TEXT, branch_name TEXT,
                 started_at TEXT, completed_at TEXT);
             CREATE TABLE tasks (
                 id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id_fk INTEGER,
                 wave_id TEXT, task_id TEXT, title TEXT, status TEXT DEFAULT 'pending',
                 priority TEXT, type TEXT, assignee TEXT, test_criteria TEXT,
                 description TEXT, model TEXT, started_at TEXT, completed_at TEXT,
                 validated_at TEXT, validated_by TEXT, validation_report TEXT,
                 executor_host TEXT, notes TEXT);
             CREATE TABLE deliverables (
                 id INTEGER PRIMARY KEY, task_id INTEGER, project_id TEXT NOT NULL,
                 name TEXT NOT NULL, output_type TEXT NOT NULL DEFAULT 'file',
                 status TEXT DEFAULT 'pending');
             INSERT INTO projects (id, name) VALUES ('proj', 'Test');
             INSERT INTO plans (id, project_id, name, status, tasks_total,
                 tasks_done, waves_total, waves_merged)
                 VALUES (1, 'proj', 'Active Plan', 'doing', 4, 2, 2, 1);
             INSERT INTO plans (id, project_id, name, status)
                 VALUES (2, 'proj', 'Completed Plan', 'completed');
             INSERT INTO plans (id, project_id, name, status)
                 VALUES (3, 'proj', 'Cancelled Plan', 'cancelled');
             INSERT INTO waves (id, plan_id, wave_id, name, status,
                 tasks_total, tasks_done, position)
                 VALUES (10, 1, 'W1', 'Wave 1', 'done', 2, 2, 1);
             INSERT INTO waves (id, plan_id, wave_id, name, status,
                 tasks_total, tasks_done, position)
                 VALUES (11, 1, 'W2', 'Wave 2', 'in_progress', 2, 0, 2);
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status, priority)
                 VALUES (100, 1, 10, 'T1', 'Done task', 'done', 'P0');
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status, priority)
                 VALUES (101, 1, 10, 'T2', 'Also done', 'done', 'P1');
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status, priority,
                 started_at)
                 VALUES (102, 1, 11, 'T3', 'In progress', 'in_progress', 'P0',
                     datetime('now', '-48 hours'));
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status, priority,
                 test_criteria, validated_by)
                 VALUES (103, 1, 11, 'T4', 'Submitted', 'submitted', 'P1',
                     'cargo test passes', 'admin');",
        )
        .expect("schema");
    db
}

// --- handle_list query logic ---

#[test]
fn list_shows_all_plans_by_default() {
    let db = setup_db();
    let plans = query_rows(
        db.connection(),
        "SELECT id, name, status FROM plans ORDER BY id DESC LIMIT 20",
        [],
    )
    .expect("query");
    assert_eq!(plans.len(), 3, "all plans shown by default");
    assert_eq!(plans[0]["name"].as_str().unwrap(), "Cancelled Plan");
}

#[test]
fn list_filters_by_active_status() {
    let db = setup_db();
    let plans = query_rows(
        db.connection(),
        "SELECT id, name, status FROM plans \
         WHERE status NOT IN ('completed', 'cancelled', 'done') ORDER BY id DESC",
        [],
    )
    .expect("query");
    assert_eq!(plans.len(), 1, "only active plan with ?status=active");
    assert_eq!(plans[0]["name"].as_str().unwrap(), "Active Plan");
}

#[test]
fn list_includes_merge_pct() {
    let db = setup_db();
    let plans = query_rows(
        db.connection(),
        "SELECT id, \
         CASE WHEN COALESCE(waves_total, 0) > 0 \
           THEN COALESCE(waves_merged, 0) * 100 / waves_total \
           ELSE 0 END AS merge_pct \
         FROM plans WHERE id = 1",
        [],
    )
    .expect("query");
    assert_eq!(plans[0]["merge_pct"].as_i64().unwrap(), 50);
}

#[test]
fn list_includes_deliverables_counts() {
    let db = setup_db();
    let conn = db.connection();
    conn.execute_batch(
        "INSERT INTO deliverables (id, task_id, project_id, name, output_type, status)
             VALUES (1, 100, 'proj', 'D1', 'file', 'approved');
         INSERT INTO deliverables (id, task_id, project_id, name, output_type, status)
             VALUES (2, 101, 'proj', 'D2', 'pr', 'pending');
         INSERT INTO deliverables (id, task_id, project_id, name, output_type, status)
             VALUES (3, 101, 'proj', 'D3', 'code', 'pending');",
    )
    .unwrap();
    let plans = query_rows(
        conn,
        "SELECT p.id, \
         (SELECT COUNT(*) FROM deliverables d JOIN tasks t ON d.task_id = t.id \
           WHERE t.plan_id = p.id AND d.status = 'approved') AS deliverables_approved, \
         (SELECT COUNT(*) FROM deliverables d JOIN tasks t ON d.task_id = t.id \
           WHERE t.plan_id = p.id AND COALESCE(d.output_type, '') != 'pr') AS deliverables_total \
         FROM plans p WHERE p.id = 1",
        [],
    )
    .expect("query");
    assert_eq!(plans[0]["deliverables_approved"].as_i64().unwrap(), 1);
    assert_eq!(plans[0]["deliverables_total"].as_i64().unwrap(), 2);
}

// --- handle_drift_check query logic ---

#[test]
fn drift_check_finds_stale_tasks() {
    let db = setup_db();
    let stale = query_rows(
        db.connection(),
        "SELECT id, task_id, title, status FROM tasks \
         WHERE plan_id = ?1 AND status = 'in_progress' \
         AND started_at < datetime('now', '-24 hours') ORDER BY started_at",
        rusqlite::params![1],
    )
    .expect("query");
    assert_eq!(stale.len(), 1, "task 102 started 48h ago is stale");
    assert_eq!(stale[0]["task_id"].as_str().unwrap(), "T3");
}

#[test]
fn drift_check_counts_in_progress() {
    let db = setup_db();
    let count = query_one(
        db.connection(),
        "SELECT COUNT(*) AS c FROM tasks \
         WHERE plan_id = ?1 AND status = 'in_progress'",
        rusqlite::params![1],
    )
    .expect("query")
    .and_then(|v| v["c"].as_i64())
    .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn drift_check_no_stale_when_fresh() {
    let db = setup_db();
    let conn = db.connection();
    conn.execute(
        "UPDATE tasks SET started_at = datetime('now') WHERE id = 102",
        [],
    )
    .unwrap();
    let stale = query_rows(
        conn,
        "SELECT id FROM tasks WHERE plan_id = 1 AND status = 'in_progress' \
         AND started_at < datetime('now', '-24 hours')",
        [],
    )
    .expect("query");
    assert!(stale.is_empty());
}

// --- handle_validate_task query logic ---

#[test]
fn validate_task_query_returns_task_fields() {
    let db = setup_db();
    let task = query_one(
        db.connection(),
        "SELECT id, task_id, title, status, test_criteria, notes, \
         validated_at, validated_by FROM tasks WHERE id = ?1 AND plan_id = ?2",
        rusqlite::params![103, 1],
    )
    .expect("query")
    .expect("task exists");
    assert_eq!(task["status"].as_str().unwrap(), "submitted");
    assert_eq!(task["test_criteria"].as_str().unwrap(), "cargo test passes");
    assert_eq!(task["validated_by"].as_str().unwrap(), "admin");
}

#[test]
fn validate_task_not_found_and_wrong_plan() {
    let db = setup_db();
    let conn = db.connection();
    // Nonexistent task returns None
    let t = query_one(
        conn,
        "SELECT id FROM tasks WHERE id = ?1 AND plan_id = ?2",
        rusqlite::params![999, 1],
    )
    .expect("query");
    assert!(t.is_none());
    // Task 100 exists in plan 1 but not plan 99
    let t = query_one(
        conn,
        "SELECT id FROM tasks WHERE id = ?1 AND plan_id = ?2",
        rusqlite::params![100, 99],
    )
    .expect("query");
    assert!(t.is_none());
}

#[test]
fn validate_task_is_validated_logic() {
    let db = setup_db();
    let conn = db.connection();
    // Task 103 has validated_by='admin'
    let t = query_one(conn, "SELECT validated_by FROM tasks WHERE id = 103", [])
        .expect("q")
        .unwrap();
    let v = t.get("validated_by").is_some() && !t["validated_by"].as_str().unwrap_or("").is_empty();
    assert!(v, "task 103 should be validated");
    // Task 100 has validated_by=NULL
    let t = query_one(conn, "SELECT validated_by FROM tasks WHERE id = 100", [])
        .expect("q")
        .unwrap();
    let v = t.get("validated_by").is_some() && !t["validated_by"].as_str().unwrap_or("").is_empty();
    assert!(!v, "task 100 should not be validated");
}
