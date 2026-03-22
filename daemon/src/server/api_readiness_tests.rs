use crate::db::PlanDb;
use crate::server::state::query_one;

fn setup_db() -> PlanDb {
    let db = PlanDb::open_in_memory().expect("db");
    db.connection()
        .execute_batch(
            "CREATE TABLE plans (
                 id INTEGER PRIMARY KEY, project_id TEXT, name TEXT,
                 status TEXT, tasks_total INTEGER DEFAULT 0,
                 tasks_done INTEGER DEFAULT 0, updated_at TEXT,
                 waves_total INTEGER DEFAULT 0, waves_merged INTEGER DEFAULT 0
             );
             CREATE TABLE waves (
                 id INTEGER PRIMARY KEY, plan_id INTEGER, wave_id TEXT,
                 name TEXT, status TEXT DEFAULT 'pending',
                 started_at TEXT, completed_at TEXT,
                 tasks_total INTEGER DEFAULT 0, tasks_done INTEGER DEFAULT 0
             );
             CREATE TABLE tasks (
                 id INTEGER PRIMARY KEY, plan_id INTEGER,
                 wave_id_fk INTEGER, wave_id TEXT, task_id TEXT,
                 title TEXT, status TEXT DEFAULT 'pending',
                 project_id TEXT, model TEXT
             );
             CREATE TABLE plan_reviews (
                 id INTEGER PRIMARY KEY, plan_id INTEGER,
                 reviewer_agent TEXT, verdict TEXT,
                 suggestions TEXT, raw_report TEXT, reviewed_at TEXT
             );",
        )
        .expect("schema");
    db
}

#[test]
fn readiness_all_gates_pass() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO plans (id, project_id, name, status, tasks_total) \
         VALUES (1, 'proj', 'Plan Alpha', 'doing', 2)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, plan_id, task_id, title, model) \
         VALUES (10, 1, 'T1-01', 'First task', 'claude-sonnet-4.6')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, plan_id, task_id, title, model) \
         VALUES (11, 1, 'T1-02', 'Second task', 'claude-opus-4.6')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict) \
         VALUES (1, 'plan-reviewer', 'approved')",
        [],
    )
    .unwrap();

    let result = super::check_readiness(conn, 1).expect("readiness check");
    assert!(result.ready, "all gates pass, should be ready");
    assert!(result.errors.is_empty(), "no errors expected");
    assert!(result.gates.iter().all(|g| g.passed), "all gates must pass");
}

#[test]
fn readiness_fails_when_no_tasks() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO plans (id, project_id, name, status, tasks_total) \
         VALUES (1, 'proj', 'Empty Plan', 'doing', 0)",
        [],
    )
    .unwrap();

    let result = super::check_readiness(conn, 1).expect("readiness check");
    assert!(!result.ready, "plan with no tasks should not be ready");
    assert!(
        result.gates.iter().any(|g| g.name == "spec_imported" && !g.passed),
        "spec_imported gate must fail"
    );
}

#[test]
fn readiness_fails_when_no_review() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO plans (id, project_id, name, status, tasks_total) \
         VALUES (1, 'proj', 'No Review', 'doing', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, plan_id, task_id, title, model) \
         VALUES (10, 1, 'T1-01', 'Task', 'claude-sonnet-4.6')",
        [],
    )
    .unwrap();

    let result = super::check_readiness(conn, 1).expect("readiness check");
    assert!(!result.ready, "plan without review should not be ready");
    assert!(
        result.gates.iter().any(|g| g.name == "review_approved" && !g.passed),
        "review_approved gate must fail"
    );
}

#[test]
fn readiness_fails_when_task_missing_model() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO plans (id, project_id, name, status, tasks_total) \
         VALUES (1, 'proj', 'Missing Model', 'doing', 1)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO tasks (id, plan_id, task_id, title) \
         VALUES (10, 1, 'T1-01', 'No Model Task')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO plan_reviews (plan_id, reviewer_agent, verdict) \
         VALUES (1, 'plan-reviewer', 'approved')",
        [],
    )
    .unwrap();

    let result = super::check_readiness(conn, 1).expect("readiness check");
    assert!(!result.ready, "task without model should block readiness");
    assert!(
        result.gates.iter().any(|g| g.name == "all_tasks_have_model" && !g.passed),
        "all_tasks_have_model gate must fail"
    );
}

#[test]
fn readiness_plan_not_found() {
    let db = setup_db();
    let conn = db.connection();

    let result = super::check_readiness(conn, 999);
    assert!(result.is_err(), "non-existent plan should return error");
}

#[test]
fn waves_total_and_merged_columns_exist() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO plans (id, project_id, name, status, waves_total, waves_merged) \
         VALUES (1, 'proj', 'Wave Plan', 'doing', 3, 1)",
        [],
    )
    .unwrap();

    let row = query_one(
        conn,
        "SELECT waves_total, waves_merged FROM plans WHERE id = 1",
        [],
    )
    .expect("query")
    .expect("row");

    assert_eq!(row.get("waves_total").and_then(|v| v.as_i64()), Some(3));
    assert_eq!(row.get("waves_merged").and_then(|v| v.as_i64()), Some(1));
}

#[test]
fn merge_pct_calculation() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO plans (id, project_id, name, status, waves_total, waves_merged) \
         VALUES (1, 'proj', 'Merge PCT', 'doing', 4, 2)",
        [],
    )
    .unwrap();

    let row = query_one(
        conn,
        "SELECT CASE WHEN waves_total > 0 \
         THEN waves_merged * 100 / waves_total ELSE 0 END AS merge_pct \
         FROM plans WHERE id = 1",
        [],
    )
    .expect("query")
    .expect("row");

    assert_eq!(row.get("merge_pct").and_then(|v| v.as_i64()), Some(50));
}
