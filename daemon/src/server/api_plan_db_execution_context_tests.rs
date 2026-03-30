//! Tests for execution-context and set-worktree endpoints.

use crate::db::PlanDb;
use crate::server::api_plan_db_execution_context::{
    build_execution_context, set_worktree_in_db,
};
use crate::server::state::{query_one, query_rows};

fn setup_db() -> PlanDb {
    let db = PlanDb::open_in_memory().expect("db");
    db.connection()
        .execute_batch(
            "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE plans (
                 id INTEGER PRIMARY KEY, project_id TEXT NOT NULL, name TEXT NOT NULL,
                 status TEXT DEFAULT 'todo', execution_host TEXT, worktree_path TEXT,
                 branch_name TEXT, description TEXT, human_summary TEXT,
                 parallel_mode TEXT, tasks_total INTEGER DEFAULT 0,
                 tasks_done INTEGER DEFAULT 0, created_at TEXT, started_at TEXT,
                 updated_at TEXT, waves_total INTEGER DEFAULT 0,
                 waves_merged INTEGER DEFAULT 0);
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
                 executor_host TEXT, notes TEXT, executor_agent TEXT);
             CREATE TABLE decision_log (
                 id INTEGER PRIMARY KEY, plan_id INTEGER, task_id TEXT,
                 decision TEXT, reasoning TEXT, agent TEXT,
                 first_principles TEXT, outcome TEXT,
                 created_at TEXT DEFAULT (datetime('now')));
             INSERT INTO projects (id, name) VALUES ('proj', 'Convergio');",
        )
        .expect("schema");
    db
}

fn seed_active_plan(db: &PlanDb) {
    db.connection()
        .execute_batch(
            "INSERT INTO plans (id, project_id, name, status, worktree_path,
                 branch_name, tasks_total, tasks_done, waves_total)
                 VALUES (742, 'proj', 'Plan X v2 — Convergio Hardening', 'doing',
                     '/tmp/worktree-test', 'plan-x-v2-hardening', 6, 2, 2);
             INSERT INTO waves (id, plan_id, wave_id, name, status,
                 tasks_total, tasks_done, position)
                 VALUES (10, 742, 'W1', 'libSQL Migration', 'in_progress', 3, 2, 1);
             INSERT INTO waves (id, plan_id, wave_id, name, status,
                 tasks_total, tasks_done, position)
                 VALUES (11, 742, 'W2', 'Bug Fixes', 'pending', 3, 0, 2);
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status,
                 model, executor_agent, test_criteria, wave_id)
                 VALUES (9520, 742, 10, 'T1-01', 'Migrate schema', 'done',
                     'claude-opus-4-6', 'copilot', 'cargo test passes', 'W1');
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status,
                 model, executor_agent, test_criteria, wave_id)
                 VALUES (9521, 742, 10, 'T1-02', 'Migrate queries', 'done',
                     'claude-opus-4-6', 'copilot', 'cargo check', 'W1');
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status,
                 model, executor_agent, test_criteria, wave_id)
                 VALUES (9522, 742, 10, 'T1-03', 'Fix adapter', 'pending',
                     'claude-opus-4-6', 'copilot',
                     'cargo check\ngrep -q adapter daemon/src/db/', 'W1');
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status,
                 model, executor_agent, test_criteria, wave_id)
                 VALUES (9523, 742, 11, 'T2-01', 'Fix bug B1', 'pending',
                     'claude-opus-4-6', 'copilot', 'cargo test', 'W2');
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status,
                 model, executor_agent, test_criteria, wave_id)
                 VALUES (9524, 742, 11, 'T2-02', 'Fix bug B2', 'pending',
                     'claude-opus-4-6', 'copilot', 'cargo test', 'W2');
             INSERT INTO tasks (id, plan_id, wave_id_fk, task_id, title, status,
                 model, executor_agent, test_criteria, wave_id)
                 VALUES (9525, 742, 11, 'T2-03', 'Fix bug B3', 'pending',
                     'claude-opus-4-6', 'copilot', 'cargo test', 'W2');
             INSERT INTO decision_log (plan_id, decision, reasoning)
                 VALUES (742, 'libSQL async-only fallback to rusqlite', 'async API incompatible');",
        )
        .expect("seed");
}

