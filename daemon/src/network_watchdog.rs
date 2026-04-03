//! Background network connectivity monitor.
//! Detects drops/recoveries and logs which agents were affected.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

static NETWORK_UP: AtomicBool = AtomicBool::new(true);

/// Returns current network status (non-blocking).
pub fn is_network_up() -> bool {
    NETWORK_UP.load(Ordering::Relaxed)
}

/// Spawn as a tokio background task at daemon boot.
/// Checks connectivity every 30s and logs agent impact on drop/recovery.
pub async fn run_watchdog(pool: Pool<SqliteConnectionManager>) {
    let mut was_up = true;
    let mut lost_agents: Vec<String> = Vec::new();

    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let up = check_connectivity().await;
        NETWORK_UP.store(up, Ordering::Relaxed);

        if was_up && !up {
            tracing::warn!("network_watchdog: connectivity lost");
            lost_agents = get_active_agents(&pool);
            tracing::warn!(
                "network_watchdog: {} agents were active: {:?}",
                lost_agents.len(),
                lost_agents
            );
        }

        if !was_up && up {
            tracing::info!("network_watchdog: connectivity restored");
            log_recovery_impact(&pool, &mut lost_agents);
        }

        was_up = up;
    }
}

fn log_recovery_impact(
    pool: &Pool<SqliteConnectionManager>,
    lost_agents: &mut Vec<String>,
) {
    if lost_agents.is_empty() {
        return;
    }
    let current = get_active_agents(pool);
    let missing: Vec<_> = lost_agents
        .iter()
        .filter(|a| !current.contains(a))
        .cloned()
        .collect();
    if !missing.is_empty() {
        tracing::warn!(
            "network_watchdog: {} agents lost during outage: {:?}",
            missing.len(),
            missing
        );
        respawn_copilot_sessions(&missing);
    }
    lost_agents.clear();
}

/// Attempt to resume dead Copilot CLI sessions.
/// Scans ~/.copilot/session-state/ for recent sessions and relaunches them.
fn respawn_copilot_sessions(missing_agents: &[String]) {
    let copilot_agents: Vec<_> = missing_agents
        .iter()
        .filter(|a| a.contains("copilot"))
        .collect();
    if copilot_agents.is_empty() {
        return;
    }

    // Find recent session IDs from session-state directory
    let session_dir = dirs::home_dir()
        .map(|h| h.join(".copilot/session-state"))
        .unwrap_or_default();
    if !session_dir.exists() {
        tracing::debug!("network_watchdog: no session-state dir found");
        return;
    }

    // Get most recent sessions (by modification time)
    let mut sessions: Vec<_> = match std::fs::read_dir(&session_dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter_map(|e| {
                let mtime = e.metadata().ok()?.modified().ok()?;
                Some((e.file_name().to_string_lossy().to_string(), mtime))
            })
            .collect(),
        Err(_) => return,
    };
    sessions.sort_by(|a, b| b.1.cmp(&a.1));

    // Resume up to N most recent sessions (one per lost copilot agent)
    let to_resume = sessions.iter().take(copilot_agents.len());
    for (session_id, _) in to_resume {
        tracing::info!(
            "network_watchdog: resuming copilot session {session_id}"
        );
        let resume_arg = format!("--resume={session_id}");
        match std::process::Command::new("copilot")
            .args([&resume_arg, "--allow-all-tools"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => tracing::info!(
                "network_watchdog: copilot resumed pid={}", child.id()
            ),
            Err(e) => tracing::warn!(
                "network_watchdog: copilot resume failed: {e}"
            ),
        }
    }
}

async fn check_connectivity() -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default();
    client
        .get("https://api.github.com/zen")
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn get_active_agents(pool: &Pool<SqliteConnectionManager>) -> Vec<String> {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("network_watchdog: pool error: {e}");
            return Vec::new();
        }
    };
    let mut stmt = match conn.prepare(
        "SELECT name FROM ipc_agents \
         WHERE last_seen >= datetime('now', '-10 minutes')",
    ) {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!(
                "network_watchdog: ipc_agents query failed (table may not exist): {e}"
            );
            return Vec::new();
        }
    };
    stmt.query_map([], |row| row.get::<_, String>(0))
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
}
