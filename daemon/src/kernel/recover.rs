// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Deterministic recovery chain for the kernel monitor.
// Actions are selected by if/else rules — NOT by LLM inference.
// WARN threshold: >= 3 consecutive cycles (≈90 s at 30 s/cycle).

use std::fmt;
use std::process::Command;
use tracing::{info, warn};

// ----- Public types ----------------------------------------------------------

/// Severity level passed into the recovery chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Ok,
    Warn,
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Ok => write!(f, "OK"),
            Severity::Warn => write!(f, "WARN"),
            Severity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Notification channel selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyChannel {
    Ntfy,
    /// Audio notification (W2 — stub, logs only).
    Local,
    /// Telegram bot (W3 — stub, logs only).
    Telegram,
}

/// Runtime configuration resolved from env vars.
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// ntfy.sh topic name (env: KERNEL_NTFY_TOPIC, default: "convergio").
    pub ntfy_topic: String,
    /// Active notification channels (env: KERNEL_NOTIFY_CHANNELS).
    pub channels: Vec<NotifyChannel>,
    /// When true, external commands (checkpoint/reap/SSH/ntfy POST) are skipped.
    /// Enabled automatically in tests.
    pub dry_run: bool,
}

impl RecoveryConfig {
    /// Build from environment variables; safe defaults when vars are absent.
    pub fn from_env() -> Self {
        let ntfy_topic =
            std::env::var("KERNEL_NTFY_TOPIC").unwrap_or_else(|_| "convergio".to_string());
        let channels = parse_channels(
            &std::env::var("KERNEL_NOTIFY_CHANNELS").unwrap_or_else(|_| "ntfy".to_string()),
        );
        Self { ntfy_topic, channels, dry_run: false }
    }
}

// ----- Public entry points ---------------------------------------------------

/// Deterministic recovery chain.
/// Critical → checkpoint + SSH restart peer + reap + notify.
/// Warn ≥3 cycles (90 s) → log + notify. Warn <3 / Ok → log only.
pub async fn recover(
    severity: Severity,
    consecutive_cycles: Option<u32>,
    cfg: &RecoveryConfig,
) -> Result<(), String> {
    match severity {
        Severity::Critical => {
            info!("kernel.recover: CRITICAL — starting recovery chain");
            run_critical_chain(cfg).await
        }
        Severity::Warn => {
            let cycles = consecutive_cycles.unwrap_or(0);
            if cycles >= 3 {
                warn!("kernel.recover: WARN sustained {} cycles (>=90s) — notifying", cycles);
                log_kernel_event("sustained WARN", cycles);
                communicate("Sustained WARN: kernel health degraded", Severity::Warn, cfg).await;
            } else {
                info!("kernel.recover: WARN cycle {} — logging only", cycles);
                log_kernel_event("WARN", cycles);
            }
            Ok(())
        }
        Severity::Ok => {
            info!("kernel.recover: OK — no action needed");
            Ok(())
        }
    }
}

/// Dispatch a notification to all configured channels.
///
/// ntfy.sh is the only live channel; Local and Telegram are stubs (W2/W3).
pub async fn communicate(message: &str, severity: Severity, cfg: &RecoveryConfig) {
    for channel in &cfg.channels {
        match channel {
            NotifyChannel::Ntfy => {
                if cfg.dry_run {
                    info!(
                        "kernel.recover: [dry_run] ntfy POST skipped — topic={} msg={}",
                        cfg.ntfy_topic, message
                    );
                } else {
                    post_ntfy(&cfg.ntfy_topic, message, &severity).await;
                }
            }
            NotifyChannel::Local => {
                // W2 stub — local audio notification
                info!("kernel.recover: [local] audio stub — severity={severity} msg={message}");
            }
            NotifyChannel::Telegram => {
                // W3 stub — Telegram bot notification
                info!(
                    "kernel.recover: [telegram] stub — severity={severity} msg={message}"
                );
            }
        }
    }
}

// ----- Private helpers -------------------------------------------------------