#[test]
fn execution_context_returns_next_pending_task_and_prompt() {
    let db = setup_db();
    seed_active_plan(&db);
    let ctx = build_execution_context(db.connection(), 742, None).expect("ctx");
    assert_eq!(ctx["ok"].as_bool(), Some(true));
    assert_eq!(ctx["plan_id"].as_i64(), Some(742));
    assert_eq!(ctx["status"].as_str(), Some("doing"));
    assert_eq!(ctx["worktree_path"].as_str(), Some("/tmp/worktree-test"));
    assert_eq!(ctx["branch"].as_str(), Some("plan-x-v2-hardening"));

    let wave = &ctx["current_wave"];
    assert_eq!(wave["id"].as_str(), Some("W1"));
    assert_eq!(wave["all_submitted"].as_bool(), Some(false));
    assert_eq!(wave["needs_thor"].as_bool(), Some(false));

    let task = &ctx["next_task"];
    assert_eq!(task["db_id"].as_i64(), Some(9522));
    assert_eq!(task["task_id"].as_str(), Some("T1-03"));
    assert_eq!(task["status"].as_str(), Some("pending"));

    let prompt = ctx["prompt"].as_str().expect("prompt string");
    assert!(prompt.contains("T1-03"), "prompt contains task_id");
    assert!(prompt.contains("9522"), "prompt contains db_id");
    assert!(prompt.contains("/tmp/worktree-test"), "prompt has worktree");
    assert!(prompt.contains("cargo check"), "prompt has verify commands");
}

#[test]
fn execution_context_needs_thor_when_all_submitted() {
    let db = setup_db();
    seed_active_plan(&db);
    // Mark the pending W1 task as submitted
    db.connection()
        .execute(
            "UPDATE tasks SET status = 'submitted' WHERE id = 9522",
            [],
        )
        .unwrap();
    db.connection()
        .execute(
            "UPDATE waves SET tasks_done = 2, status = 'in_progress' WHERE id = 10",
            [],
        )
        .unwrap();

    let ctx = build_execution_context(db.connection(), 742, None).expect("ctx");
    let wave = &ctx["current_wave"];
    assert_eq!(wave["all_submitted"].as_bool(), Some(true));
    assert_eq!(wave["needs_thor"].as_bool(), Some(true));

    let prompt = ctx["prompt"].as_str().expect("prompt");
    assert!(
        prompt.contains("Thor") || prompt.contains("validate"),
        "prompt instructs Thor validation"
    );
}

#[test]
fn execution_context_completed_plan_no_pending() {
    let db = setup_db();
    seed_active_plan(&db);
    // Mark all tasks done, all waves done
    db.connection()
        .execute_batch(
            "UPDATE tasks SET status = 'done' WHERE plan_id = 742;
             UPDATE waves SET status = 'done' WHERE plan_id = 742;
             UPDATE plans SET status = 'done' WHERE id = 742;",
        )
        .unwrap();

    let ctx = build_execution_context(db.connection(), 742, None).expect("ctx");
    assert_eq!(ctx["status"].as_str(), Some("done"));
    assert!(ctx["next_task"].is_null(), "no next task for done plan");
    let prompt = ctx["prompt"].as_str().expect("prompt");
    assert!(
        prompt.contains("complete") || prompt.contains("done"),
        "prompt says plan is complete"
    );
}

#[test]
fn execution_context_includes_decisions() {
    let db = setup_db();
    seed_active_plan(&db);
    let ctx = build_execution_context(db.connection(), 742, None).expect("ctx");
    let decisions = ctx["decisions"].as_array().expect("decisions array");
    assert!(!decisions.is_empty(), "should include decision_log entries");
    let first = decisions[0].as_str().expect("string");
    assert!(first.contains("libSQL"), "decision mentions libSQL");
}

#[test]
fn set_worktree_updates_plan() {
    let db = setup_db();
    seed_active_plan(&db);
    set_worktree_in_db(db.connection(), 742, "/new/path").expect("set");
    let plan = query_one(
        db.connection(),
        "SELECT worktree_path FROM plans WHERE id = 742",
        [],
    )
    .expect("query")
    .expect("plan");
    assert_eq!(plan["worktree_path"].as_str(), Some("/new/path"));
}

#[test]
fn set_worktree_fails_for_missing_plan() {
    let db = setup_db();
    seed_active_plan(&db);
    let result = set_worktree_in_db(db.connection(), 9999, "/path");
    assert!(result.is_err(), "should fail for nonexistent plan");
}

#[test]
fn execution_context_verify_includes_test_criteria_lines() {
    let db = setup_db();
    seed_active_plan(&db);
    let ctx = build_execution_context(db.connection(), 742, None).expect("ctx");
    let task = &ctx["next_task"];
    let verify = task["verify"].as_array().expect("verify array");
    assert_eq!(verify.len(), 2, "two verify lines from test_criteria");
    assert_eq!(verify[0].as_str(), Some("cargo check"));
}
