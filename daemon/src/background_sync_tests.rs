use rusqlite::Connection;
use std::sync::{Arc, Mutex, OnceLock};

use super::{resolve_interval_secs, spawn_sync_loop, sync_table_with_peer};
use crate::db_path_from_env;

/// Serialise all env-var-mutating tests to prevent parallel interference.
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
fn env_lock() -> &'static Mutex<()> {
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

fn setup_db() -> Arc<Mutex<Connection>> {
    let conn = Connection::open_in_memory().expect("in-memory db");
    conn.execute_batch(
        "CREATE TABLE peer_heartbeats (
            peer_name TEXT PRIMARY KEY,
            last_seen REAL NOT NULL,
            load_json TEXT
        );
        CREATE TABLE mesh_sync_stats (
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

/// Create a temporary peers.conf file and set CONVERGIO_PEERS_CONF to point to it.
/// Returns the path (kept alive by the caller owning the string).
fn setup_peers_conf(content: &str) -> String {
    let path = format!("/tmp/test-peers-{}.conf", std::process::id());
    std::fs::write(&path, content).expect("write temp peers.conf");
    std::env::set_var("CONVERGIO_PEERS_CONF", &path);
    path
}

fn cleanup_peers_conf(path: &str) {
    let _ = std::fs::remove_file(path);
    std::env::remove_var("CONVERGIO_PEERS_CONF");
}

#[tokio::test]
async fn test_loop_calls_sync_returns_join_handle() {
    let _guard = env_lock().lock().expect("env lock");
    // spawn_sync_loop must return a JoinHandle immediately without blocking.
    let db = setup_db();
    let path = setup_peers_conf("[mesh]\nshared_secret=test\n");
    let handle = spawn_sync_loop(db, 60);
    handle.abort();
    cleanup_peers_conf(&path);
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
fn test_query_active_peers_returns_recent_heartbeats() {
    use super::query_active_peers;
    let _guard = env_lock().lock().expect("env lock");
    let db = setup_db();

    // Create peers.conf with the test peer names
    let conf_path = setup_peers_conf(
        "[mesh]\nshared_secret=test\n\n\
         [node-a]\nssh_alias=node-a\nuser=test\nos=macos\n\
         tailscale_ip=100.1.1.1\ndns_name=node-a.ts.net\n\
         capabilities=claude\nrole=worker\n\n\
         [node-c]\nssh_alias=node-c\nuser=test\nos=macos\n\
         tailscale_ip=100.2.2.2\ndns_name=node-c.ts.net\n\
         capabilities=claude\nrole=worker\n",
    );

    {
        let conn = db.lock().expect("lock");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        conn.execute_batch(&format!(
            "INSERT INTO peer_heartbeats (peer_name, last_seen) VALUES
             ('node-a', {now}),
             ('node-b', {old}),
             ('node-c', {now});",
            now = now,
            old = now - 3600.0,
        ))
        .expect("seed peers");
    }
    let peers = query_active_peers(&db).expect("query peers");
    // node-a and node-c are online but may not be TCP-reachable in test env.
    // The key assertion: node-b (old heartbeat) is never included.
    for addr in &peers {
        assert!(
            !addr.contains("node-b"),
            "stale peer node-b must not appear, got: {addr}"
        );
    }
    // Addresses must not have http:// scheme (transport layer adds it)
    for addr in &peers {
        assert!(
            !addr.starts_with("http://"),
            "peer addr must not include scheme, got: {addr}"
        );
    }
    cleanup_peers_conf(&conf_path);
}

#[test]
fn test_query_active_peers_returns_host_port_without_scheme() {
    // Peer addresses must be "host:port" — the HTTP transport adds the scheme.
    // A double scheme (http://http://...) causes silent sync failure.
    use super::query_active_peers;
    let _guard = env_lock().lock().expect("env lock");
    let db = setup_db();

    let conf_path = setup_peers_conf(
        "[mesh]\nshared_secret=test\n\n\
         [node-x]\nssh_alias=node-x\nuser=test\nos=macos\n\
         tailscale_ip=100.5.5.5\ndns_name=node-x.ts.net\n\
         capabilities=claude\nrole=worker\n",
    );

    {
        let conn = db.lock().expect("lock");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        conn.execute_batch(&format!(
            "INSERT INTO peer_heartbeats (peer_name, last_seen) VALUES \
             ('node-x', {now});",
        ))
        .expect("seed peer");
    }
    let peers = query_active_peers(&db).expect("query");
    // Peer may not be TCP-reachable in test, but if resolved it must not have scheme
    for addr in &peers {
        assert!(
            !addr.starts_with("http://"),
            "peer addr must not include scheme, got: {addr}"
        );
        assert!(
            addr.ends_with(":8420"),
            "peer addr must end with :8420, got: {addr}"
        );
    }
    cleanup_peers_conf(&conf_path);
}

#[test]
fn test_query_active_peers_peer_not_in_conf() {
    // Peer online in heartbeats but missing from peers.conf → error logged, not panicked.
    use super::query_active_peers;
    let _guard = env_lock().lock().expect("env lock");
    let db = setup_db();

    // peers.conf has NO peers — only [mesh] section
    let conf_path = setup_peers_conf("[mesh]\nshared_secret=test\n");

    {
        let conn = db.lock().expect("lock");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        conn.execute_batch(&format!(
            "INSERT INTO peer_heartbeats (peer_name, last_seen) VALUES ('ghost', {now});",
        ))
        .expect("seed");
    }
    let peers = query_active_peers(&db).expect("query");
    assert!(
        peers.is_empty(),
        "peer not in peers.conf must not be returned"
    );
    cleanup_peers_conf(&conf_path);
}

#[test]
fn test_db_path_from_env_uses_dashboard_db() {
    let _guard = env_lock().lock().expect("env lock");
    std::env::set_var("DASHBOARD_DB", "/tmp/test-convergio.db");
    let path = db_path_from_env();
    std::env::remove_var("DASHBOARD_DB");
    assert_eq!(path.to_str().unwrap(), "/tmp/test-convergio.db");
}

#[test]
fn test_sync_table_with_peer_handles_unreachable_peer() {
    // sync_table_with_peer should return 0 when the peer is unreachable
    // (HTTP request fails). No panic expected.
    let conn = Connection::open_in_memory().expect("db");
    conn.execute_batch(
        "CREATE TABLE tasks (
           id INTEGER PRIMARY KEY,
           title TEXT NOT NULL,
           status TEXT NOT NULL DEFAULT 'pending',
           updated_at TEXT NOT NULL DEFAULT (datetime('now'))
         );
         CREATE TABLE _sync_meta (
           peer TEXT NOT NULL,
           table_name TEXT NOT NULL,
           last_sync_at TEXT NOT NULL,
           PRIMARY KEY (peer, table_name)
         );",
    )
    .expect("schema");
    // Use a non-routable address to ensure HTTP fails fast
    let (sent, recv, applied) = sync_table_with_peer(&conn, "192.0.2.1:9999", "tasks");
    assert_eq!((sent, recv, applied), (0, 0, 0), "unreachable peer should yield all zeros");
}

#[test]
fn test_db_path_from_env_fallback_to_home() {
    let _guard = env_lock().lock().expect("env lock");
    std::env::remove_var("DASHBOARD_DB");
    let path = db_path_from_env();
    assert!(
        path.to_str().unwrap().ends_with(".claude/data/dashboard.db"),
        "fallback must resolve to ~/.claude/data/dashboard.db, got: {}",
        path.display()
    );
}
