// Agent reaper — garbage collector for stale agents, dead delegations, orphan sessions.
// Runs as a periodic background task alongside Ali.
// Also cleans up temp files on mesh peers from completed plan delegations.

use std::path::Path;

const STALE_AGENT_MINUTES: i64 = 30;
const STALE_DELEGATION_HOURS: i64 = 24;
const DAEMON_BASE: &str = "http://localhost:8420";

/// Reap stale agents (not seen for 30 min), dead delegations, orphan sessions.
/// Returns (agents_reaped, delegations_cleaned, sessions_cleaned).
pub fn reap(db_path: &Path) -> Result<(usize, usize, usize), Box<dyn std::error::Error>> {
    let conn = rusqlite::Connection::open(db_path)?;

    // 1. Stale agents: registered but no heartbeat in 30 min
    let agents_reaped = conn.execute(
        "DELETE FROM ipc_agents WHERE last_seen < datetime('now', ?1)",
        rusqlite::params![format!("-{STALE_AGENT_MINUTES} minutes")],
    )? + conn.execute(
        "DELETE FROM agent_activity WHERE status='running' \
         AND started_at < datetime('now', '-2 hours')",
        [],
    ).unwrap_or(0);

    // 2. Dead delegations: plans assigned to peers but no progress in 24h
    let delegations_cleaned = conn.execute(
        "UPDATE plans SET execution_host=NULL \
         WHERE execution_host IS NOT NULL \
         AND status='doing' \
         AND updated_at < datetime('now', ?1)",
        rusqlite::params![format!("-{STALE_DELEGATION_HOURS} hours")],
    )?;

    // 3. Orphan IPC messages: older than 7 days
    let sessions_cleaned = conn.execute(
        "DELETE FROM ipc_messages WHERE created_at < datetime('now', '-7 days')",
        [],
    ).unwrap_or(0);

    // 4. Expired file locks
    let _ = conn.execute(
        "DELETE FROM ipc_file_locks WHERE expires_at IS NOT NULL AND expires_at < datetime('now')",
        [],
    );

    if agents_reaped > 0 || delegations_cleaned > 0 || sessions_cleaned > 0 {
        tracing::info!(
            "reaper: reaped {agents_reaped} agents, {delegations_cleaned} delegations, {sessions_cleaned} messages"
        );
    }

    Ok((agents_reaped, delegations_cleaned, sessions_cleaned))
}

/// Clean up temp files on mesh peers: /tmp/convergio-plan-*.md and done scripts.
/// Only removes files older than 24 hours to avoid interfering with active plans.
async fn reap_peer_tmp_files() -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_default();

    let resp = client.get(format!("{DAEMON_BASE}/api/mesh/status")).send().await?;
    let body: serde_json::Value = resp.json().await?;
    let peers = body.get("peers").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let mut cleaned = 0usize;
    for peer in &peers {
        let name = peer.get("peer_name").and_then(|v| v.as_str()).unwrap_or("");
        let online = peer.get("is_online").and_then(|v| v.as_bool()).unwrap_or(false);
        if !online || name.is_empty() {
            continue;
        }

        // Remove temp files older than 24h + kill dead tmux plan sessions
        let cmd = "find /tmp -name 'convergio-plan-*' -mmin +1440 -delete 2>/dev/null; \
                   for s in $(tmux list-sessions -F '#{session_name}' 2>/dev/null | grep '^plan-'); do \
                     if tmux list-panes -t \"$s\" -F '#{pane_dead}' 2>/dev/null | grep -q 1; then \
                       tmux kill-session -t \"$s\" 2>/dev/null; \
                     fi; \
                   done; true";

        let result = client
            .post(format!("{DAEMON_BASE}/api/mesh/exec"))
            .json(&serde_json::json!({
                "peer": name,
                "command": "bash",
                "args": ["-c", cmd],
                "timeout_secs": 15,
            }))
            .send()
            .await;

        if result.is_ok() {
            cleaned += 1;
        }
    }

    if cleaned > 0 {
        tracing::info!("reaper: cleaned tmp files on {cleaned} peers");
    }
    Ok(cleaned)
}

/// Spawn the reaper as a periodic background task (every 5 min).
pub fn spawn_reaper(db_path: std::path::PathBuf) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            // Local cleanup (sync DB operations)
            match reap(&db_path) {
                Ok((a, d, s)) => {
                    if a + d + s > 0 {
                        tracing::info!("reaper cycle: agents={a} delegations={d} messages={s}");
                    }
                }
                Err(e) => tracing::warn!("reaper error: {e}"),
            }
            // Remote cleanup (async mesh operations)
            if let Err(e) = reap_peer_tmp_files().await {
                tracing::debug!("reaper: peer cleanup skipped: {e}");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> (tempfile::NamedTempFile, rusqlite::Connection) {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(tmp.path()).unwrap();
        conn.execute_batch(
            "CREATE TABLE ipc_agents (name TEXT PRIMARY KEY, host TEXT, agent_type TEXT, pid INTEGER, metadata TEXT, registered_at TEXT, last_seen TEXT DEFAULT (datetime('now')));
             CREATE TABLE ipc_messages (id TEXT PRIMARY KEY, from_agent TEXT, to_agent TEXT, channel TEXT, content TEXT, msg_type TEXT, created_at TEXT DEFAULT (datetime('now')), read_at TEXT, priority INTEGER DEFAULT 0);
             CREATE TABLE ipc_file_locks (file_path TEXT PRIMARY KEY, locked_by TEXT, lock_type TEXT, acquired_at TEXT, expires_at TEXT);
             CREATE TABLE agent_activity (id INTEGER PRIMARY KEY, status TEXT, started_at TEXT);
             CREATE TABLE plans (id INTEGER PRIMARY KEY, status TEXT, execution_host TEXT, updated_at TEXT);",
        ).unwrap();
        (tmp, conn)
    }

    #[test]
    fn reap_removes_stale_agents() {
        let (tmp, conn) = setup_db();
        conn.execute(
            "INSERT INTO ipc_agents(name, last_seen) VALUES ('stale', datetime('now', '-2 hours'))",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO ipc_agents(name, last_seen) VALUES ('fresh', datetime('now'))",
            [],
        ).unwrap();

        let (reaped, _, _) = reap(tmp.path()).unwrap();
        assert!(reaped >= 1);

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM ipc_agents", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1); // only 'fresh' remains
    }

    #[test]
    fn reap_clears_dead_delegations() {
        let (tmp, conn) = setup_db();
        conn.execute(
            "INSERT INTO plans(id, status, execution_host, updated_at) VALUES (1, 'doing', 'dead-peer', datetime('now', '-48 hours'))",
            [],
        ).unwrap();

        let (_, cleaned, _) = reap(tmp.path()).unwrap();
        assert_eq!(cleaned, 1);

        let host: Option<String> = conn.query_row(
            "SELECT execution_host FROM plans WHERE id=1", [], |r| r.get(0)
        ).unwrap();
        assert!(host.is_none());
    }

    #[test]
    fn reap_removes_expired_locks() {
        let (tmp, conn) = setup_db();
        conn.execute(
            "INSERT INTO ipc_file_locks(file_path, locked_by, expires_at) VALUES ('test.rs', 'agent1', datetime('now', '-1 hour'))",
            [],
        ).unwrap();

        let _ = reap(tmp.path()).unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM ipc_file_locks", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn reap_peer_tmp_files_handles_no_daemon() {
        // Should return error when daemon is not running, not panic
        let result = reap_peer_tmp_files().await;
        assert!(result.is_err(), "should error when daemon is not running");
    }
}
