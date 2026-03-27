// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Kernel monitor — model-agnostic health checks every 30s.
// Replaces watchdog.rs (Ollama-based). No LLM dep.
// Writes to kernel_events table (migration: state_init_migrations.rs).

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{info, warn};

const POLL_INTERVAL: Duration = Duration::from_secs(30);
const READINESS_INTERVAL: Duration = Duration::from_secs(300); // 5 min — less frequent than 30s cycle
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const STALL_SECS: u64 = 300;      // agents idle >5 min with a task
const RATE_LIMIT_WARN: u64 = 3;   // 429 count in last 5 min
const DISK_WARN_PCT: f64 = 85.0;
const RAM_WARN_PCT: f64 = 80.0;

/// Checks classified as critical if they fail on a peer node.
const CRITICAL_CHECKS: &[&str] = &["db_integrity", "daemon_version", "db_exists"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelCheckResult {
    pub check_name: String,
    pub ok: bool,
    pub details: Option<String>,
}
impl KernelCheckResult {
    pub fn pass(name: &str) -> Self { Self { check_name: name.into(), ok: true, details: None } }
    pub fn fail(name: &str, d: &str) -> Self { Self { check_name: name.into(), ok: false, details: Some(d.into()) } }
}

#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub daemon_url: String,
    pub peer_urls: Vec<String>,
    /// Token limit for compaction risk heuristic (0 = skip).
    pub compaction_token_limit: u64,
}
impl Default for MonitorConfig {
    fn default() -> Self {
        Self { daemon_url: "http://127.0.0.1:8420".into(), peer_urls: vec![], compaction_token_limit: 180_000 }
    }
}

pub async fn check_daemon_reachable(daemon_url: &str) -> KernelCheckResult {
    http_health(daemon_url, "daemon_health").await
}

pub async fn check_mesh_peers(peer_urls: &[String]) -> Vec<KernelCheckResult> {
    let mut out = vec![];
    for p in peer_urls {
        out.push(http_health(p, &format!("peer_health:{p}")).await);
    }
    out
}

async fn http_health(base: &str, name: &str) -> KernelCheckResult {
    let c = Client::builder().timeout(HTTP_TIMEOUT).build().unwrap_or_default();
    match c.get(format!("{base}/api/health")).send().await {
        Ok(r) if r.status().is_success() => KernelCheckResult::pass(name),
        Ok(r) => KernelCheckResult::fail(name, &format!("HTTP {}", r.status())),
        Err(e) => KernelCheckResult::fail(name, &e.to_string()),
    }
}

pub async fn detect_stalled_agents(daemon_url: &str) -> KernelCheckResult {
    let c = Client::builder().timeout(HTTP_TIMEOUT).build().unwrap_or_default();
    match c.get(format!("{daemon_url}/api/ipc/agents")).send().await {
        Err(e) => KernelCheckResult::fail("stalled_agents", &e.to_string()),
        Ok(r) => match r.json::<serde_json::Value>().await {
            Err(e) => KernelCheckResult::fail("stalled_agents", &e.to_string()),
            Ok(j) => {
                let stalled: Vec<_> = j.as_array().unwrap_or(&vec![]).iter()
                    .filter(|a| a["task_id"].is_string() && a["idle_secs"].as_u64().unwrap_or(0) > STALL_SECS)
                    .map(|a| a["id"].as_str().unwrap_or("?").to_string())
                    .collect();
                if stalled.is_empty() { KernelCheckResult::pass("stalled_agents") }
                else { KernelCheckResult::fail("stalled_agents", &format!("stalled: {}", stalled.join(","))) }
            }
        }
    }
}

pub async fn detect_rate_limits(daemon_url: &str) -> KernelCheckResult {
    let c = Client::builder().timeout(HTTP_TIMEOUT).build().unwrap_or_default();
    match c.get(format!("{daemon_url}/api/ipc/route-history")).send().await {
        Err(e) => KernelCheckResult::fail("rate_limits", &e.to_string()),
        Ok(r) => match r.json::<serde_json::Value>().await {
            Err(e) => KernelCheckResult::fail("rate_limits", &e.to_string()),
            Ok(j) => {
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default().as_secs();
                let count = j.as_array().unwrap_or(&vec![]).iter()
                    .filter(|e| e["status"].as_u64() == Some(429) && now.saturating_sub(e["timestamp"].as_u64().unwrap_or(0)) < 300)
                    .count() as u64;
                if count >= RATE_LIMIT_WARN { KernelCheckResult::fail("rate_limits", &format!("{count} 429s in last 5min")) }
                else { KernelCheckResult::pass("rate_limits") }
            }
        }
    }
}

