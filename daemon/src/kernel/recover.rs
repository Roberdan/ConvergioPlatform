// Deterministic recovery chain. Actions by if/else rules, NOT LLM.
// WARN threshold: >= 3 consecutive cycles (≈90s).

use std::fmt;
use std::path::PathBuf;
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
    /// DB path for active-node resolution in the Local audio channel.
    /// Defaults to ~/.claude/data/dashboard.db.
    pub db_path: Option<PathBuf>,
}

impl RecoveryConfig {
    /// Build from environment variables; safe defaults when vars are absent.
    pub fn from_env() -> Self {
        let ntfy_topic =
            std::env::var("KERNEL_NTFY_TOPIC").unwrap_or_else(|_| "convergio".to_string());
        let channels = parse_channels(
            &std::env::var("KERNEL_NOTIFY_CHANNELS").unwrap_or_else(|_| "ntfy".to_string()),
        );
        let db_path = match std::env::var("DASHBOARD_DB") {
            Ok(v) => Some(PathBuf::from(v)),
            Err(_) => match std::env::var("HOME") {
                Ok(h) => Some(PathBuf::from(h).join(".claude/data/dashboard.db")),
                Err(_) => None,
            },
        };
        Self { ntfy_topic, channels, dry_run: false, db_path }
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
            info!("jarvis.recover: CRITICAL — starting recovery chain");
            run_critical_chain(cfg).await
        }
        Severity::Warn => {
            let cycles = consecutive_cycles.unwrap_or(0);
            if cycles >= 3 {
                warn!("jarvis.recover: WARN sustained {} cycles (>=90s) — notifying", cycles);
                log_kernel_event("sustained WARN", cycles);
                communicate("Sustained WARN: kernel health degraded", Severity::Warn, cfg).await;
            } else {
                info!("jarvis.recover: WARN cycle {} — logging only", cycles);
                log_kernel_event("WARN", cycles);
            }
            Ok(())
        }
        Severity::Ok => {
            info!("jarvis.recover: OK — no action needed");
            Ok(())
        }
    }
}

/// Dispatch a notification to all configured channels.
///
/// Ntfy: HTTP POST to ntfy.sh.
/// Local: synthesise message via TTS → route to active mesh node via audio::play_on_active_node.
/// Telegram: stub (W3).
pub async fn communicate(message: &str, severity: Severity, cfg: &RecoveryConfig) {
    for channel in &cfg.channels {
        match channel {
            NotifyChannel::Ntfy => {
                if cfg.dry_run {
                    info!(
                        "jarvis.recover: [dry_run] ntfy POST skipped — topic={} msg={}",
                        cfg.ntfy_topic, message
                    );
                } else {
                    post_ntfy(&cfg.ntfy_topic, message, &severity).await;
                }
            }
            NotifyChannel::Local => {
                if cfg.dry_run {
                    info!(
                        "jarvis.recover: [dry_run] audio skipped — severity={severity} msg={message}"
                    );
                } else {
                    // TTS speak + play on active node
                    let mut tts = super::tts::TtsEngine::new();
                    if let Ok(audio) = tts.speak(message, "it-IT") {
                        super::audio::play_local(&audio).await;
                    }
                }
            }
            NotifyChannel::Telegram => {
                // Why: replaced W3 stub with real Telegram send_text (Plan 729 T3-01).
                super::telegram::communicate(message, severity.clone(), cfg.dry_run).await
                    .unwrap_or_else(|e| warn!("jarvis.recover: telegram communicate failed: {e}"));
            }
        }
    }
}

/// Execute the full CRITICAL recovery chain.
async fn run_critical_chain(cfg: &RecoveryConfig) -> Result<(), String> {
    if cfg.dry_run {
        info!("jarvis.recover: [dry_run] skipping checkpoint+restart+reap");
    } else {
        run_checkpoint();
        ssh_restart_peer();
        run_reap();
    }
    communicate("CRITICAL: daemon recovery chain triggered", Severity::Critical, cfg).await;

    Ok(())
}

/// Run `cvg checkpoint save <plan_id>` — plan_id sourced from env KERNEL_PLAN_ID.
fn run_checkpoint() {
    let plan_id = std::env::var("KERNEL_PLAN_ID").unwrap_or_else(|_| "0".to_string());
    info!("jarvis.recover: running cvg checkpoint save {plan_id}");
    match Command::new("cvg").args(["checkpoint", "save", &plan_id]).output() {
        Ok(o) if o.status.success() => info!("jarvis.recover: checkpoint saved"),
        Ok(o) => warn!(
            "jarvis.recover: checkpoint failed: {}",
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => warn!("jarvis.recover: checkpoint exec error: {e}"),
    }
}

/// Restart the peer daemon over SSH, reusing mesh SSH primitives.
fn ssh_restart_peer() {
    let peer = std::env::var("KERNEL_PEER_SSH").unwrap_or_default();
    if peer.is_empty() {
        warn!("jarvis.recover: KERNEL_PEER_SSH not set — skipping SSH peer restart");
        return;
    }
    info!("jarvis.recover: SSH restart peer daemon on {peer}");
    // Reuse the handoff_ssh SshClient pattern via a fresh Command invocation.
    // Full mesh SSH integration is in daemon/src/mesh/handoff_ssh.rs.
    match Command::new("ssh")
        .args([&peer, "systemctl", "--user", "restart", "convergio-daemon"])
        .output()
    {
        Ok(o) if o.status.success() => info!("jarvis.recover: peer daemon restarted"),
        Ok(o) => warn!(
            "jarvis.recover: peer restart failed: {}",
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => warn!("jarvis.recover: ssh exec error: {e}"),
    }
}

/// Run `cvg reap` to clean zombie processes.
fn run_reap() {
    info!("jarvis.recover: running cvg reap");
    match Command::new("cvg").arg("reap").output() {
        Ok(o) if o.status.success() => info!("jarvis.recover: reap complete"),
        Ok(o) => warn!(
            "jarvis.recover: reap failed: {}",
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => warn!("jarvis.recover: reap exec error: {e}"),
    }
}

/// POST to ntfy.sh with a plain-text message body.
async fn post_ntfy(topic: &str, message: &str, severity: &Severity) {
    let url = format!("https://ntfy.sh/{topic}");
    info!("jarvis.recover: posting to ntfy topic={topic}");
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!("jarvis.recover: failed to build reqwest client: {e}");
            return;
        }
    };
    let body = format!("[{severity}] {message}");
    match client.post(&url).body(body).send().await {
        Ok(resp) => info!("jarvis.recover: ntfy response status={}", resp.status()),
        Err(e) => warn!("jarvis.recover: ntfy POST failed: {e}"),
    }
}

/// Log an event to kernel_events (tracing only; DB write is additive in W2).
fn log_kernel_event(label: &str, cycles: u32) {
    info!("jarvis.recover: kernel_event label={label} consecutive_cycles={cycles}");
}
/// Parse "local,telegram,ntfy" → Vec<NotifyChannel>.
fn parse_channels(raw: &str) -> Vec<NotifyChannel> {
    raw.split(',')
        .filter_map(|s| match s.trim() {
            "ntfy" => Some(NotifyChannel::Ntfy),
            "local" => Some(NotifyChannel::Local),
            "telegram" => Some(NotifyChannel::Telegram),
            other => {
                warn!("jarvis.recover: unknown channel '{other}' — skipped");
                None
            }
        })
        .collect()
}