/// Execute the full CRITICAL recovery chain.
async fn run_critical_chain(cfg: &RecoveryConfig) -> Result<(), String> {
    // Step 1: checkpoint — best-effort, failure does not halt the chain
    if cfg.dry_run {
        info!("kernel.recover: [dry_run] skipping cvg checkpoint save");
    } else {
        run_checkpoint();
    }

    // Step 2: SSH restart peer daemon via mesh
    if cfg.dry_run {
        info!("kernel.recover: [dry_run] skipping SSH peer restart");
    } else {
        ssh_restart_peer();
    }

    // Step 3: reap zombies
    if cfg.dry_run {
        info!("kernel.recover: [dry_run] skipping cvg reap");
    } else {
        run_reap();
    }

    // Step 4: notify
    communicate("CRITICAL: daemon recovery chain triggered", Severity::Critical, cfg).await;

    Ok(())
}

/// Run `cvg checkpoint save <plan_id>` — plan_id sourced from env KERNEL_PLAN_ID.
fn run_checkpoint() {
    let plan_id = std::env::var("KERNEL_PLAN_ID").unwrap_or_else(|_| "0".to_string());
    info!("kernel.recover: running cvg checkpoint save {plan_id}");
    match Command::new("cvg").args(["checkpoint", "save", &plan_id]).output() {
        Ok(o) if o.status.success() => info!("kernel.recover: checkpoint saved"),
        Ok(o) => warn!(
            "kernel.recover: checkpoint failed: {}",
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => warn!("kernel.recover: checkpoint exec error: {e}"),
    }
}

/// Restart the peer daemon over SSH, reusing mesh SSH primitives.
fn ssh_restart_peer() {
    let peer = std::env::var("KERNEL_PEER_SSH").unwrap_or_default();
    if peer.is_empty() {
        warn!("kernel.recover: KERNEL_PEER_SSH not set — skipping SSH peer restart");
        return;
    }
    info!("kernel.recover: SSH restart peer daemon on {peer}");
    // Reuse the handoff_ssh SshClient pattern via a fresh Command invocation.
    // Full mesh SSH integration is in daemon/src/mesh/handoff_ssh.rs.
    match Command::new("ssh")
        .args([&peer, "systemctl", "--user", "restart", "convergio-daemon"])
        .output()
    {
        Ok(o) if o.status.success() => info!("kernel.recover: peer daemon restarted"),
        Ok(o) => warn!(
            "kernel.recover: peer restart failed: {}",
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => warn!("kernel.recover: ssh exec error: {e}"),
    }
}

/// Run `cvg reap` to clean zombie processes.
fn run_reap() {
    info!("kernel.recover: running cvg reap");
    match Command::new("cvg").arg("reap").output() {
        Ok(o) if o.status.success() => info!("kernel.recover: reap complete"),
        Ok(o) => warn!(
            "kernel.recover: reap failed: {}",
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => warn!("kernel.recover: reap exec error: {e}"),
    }
}

/// POST to ntfy.sh with a plain-text message body.
async fn post_ntfy(topic: &str, message: &str, severity: &Severity) {
    let url = format!("https://ntfy.sh/{topic}");
    info!("kernel.recover: posting to ntfy topic={topic}");
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("kernel.recover: failed to build reqwest client: {e}");
            return;
        }
    };
    let body = format!("[{severity}] {message}");
    match client.post(&url).body(body).send().await {
        Ok(resp) => info!("kernel.recover: ntfy response status={}", resp.status()),
        Err(e) => warn!("kernel.recover: ntfy POST failed: {e}"),
    }
}

/// Log an event to kernel_events (tracing only; DB write is additive in W2).
fn log_kernel_event(label: &str, cycles: u32) {
    info!("kernel.recover: kernel_event label={label} consecutive_cycles={cycles}");
}

/// Parse "local,telegram,ntfy" → Vec<NotifyChannel>.
fn parse_channels(raw: &str) -> Vec<NotifyChannel> {
    raw.split(',')
        .filter_map(|s| match s.trim() {
            "ntfy" => Some(NotifyChannel::Ntfy),
            "local" => Some(NotifyChannel::Local),
            "telegram" => Some(NotifyChannel::Telegram),
            other => {
                warn!("kernel.recover: unknown channel '{other}' — skipped");
                None
            }
        })
        .collect()
}
