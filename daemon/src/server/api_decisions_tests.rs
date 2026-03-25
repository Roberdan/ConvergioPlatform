// TDD tests for api_decisions — written BEFORE implementation (RED phase).
// F-27: Decision audit trail.

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;
    use crate::server::state::query_rows;

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS decision_log (
                     id INTEGER PRIMARY KEY,
                     plan_id INTEGER,
                     task_id INTEGER,
                     decision TEXT NOT NULL,
                     reasoning TEXT NOT NULL,
                     first_principles TEXT,
                     alternatives_considered TEXT,
                     outcome TEXT,
                     created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                     agent TEXT
                 );",
            )
            .expect("schema");
        db
    }

    #[test]
    fn decision_log_insert_and_query() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO decision_log (plan_id, task_id, decision, reasoning, agent) \
             VALUES (724, 9285, 'Use Ollama fallback', 'Ollama unavailable; hardcoded rules applied', 'task-executor')",
            [],
        )
        .unwrap();

        let rows = query_rows(
            conn,
            "SELECT id, decision, reasoning FROM decision_log WHERE plan_id = 724",
            [],
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].get("decision").and_then(|v| v.as_str()),
            Some("Use Ollama fallback")
        );
    }

    #[test]
    fn decision_log_with_first_principles() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO decision_log \
             (plan_id, decision, reasoning, first_principles, alternatives_considered) \
             VALUES (724, 'restart agent', 'agent stalled >5min', \
                     'resilience requires self-recovery', 'escalate vs restart')",
            [],
        )
        .unwrap();

        let row = query_rows(
            conn,
            "SELECT first_principles, alternatives_considered FROM decision_log LIMIT 1",
            [],
        )
        .unwrap();
        assert!(!row.is_empty());
        assert!(
            row[0]
                .get("first_principles")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .contains("resilience")
        );
    }

    #[test]
    fn decision_log_query_by_plan_id() {
        let db = setup_db();
        let conn = db.connection();

        for i in 0..3 {
            conn.execute(
                "INSERT INTO decision_log (plan_id, decision, reasoning) \
                 VALUES (?1, ?2, 'test reason')",
                rusqlite::params![if i < 2 { 724 } else { 725 }, format!("decision {i}")],
            )
            .unwrap();
        }

        let rows = query_rows(
            conn,
            "SELECT id FROM decision_log WHERE plan_id = 724",
            [],
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn decision_log_outcome_nullable() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO decision_log (decision, reasoning) VALUES ('test', 'reason')",
            [],
        )
        .unwrap();

        let row = query_rows(
            conn,
            "SELECT outcome FROM decision_log LIMIT 1",
            [],
        )
        .unwrap();
        // outcome is NULL by default
        assert!(row[0].get("outcome").map_or(true, |v| v.is_null()));
    }
}
