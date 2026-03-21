mod handlers;
pub use handlers::router;

#[cfg(test)]
mod tests {
    use crate::db::PlanDb;
    use crate::server::state::query_rows;

    fn setup_db() -> PlanDb {
        let db = PlanDb::open_in_memory().expect("db");
        db.connection()
            .execute_batch(
                "CREATE TABLE notification_queue (
                     id INTEGER PRIMARY KEY, severity TEXT DEFAULT 'info',
                     title TEXT NOT NULL DEFAULT '', message TEXT,
                     plan_id INTEGER, link TEXT,
                     status TEXT DEFAULT 'pending',
                     created_at TEXT DEFAULT (datetime('now')),
                     delivered_at TEXT
                 );",
            )
            .expect("schema");
        db
    }

    #[test]
    fn api_notify_insert_and_query() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO notification_queue (severity, title, message, status) \
             VALUES ('info', 'Test', 'Hello world', 'pending')",
            [],
        )
        .unwrap();

        let pending = query_rows(
            conn,
            "SELECT id, title FROM notification_queue WHERE status = 'pending'",
            [],
        )
        .unwrap();
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn api_notify_deliver_marks_done() {
        let db = setup_db();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO notification_queue (severity, title, status) \
             VALUES ('info', 'N1', 'pending'), ('info', 'N2', 'pending')",
            [],
        )
        .unwrap();

        let changed = conn
            .execute(
                "UPDATE notification_queue SET status = 'delivered', \
                 delivered_at = datetime('now') WHERE id = 1 AND status = 'pending'",
                rusqlite::params![],
            )
            .unwrap();
        assert_eq!(changed, 1);

        let pending: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM notification_queue WHERE status = 'pending'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending, 1);
    }
}