pub fn check_disk_ram() -> Vec<KernelCheckResult> {
    use sysinfo::{Disks, System};
    let mut out = vec![];
    let sys = System::new_all();
    let total = sys.total_memory();
    if total > 0 {
        let pct = sys.used_memory() as f64 / total as f64 * 100.0;
        if pct >= RAM_WARN_PCT { out.push(KernelCheckResult::fail("ram_pressure", &format!("{pct:.1}% RAM used"))) }
        else { out.push(KernelCheckResult::pass("ram_pressure")) }
    }
    for disk in Disks::new_with_refreshed_list().list() {
        let t = disk.total_space();
        if t > 0 {
            let pct = (t - disk.available_space()) as f64 / t as f64 * 100.0;
            let n = format!("disk:{}", disk.mount_point().display());
            if pct >= DISK_WARN_PCT { out.push(KernelCheckResult::fail(&n, &format!("{pct:.1}% used"))) }
            else { out.push(KernelCheckResult::pass(&n)) }
        }
    }
    out
}

pub fn detect_compaction_risk(current_tokens: u64, limit: u64) -> KernelCheckResult {
    if limit == 0 { return KernelCheckResult::pass("compaction_risk"); }
    let pct = current_tokens as f64 / limit as f64 * 100.0;
    if pct >= 85.0 { KernelCheckResult::fail("compaction_risk", &format!("{pct:.1}% context — checkpoint now")) }
    else { KernelCheckResult::pass("compaction_risk") }
}

/// Extract peer name from a base URL (hostname portion, no port).
pub fn peer_name_from_url(url: &str) -> String {
    // Strip scheme, then take everything before the first '/' or end.
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host_port = without_scheme.split('/').next().unwrap_or(without_scheme);
    // Remove port if present.
    host_port.split(':').next().unwrap_or(host_port).to_string()
}

/// Parse a JSON `checks` array from `/api/node/readiness`, store CRITICAL/WARN events.
/// Returns true if any critical check failed.
pub fn classify_readiness_results(
    pool: &Pool<SqliteConnectionManager>,
    peer_name: &str,
    checks: &serde_json::Value,
) -> bool {
    let source = format!("readiness:{peer_name}");
    let mut critical = false;
    if let Some(arr) = checks.as_array() {
        for c in arr {
            let passed = c["passed"].as_bool().unwrap_or(true);
            if !passed {
                let name = c["name"].as_str().unwrap_or("unknown");
                let detail = c["detail"].as_str().unwrap_or("check failed");
                let sev = if CRITICAL_CHECKS.contains(&name) {
                    critical = true;
                    "critical"
                } else {
                    "warn"
                };
                store_kernel_event(pool, &source, &format!("{name}: {detail}"), sev);
                warn!("kernel.monitor [{}] readiness:{} {}: {}", sev, peer_name, name, detail);
            }
        }
    }
    critical
}

/// Call `/api/node/readiness` on a single peer and persist any failed checks.
pub async fn check_peer_readiness(pool: &Pool<SqliteConnectionManager>, peer_url: &str) {
    let peer_name = peer_name_from_url(peer_url);
    let source = format!("readiness:{peer_name}");
    let client = Client::builder().timeout(HTTP_TIMEOUT).build().unwrap_or_default();
    let warn_event = |msg: String| {
        store_kernel_event(pool, &source, &msg, "warn");
        warn!("kernel.monitor {source} {msg}");
    };
    match client.get(format!("{peer_url}/api/node/readiness")).send().await {
        // Network failure — warn only; peer may be temporarily offline.
        Err(e) => warn_event(format!("unreachable: {e}")),
        Ok(r) if !r.status().is_success() => warn_event(format!("HTTP {}", r.status())),
        Ok(r) => match r.json::<serde_json::Value>().await {
            Err(e) => warn_event(format!("parse error: {e}")),
            Ok(body) => { classify_readiness_results(pool, &peer_name, &body["checks"]); }
        },
    }
}

