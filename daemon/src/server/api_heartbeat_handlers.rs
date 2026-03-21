// System info collection and tests extracted from api_heartbeat.rs (250-line split).
use serde_json::{json, Value};

pub fn collect_system_info() -> Value {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    json!({
        "cpu_usage": sys.global_cpu_usage(),
        "total_memory_mb": sys.total_memory() / 1_048_576,
        "used_memory_mb": sys.used_memory() / 1_048_576,
        "hostname": hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;
    use crate::server::state::query_one;

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE peer_heartbeats (
                     peer_name TEXT PRIMARY KEY, last_seen REAL,
                     load_json TEXT, capabilities TEXT
                 );
                 CREATE TABLE host_heartbeats (
                     hostname TEXT PRIMARY KEY, last_seen TEXT,
                     status TEXT, metadata TEXT
                 );
                 CREATE TABLE tasks (
                     id INTEGER PRIMARY KEY, task_id TEXT, title TEXT,
                     status TEXT, started_at TEXT, plan_id INTEGER
                 );
                 CREATE TABLE agent_activity (
                     id INTEGER PRIMARY KEY, agent_id TEXT, agent_type TEXT,
                     status TEXT, started_at TEXT
                 );
                 CREATE TABLE notification_queue (
                     id INTEGER PRIMARY KEY, status TEXT DEFAULT 'pending'
                 );
                 CREATE TABLE plans (
                     id INTEGER PRIMARY KEY, status TEXT
                 );",
            )
            .expect("schema");
        db
    }

    #[test]
    fn heartbeat_inserts_peer() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT OR REPLACE INTO peer_heartbeats (peer_name, last_seen) \
             VALUES ('test-node', strftime('%s','now'))",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM peer_heartbeats", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn watchdog_detects_stale_tasks() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO tasks (task_id, title, status, started_at) \
             VALUES ('T1', 'Stale', 'in_progress', datetime('now', '-48 hours'))",
            [],
        )
        .unwrap();

        let stale = query_one(
            conn,
            "SELECT COUNT(*) AS c FROM tasks WHERE status = 'in_progress' \
             AND started_at < datetime('now', '-24 hours')",
            [],
        )
        .unwrap()
        .and_then(|v| v.get("c").and_then(|c| c.as_i64()))
        .unwrap_or(0);
        assert_eq!(stale, 1);
    }
}
