// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Kernel monitor — replaces watchdog.rs (Ollama-based) with model-agnostic
// health checks. Reuses the core health/stale-lock logic without any LLM dep.
// watchdog.rs is deprecated; this module is the replacement (cvg kernel).

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

/// Single check outcome produced by the kernel monitor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelCheckResult {
    pub check_name: String,
    pub ok: bool,
    pub details: Option<String>,
}

impl KernelCheckResult {
    pub fn pass(name: &str) -> Self {
        Self { check_name: name.to_string(), ok: true, details: None }
    }

    pub fn fail(name: &str, detail: &str) -> Self {
        Self { check_name: name.to_string(), ok: false, details: Some(detail.to_string()) }
    }
}

/// Check whether the daemon HTTP API responds on the given base URL.
pub async fn check_daemon_reachable(daemon_url: &str) -> KernelCheckResult {
    let client = Client::builder().timeout(Duration::from_secs(5)).build().unwrap_or_default();
    let url = format!("{daemon_url}/api/health");
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => KernelCheckResult::pass("daemon_health"),
        Ok(resp) => KernelCheckResult::fail("daemon_health", &format!("HTTP {}", resp.status())),
        Err(e) => KernelCheckResult::fail("daemon_health", &e.to_string()),
    }
}

/// Scan /tmp for stale .lock files older than `threshold_secs`.
/// Extracted from watchdog::check_stale_locks; no Ollama dependency.
pub fn detect_stale_locks(threshold_secs: u64) -> KernelCheckResult {
    let tmp = std::path::Path::new("/tmp");
    let cutoff = Duration::from_secs(threshold_secs);
    let now = std::time::SystemTime::now();
    let mut stale = vec![];
    if let Ok(rd) = std::fs::read_dir(tmp) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "lock") {
                if let Ok(meta) = std::fs::metadata(&path) {
                    if let Ok(modified) = meta.modified() {
                        if now.duration_since(modified).unwrap_or_default() > cutoff {
                            stale.push(path.display().to_string());
                        }
                    }
                }
            }
        }
    }
    if stale.is_empty() {
        KernelCheckResult::pass("stale_locks")
    } else {
        KernelCheckResult::fail("stale_locks", &format!("stale: {}", stale.join(", ")))
    }
}

/// Heuristic orphan worktree check — logs count, passes unless git errors.
pub async fn detect_orphan_worktrees() -> KernelCheckResult {
    match std::process::Command::new("git").args(["worktree", "list", "--porcelain"]).output() {
        Ok(o) if o.status.success() => {
            let count = String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| l.starts_with("worktree ") && l.contains(".claude/worktrees"))
                .count();
            info!("kernel.monitor: {} active worktrees", count);
            KernelCheckResult::pass("orphan_worktrees")
        }
        Ok(_) => KernelCheckResult::pass("orphan_worktrees"),
        Err(e) => KernelCheckResult::fail("orphan_worktrees", &e.to_string()),
    }
}

/// Run a full monitor cycle: daemon reachability + stale locks + orphan worktrees.
/// Replaces watchdog::run_checks without Ollama dependency.
pub async fn run_monitor_cycle(daemon_url: &str, stale_threshold_secs: u64) -> Vec<KernelCheckResult> {
    vec![
        check_daemon_reachable(daemon_url).await,
        detect_stale_locks(stale_threshold_secs),
        detect_orphan_worktrees().await,
    ]
}
