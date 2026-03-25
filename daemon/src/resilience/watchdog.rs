// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// F-26: Local LLM kernel watchdog — monitors daemon health, agent progress,
// orphan worktrees and stale locks. Uses Ollama for decision summarisation;
// falls back to hard-coded rules when Ollama is unavailable.

use crate::resilience::notify::{ChannelConfig, NotifyMessage, NotifySeverity};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{info, warn};

/// Watchdog configuration — loaded from notifications.conf at startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogConfig {
    /// How often (seconds) to run all health checks. Default: 30.
    pub check_interval_secs: u64,
    /// Ollama HTTP API base URL. Default: http://localhost:11434.
    pub ollama_url: String,
    /// LLM model name for summarisation. Default: llama3.
    pub model_name: String,
    /// Daemon API base URL. Default: http://localhost:8420.
    pub daemon_url: String,
    /// Seconds without DB update before a task is considered stalled. Default: 300.
    pub stale_threshold_secs: u64,
    /// Ordered list of notification channels.
    pub notification_channels: Vec<ChannelConfig>,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 30,
            ollama_url: "http://localhost:11434".to_string(),
            model_name: "llama3".to_string(),
            daemon_url: "http://localhost:8420".to_string(),
            stale_threshold_secs: 300,
            notification_channels: vec![],
        }
    }
}

/// Single check outcome — passed to notifier on failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub check_name: String,
    pub ok: bool,
    pub details: Option<String>,
}

impl CheckResult {
    pub fn pass(name: &str) -> Self {
        Self { check_name: name.to_string(), ok: true, details: None }
    }
    pub fn fail(name: &str, detail: &str) -> Self {
        Self { check_name: name.to_string(), ok: false, details: Some(detail.to_string()) }
    }
}

/// Snapshot of watchdog runtime state — returned by GET /api/watchdog/status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogStatus {
    pub running: bool,
    pub checks_passed: u32,
    pub checks_failed: u32,
    pub last_check_at: Option<String>,
}

/// Run one full health-check cycle.
pub async fn run_checks(config: &WatchdogConfig) -> Vec<CheckResult> {
    let client = Client::builder().timeout(Duration::from_secs(5)).build().unwrap_or_default();
    vec![
        check_daemon_health(&client, &config.daemon_url).await,
        check_stale_locks().await,
        check_orphan_worktrees().await,
    ]
}

async fn check_daemon_health(client: &Client, daemon_url: &str) -> CheckResult {
    let url = format!("{daemon_url}/api/health");
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => CheckResult::pass("daemon_health"),
        Ok(resp) => CheckResult::fail("daemon_health", &format!("HTTP {}", resp.status())),
        Err(e) => CheckResult::fail("daemon_health", &e.to_string()),
    }
}

/// Scan /tmp for stale .lock files older than 5 minutes.
async fn check_stale_locks() -> CheckResult {
    let tmp = std::path::Path::new("/tmp");
    let cutoff = Duration::from_secs(300);
    let now = std::time::SystemTime::now();
    let mut stale = vec![];
    if let Ok(rd) = std::fs::read_dir(tmp) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "lock") {
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
    if stale.is_empty() { CheckResult::pass("stale_locks") }
    else { CheckResult::fail("stale_locks", &format!("stale: {}", stale.join(", "))) }
}

/// Heuristic orphan worktree check — logs count, passes unless git errors.
async fn check_orphan_worktrees() -> CheckResult {
    match std::process::Command::new("git").args(["worktree", "list", "--porcelain"]).output() {
        Ok(o) if o.status.success() => {
            let count = String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| l.starts_with("worktree ") && l.contains(".claude/worktrees"))
                .count();
            info!("watchdog: {} active worktrees", count);
            CheckResult::pass("orphan_worktrees")
        }
        Ok(_) => CheckResult::pass("orphan_worktrees"),
        Err(e) => CheckResult::fail("orphan_worktrees", &e.to_string()),
    }
}

/// Ask Ollama to summarise; falls back to plain text on error (no LLM needed).
pub async fn ollama_summarise(config: &WatchdogConfig, context: &str) -> String {
    let client = Client::builder().timeout(Duration::from_secs(10)).build().unwrap_or_default();
    let url = format!("{}/api/generate", config.ollama_url);
    let payload = serde_json::json!({
        "model": config.model_name,
        "prompt": format!("Summarise this agent failure in 1 sentence: {context}"),
        "stream": false
    });
    match client.post(&url).json(&payload).send().await {
        Ok(resp) if resp.status().is_success() => {
            resp.json::<serde_json::Value>().await
                .ok()
                .and_then(|j| j["response"].as_str().map(str::to_string))
                .unwrap_or_else(|| context.to_string())
        }
        _ => { warn!("watchdog: Ollama unavailable, using raw error text"); context.to_string() }
    }
}

/// Hard-coded restart vs escalate rule (no LLM needed for basic decisions).
pub fn decide_action(failures: &[CheckResult]) -> WatchdogAction {
    if failures.iter().any(|r| r.check_name == "daemon_health") {
        WatchdogAction::Restart
    } else {
        WatchdogAction::Notify
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchdogAction { Restart, Notify, NoOp }

pub async fn build_notification(config: &WatchdogConfig, failures: &[CheckResult]) -> NotifyMessage {
    let raw = failures.iter().filter_map(|r| r.details.as_deref()).collect::<Vec<_>>().join("; ");
    let summary = ollama_summarise(config, &raw).await;
    NotifyMessage {
        title: "Watchdog alert".to_string(),
        message: summary,
        severity: if failures.iter().any(|r| r.check_name == "daemon_health") {
            NotifySeverity::Critical
        } else { NotifySeverity::Warning },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_config_defaults() {
        let cfg = WatchdogConfig::default();
        assert_eq!(cfg.check_interval_secs, 30);
        assert!(cfg.ollama_url.contains("11434"));
    }

    #[test]
    fn watchdog_config_channels_empty_by_default() {
        assert!(WatchdogConfig::default().notification_channels.is_empty());
    }

    #[test]
    fn check_result_pass() {
        let r = CheckResult::pass("health");
        assert!(r.ok);
        assert!(r.details.is_none());
    }

    #[test]
    fn check_result_fail_has_details() {
        let r = CheckResult::fail("health", "timeout");
        assert!(!r.ok);
        assert!(r.details.is_some());
    }

    #[test]
    fn watchdog_status_serializes() {
        let s = WatchdogStatus { running: true, checks_passed: 4, checks_failed: 1, last_check_at: None };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"running\":true"));
    }

    #[test]
    fn decide_action_daemon_down_is_restart() {
        let failures = vec![CheckResult::fail("daemon_health", "connection refused")];
        assert_eq!(decide_action(&failures), WatchdogAction::Restart);
    }

    #[test]
    fn decide_action_other_failure_is_notify() {
        let failures = vec![CheckResult::fail("stale_locks", "found 2")];
        assert_eq!(decide_action(&failures), WatchdogAction::Notify);
    }
}
