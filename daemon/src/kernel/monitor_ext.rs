// Monitor extension: telegram poll check + background monitor loop.

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::time::Instant;
use tracing::info;

use super::monitor::{
    check_peer_readiness, run_and_store_cycle, KernelCheckResult, MonitorConfig,
    POLL_INTERVAL, READINESS_INTERVAL, MEMORY_MAINTENANCE_INTERVAL,
};

/// Check whether the Telegram long-poll background task is still alive.
pub fn check_telegram_poll_alive(handle: &tokio::task::JoinHandle<()>) -> KernelCheckResult {
    if handle.is_finished() {
        KernelCheckResult::fail("telegram_poll_alive", "Telegram poll task has exited")
    } else {
        KernelCheckResult::pass("telegram_poll_alive")
    }
}

/// Spawn background monitor loop (30s poll, 5min readiness check).
pub fn spawn_monitor_loop(pool: Pool<SqliteConnectionManager>, config: MonitorConfig) {
    tokio::spawn(async move {
        info!("jarvis.monitor: started (poll every {}s)", POLL_INTERVAL.as_secs());
        let mut last_readiness_check: Option<Instant> = None;
        let mut last_memory_maintenance: Option<Instant> = None;
        loop { // UNBOUNDED: event loop
            tokio::time::sleep(POLL_INTERVAL).await;
            run_and_store_cycle(&pool, &config).await;
            let should_check = last_readiness_check
                .map(|t| t.elapsed() >= READINESS_INTERVAL).unwrap_or(true);
            if should_check && !config.peer_urls.is_empty() {
                info!("jarvis.monitor: running node readiness check on {} peers", config.peer_urls.len());
                for peer_url in &config.peer_urls { check_peer_readiness(&pool, peer_url).await; }
                last_readiness_check = Some(Instant::now());
            }
            let should_maintain_memory = last_memory_maintenance
                .map(|t| t.elapsed() >= MEMORY_MAINTENANCE_INTERVAL)
                .unwrap_or(true);
            if should_maintain_memory {
                let summaries = crate::server::api_memory_mgmt_gc::garbage_collect_all_projects();
                let changed = summaries
                    .iter()
                    .map(|(_, result)| result.total_changed())
                    .sum::<usize>();
                info!(
                    projects = summaries.len(),
                    changed,
                    "jarvis.monitor: completed automatic memory maintenance"
                );
                last_memory_maintenance = Some(Instant::now());
            }
        }
    });
}
