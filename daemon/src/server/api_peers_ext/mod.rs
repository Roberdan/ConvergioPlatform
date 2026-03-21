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
                "CREATE TABLE peer_heartbeats (
                     peer_name TEXT PRIMARY KEY, last_seen REAL,
                     load_json TEXT, capabilities TEXT
                 );
                 CREATE TABLE mesh_sync_stats (
                     peer_name TEXT PRIMARY KEY, avg_latency_ms REAL,
                     last_sync_at TEXT
                 );
                 INSERT INTO peer_heartbeats VALUES
                     ('mac-worker-2', strftime('%s','now'), '{\"cpu\":20}', 'coordinator'),
                     ('linux-worker', strftime('%s','now') - 60, '{\"cpu\":50}', 'worker'),
                     ('mac-worker-1', strftime('%s','now') - 200, '{\"cpu\":30}', 'worker');",
            )
            .expect("schema");
        db
    }

    #[test]
    fn api_peers_coordinator_found() {
        let db = setup_db();
        let peers = query_rows(
            db.connection(),
            "SELECT peer_name FROM peer_heartbeats ORDER BY last_seen DESC",
            [],
        )
        .unwrap();
        let coordinator = peers.iter().find(|p| {
            p.get("peer_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .contains("mac-worker-2")
        });
        assert!(coordinator.is_some());
    }

    #[test]
    fn api_peers_topology_nodes() {
        let db = setup_db();
        let nodes = query_rows(
            db.connection(),
            "SELECT peer_name FROM peer_heartbeats ORDER BY peer_name",
            [],
        )
        .unwrap();
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn api_peers_diagnostics_counts() {
        let db = setup_db();
        let total: i64 = db
            .connection()
            .query_row("SELECT COUNT(*) FROM peer_heartbeats", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 3);
    }
}
