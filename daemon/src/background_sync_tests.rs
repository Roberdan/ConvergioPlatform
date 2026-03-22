use rusqlite::Connection;
use std::sync::{Arc, Mutex, OnceLock};

use super::{resolve_interval_secs, spawn_sync_loop};

/// Serialise all env-var-mutating tests to prevent parallel interference.
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
fn env_lock() -> &'static Mutex<()> {
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

fn setup_db() -> Arc<Mutex<Connection>> {
    let conn = Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch(
        "CREATE TABLE mesh_sync_stats (
            peer_name TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            last_sync TEXT,
            bytes_sent INTEGER DEFAULT 0,
            bytes_received INTEGER DEFAULT 0
        );",
    )
    .expect("setup schema");
    Arc::new(Mutex::new(conn))
}

#[tokio::test]
async fn test_loop_calls_sync_returns_join_handle() {
    // spawn_sync_loop must return a JoinHandle immediately without blocking.
    // With no rows in mesh_sync_stats, each tick is a no-op — we just verify
    // the handle is created and the loop doesn't panic on an empty peer table.
    let db = setup_db();
    let handle = spawn_sync_loop(db, 60);
    // Aborting the background task prevents it running in test teardown.
    handle.abort();
}

#[test]
fn test_interval_configurable_reads_env_var() {
    let _guard = env_lock().lock().expect("env lock");
    std::env::set_var("CONVERGIO_SYNC_INTERVAL_SECS", "5");
    let secs = resolve_interval_secs(None);
    std::env::remove_var("CONVERGIO_SYNC_INTERVAL_SECS");
    assert_eq!(secs, 5, "env var must override default");
}

#[test]
fn test_interval_default_is_thirty() {
    let _guard = env_lock().lock().expect("env lock");
    std::env::remove_var("CONVERGIO_SYNC_INTERVAL_SECS");
    let secs = resolve_interval_secs(None);
    assert_eq!(secs, 30, "default interval must be 30 seconds");
}

#[test]
fn test_interval_arg_overrides_env() {
    let _guard = env_lock().lock().expect("env lock");
    std::env::set_var("CONVERGIO_SYNC_INTERVAL_SECS", "99");
    let secs = resolve_interval_secs(Some(10));
    std::env::remove_var("CONVERGIO_SYNC_INTERVAL_SECS");
    assert_eq!(secs, 10, "explicit arg must take priority over env var");
}

#[test]
fn test_query_active_peers_returns_non_unreachable() {
    use super::query_active_peers;
    let db = setup_db();
    {
        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "INSERT INTO mesh_sync_stats (peer_name, status) VALUES
             ('node-a', 'active'),
             ('node-b', 'unreachable'),
             ('node-c', 'degraded');",
        )
        .expect("seed peers");
    }
    let peers = query_active_peers(&db).expect("query peers");
    assert_eq!(peers.len(), 2);
    assert!(peers.contains(&"node-a".to_string()));
    assert!(peers.contains(&"node-c".to_string()));
    assert!(!peers.contains(&"node-b".to_string()));
}
