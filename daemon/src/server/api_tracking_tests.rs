// Copyright (c) 2026 Roberto D'Angelo
//! Tests for tracking API — token_usage, agent_activity, session_state, compaction.
use crate::db::PlanDb;
use crate::server::state::query_one;

fn setup_db() -> PlanDb {
    let db = PlanDb::open_in_memory().expect("db");
    db.connection()
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS token_usage (
                 id INTEGER PRIMARY KEY NOT NULL,
                 project_id TEXT, plan_id INT, wave_id TEXT, task_id TEXT,
                 agent TEXT, model TEXT,
                 input_tokens INT, output_tokens INT, cost_usd REAL,
                 created_at TEXT DEFAULT (datetime('now')),
                 execution_host TEXT
             );
             CREATE TABLE IF NOT EXISTS agent_activity (
                 id INTEGER PRIMARY KEY NOT NULL,
                 agent_id TEXT NOT NULL DEFAULT '',
                 task_db_id INTEGER, plan_id INTEGER,
                 action TEXT NOT NULL DEFAULT '',
                 details TEXT,
                 created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                 parent_session TEXT, agent_type TEXT NOT NULL DEFAULT 'legacy',
                 model TEXT, description TEXT,
                 status TEXT NOT NULL DEFAULT 'completed',
                 tokens_in INTEGER DEFAULT 0,
                 tokens_out INTEGER DEFAULT 0,
                 tokens_total INTEGER DEFAULT 0,
                 cost_usd REAL DEFAULT 0,
                 started_at TEXT, completed_at TEXT,
                 duration_s REAL, host TEXT, region TEXT, metadata TEXT
             );
             CREATE UNIQUE INDEX IF NOT EXISTS uq_agent_activity_agent_id
                 ON agent_activity(agent_id);
             CREATE TABLE IF NOT EXISTS session_state (
                 key TEXT PRIMARY KEY NOT NULL,
                 value TEXT
             );
             CREATE TABLE IF NOT EXISTS compaction_log (
                 id INTEGER PRIMARY KEY NOT NULL,
                 session_id TEXT,
                 event_type TEXT,
                 context TEXT,
                 created_at TEXT DEFAULT (datetime('now'))
             );",
        )
        .expect("schema");
    db
}

#[test]
fn token_usage_insert_required_fields() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO token_usage (agent, model, input_tokens, output_tokens, cost_usd)
         VALUES ('task-executor', 'claude-sonnet-4-6', 1000, 500, 0.01)",
        [],
    )
    .unwrap();

    let row = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM token_usage WHERE agent = 'task-executor'",
        [],
    )
    .expect("query")
    .expect("row");
    assert_eq!(row.get("c").and_then(|v| v.as_i64()), Some(1));
}

#[test]
fn token_usage_insert_with_optional_fields() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO token_usage
         (project_id, plan_id, wave_id, task_id, agent, model,
          input_tokens, output_tokens, cost_usd, execution_host)
         VALUES ('proj-1', 42, 'W1', '100', 'thor', 'claude-opus',
                 800, 400, 0.05, 'macbook-m5')",
        [],
    )
    .unwrap();

    let row = query_one(
        conn,
        "SELECT plan_id, cost_usd FROM token_usage WHERE agent = 'thor'",
        [],
    )
    .expect("query")
    .expect("row");
    assert_eq!(row.get("plan_id").and_then(|v| v.as_i64()), Some(42));
}

#[test]
fn agent_activity_upsert_by_agent_id() {
    let db = setup_db();
    let conn = db.connection();

    // Insert
    conn.execute(
        "INSERT INTO agent_activity (agent_id, action, status)
         VALUES ('agent-xyz', 'task_start', 'active')",
        [],
    )
    .unwrap();

    // Upsert (update status)
    conn.execute(
        "INSERT INTO agent_activity (agent_id, action, status, tokens_total, cost_usd)
         VALUES ('agent-xyz', 'task_complete', 'completed', 1500, 0.03)
         ON CONFLICT(agent_id) DO UPDATE SET
             action = excluded.action,
             status = excluded.status,
             tokens_total = excluded.tokens_total,
             cost_usd = excluded.cost_usd",
        [],
    )
    .unwrap();

    let row = query_one(
        conn,
        "SELECT status, tokens_total FROM agent_activity WHERE agent_id = 'agent-xyz'",
        [],
    )
    .expect("query")
    .expect("row");
    assert_eq!(
        row.get("status").and_then(|v| v.as_str()),
        Some("completed")
    );
    assert_eq!(
        row.get("tokens_total").and_then(|v| v.as_i64()),
        Some(1500)
    );
}

#[test]
fn session_state_insert_and_update() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO session_state (key, value) VALUES ('active_plan', '685')",
        [],
    )
    .unwrap();

    // Update via REPLACE (key is PRIMARY KEY)
    conn.execute(
        "INSERT OR REPLACE INTO session_state (key, value) VALUES ('active_plan', '700')",
        [],
    )
    .unwrap();

    let row = query_one(
        conn,
        "SELECT value FROM session_state WHERE key = 'active_plan'",
        [],
    )
    .expect("query")
    .expect("row");
    assert_eq!(row.get("value").and_then(|v| v.as_str()), Some("700"));
}

#[test]
fn compaction_log_insert_event() {
    let db = setup_db();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO compaction_log (session_id, event_type, context)
         VALUES ('sess-abc', 'pre_compact', 'wave W1 in progress')",
        [],
    )
    .unwrap();

    let row = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM compaction_log WHERE session_id = 'sess-abc'",
        [],
    )
    .expect("query")
    .expect("row");
    assert_eq!(row.get("c").and_then(|v| v.as_i64()), Some(1));
}