pub fn store_kernel_event(pool: &Pool<SqliteConnectionManager>, source: &str, msg: &str, severity: &str) {
    match pool.get() {
        Err(e) => warn!("kernel.monitor: db conn: {e}"),
        Ok(conn) => { let _ = conn.execute(
            "INSERT INTO kernel_events (severity, source, message, action_taken) VALUES (?1,?2,?3,'none')",
            rusqlite::params![severity, source, msg],
        ).map_err(|e| warn!("kernel.monitor: insert: {e}")); }
    }
}

/// Classify results → kernel_events. Returns true if any CRITICAL.
pub fn classify_and_store(pool: &Pool<SqliteConnectionManager>, results: &[KernelCheckResult]) -> bool {
    let mut critical = false;
    for r in results {
        if !r.ok {
            let msg = r.details.as_deref().unwrap_or("check failed");
            let sev = if r.check_name.starts_with("peer_health") || r.check_name == "daemon_health" {
                critical = true; "critical"
            } else { "warn" };
            store_kernel_event(pool, &r.check_name, msg, sev);
            warn!("kernel.monitor [{}] {}: {}", sev, r.check_name, msg);
        }
    }
    critical
}

/// Scan /tmp for stale .lock files older than `threshold_secs`.
pub fn detect_stale_locks(threshold_secs: u64) -> KernelCheckResult {
    let cutoff = Duration::from_secs(threshold_secs);
    let now = std::time::SystemTime::now();
    let stale: Vec<_> = std::fs::read_dir("/tmp").into_iter().flatten().flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "lock"))
        .filter(|e| std::fs::metadata(e.path()).ok()
            .and_then(|m| m.modified().ok())
            .map(|t| now.duration_since(t).unwrap_or_default() > cutoff)
            .unwrap_or(false))
        .map(|e| e.path().display().to_string())
        .collect();
    if stale.is_empty() { KernelCheckResult::pass("stale_locks") }
    else { KernelCheckResult::fail("stale_locks", &format!("stale: {}", stale.join(", "))) }
}

/// Run one full cycle and persist results (extracted for testability).
pub async fn run_and_store_cycle(pool: &Pool<SqliteConnectionManager>, config: &MonitorConfig) {
    let mut all: Vec<KernelCheckResult> = vec![
        check_daemon_reachable(&config.daemon_url).await,
    ];
    all.extend(check_mesh_peers(&config.peer_urls).await);
    all.push(detect_stalled_agents(&config.daemon_url).await);
    all.push(detect_rate_limits(&config.daemon_url).await);
    all.extend(check_disk_ram());
    all.push(detect_stale_locks(300));
    all.push(detect_compaction_risk(0, config.compaction_token_limit));
    let critical = classify_and_store(pool, &all);
    if critical { info!("kernel.monitor: CRITICAL — communicate stub (wired W2/W3)"); }
}

/// Spawn background monitor loop (30s interval). Pool is Arc-backed inside r2d2.
/// Node readiness is checked every 5 minutes (READINESS_INTERVAL) — less frequent
/// than the main cycle to avoid hammering peer HTTP endpoints.
pub fn spawn_monitor_loop(pool: Pool<SqliteConnectionManager>, config: MonitorConfig) {
    tokio::spawn(async move {
        info!("kernel.monitor: started (poll every {}s)", POLL_INTERVAL.as_secs());
        let mut last_readiness_check: Option<Instant> = None;
        loop { // UNBOUNDED: event loop
            tokio::time::sleep(POLL_INTERVAL).await;
            run_and_store_cycle(&pool, &config).await;

            // Readiness check: run every READINESS_INTERVAL across all mesh peers.
            let should_check = last_readiness_check
                .map(|t| t.elapsed() >= READINESS_INTERVAL)
                .unwrap_or(true); // first iteration: run immediately
            if should_check && !config.peer_urls.is_empty() {
                info!("kernel.monitor: running node readiness check on {} peers", config.peer_urls.len());
                for peer_url in &config.peer_urls {
                    check_peer_readiness(&pool, peer_url).await;
                }
                last_readiness_check = Some(Instant::now());
            }
        }
    });
}
