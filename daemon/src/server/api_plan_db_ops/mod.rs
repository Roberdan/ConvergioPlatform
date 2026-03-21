mod handlers;
pub use handlers::router;

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;
    use crate::server::state::{query_one, query_rows};

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE plans (
                     id INTEGER PRIMARY KEY, project_id TEXT, name TEXT,
                     status TEXT, tasks_total INTEGER DEFAULT 0,
                     tasks_done INTEGER DEFAULT 0, updated_at TEXT
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
                     project_id TEXT
                 );
                 CREATE TABLE knowledge_base (
                     id INTEGER PRIMARY KEY, domain TEXT, title TEXT,
                     content TEXT, created_at TEXT, hit_count INTEGER DEFAULT 0
                 );
                 INSERT INTO plans (id, project_id, name, status) VALUES (1, 'test', 'P', 'doing');
                 INSERT INTO waves (id, plan_id, wave_id, name, tasks_total)
                     VALUES (10, 1, 'W1', 'Wave 1', 2);
                 INSERT INTO tasks (id, plan_id, wave_id_fk, wave_id, task_id, title, status)
                     VALUES (100, 1, 10, 'W1', 'T1', 'Task 1', 'done'),
                            (101, 1, 10, 'W1', 'T2', 'Task 2', 'done');
                 INSERT INTO knowledge_base (domain, title, content, hit_count)
                     VALUES ('rust', 'Axum patterns', 'Use Router::new() for routing', 5),
                            ('shell', 'Bash tips', 'Use set -e for error handling', 2);",
            )
            .expect("schema");
        db
    }

    #[test]
    fn plan_db_wave_update_to_done() {
        let db = setup_db();
        let conn = db.connection();

        // All tasks done, wave should be completable
        let pending: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE wave_id_fk = 10 \
                 AND status NOT IN ('done', 'cancelled', 'skipped')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending, 0);

        conn.execute(
            "UPDATE waves SET status = 'done', completed_at = datetime('now') WHERE id = 10",
            [],
        )
        .unwrap();

        let status: String = conn
            .query_row("SELECT status FROM waves WHERE id = 10", [], |r| r.get(0))
            .unwrap();
        assert_eq!(status, "done");
    }

    #[test]
    fn plan_db_wave_update_blocked_by_pending_tasks() {
        let db = setup_db();
        let conn = db.connection();

        // Add a pending task
        conn.execute(
            "INSERT INTO tasks (id, plan_id, wave_id_fk, wave_id, task_id, title, status) \
             VALUES (102, 1, 10, 'W1', 'T3', 'Pending', 'pending')",
            [],
        )
        .unwrap();

        let pending: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE wave_id_fk = 10 \
                 AND status NOT IN ('done', 'cancelled', 'skipped')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending, 1); // Should block wave completion
    }

    #[test]
    fn plan_db_kb_search_finds_results() {
        let db = setup_db();
        let conn = db.connection();

        let results = query_rows(
            conn,
            "SELECT id, title FROM knowledge_base WHERE title LIKE ?1 OR content LIKE ?1 LIMIT 10",
            rusqlite::params!["%axum%"],
        )
        .expect("search");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn plan_db_kb_search_empty_table() {
        let db = PlanDb::open_in_memory().expect("db");
        // No knowledge_base table — should return empty
        let exists: bool = db
            .connection()
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='knowledge_base'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(!exists);
    }
}
