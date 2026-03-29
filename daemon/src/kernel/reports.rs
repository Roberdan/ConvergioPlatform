// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Proactive Telegram reports: scheduled (daily 09:00 CET, weekly Sunday 20:00 CET)
// and event-driven (plan completion, wave merge).
// Config: KERNEL_REPORTS=daily,critical,completion,weekly (comma-separated; empty = all).
// Respects KERNEL_QUIET_HOURS via QuietHoursConfig from kernel/telegram.rs.

use crate::kernel::telegram::{send_text, QuietHoursConfig};
use chrono::{Datelike, Timelike};
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

// ----- Public types ----------------------------------------------------------

/// Which report categories are enabled (env: KERNEL_REPORTS).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportConfig {
    pub daily: bool,
    pub critical: bool,
    pub completion: bool,
    pub weekly: bool,
}

/// Metrics for the daily morning report.
#[derive(Debug, Clone)]
pub struct DailyMetrics {
    pub active_plans: u32,
    pub completed_tasks: u32,
    pub cost_yesterday_usd: f64,
    pub agents_used: u32,
    pub critical_events: u32,
}

/// Info for the wave-merge event report.
#[derive(Debug, Clone)]
pub struct WaveInfo {
    pub wave_id: String,
    pub plan_name: String,
    pub next_wave: Option<String>,
}

// ----- Config parsing --------------------------------------------------------

/// Parse KERNEL_REPORTS value. Empty/missing → all enabled.
pub fn parse_report_config(raw: &str) -> ReportConfig {
    if raw.is_empty() {
        return ReportConfig { daily: true, critical: true, completion: true, weekly: true };
    }
    let parts: Vec<&str> = raw.split(',').map(str::trim).collect();
    ReportConfig {
        daily: parts.contains(&"daily"),
        critical: parts.contains(&"critical"),
        completion: parts.contains(&"completion"),
        weekly: parts.contains(&"weekly"),
    }
}

/// Load ReportConfig from KERNEL_REPORTS env var.
pub fn report_config_from_env() -> ReportConfig {
    let raw = std::env::var("KERNEL_REPORTS").unwrap_or_default();
    parse_report_config(&raw)
}

// ----- Message formatters (Italian) ------------------------------------------

/// Format the daily 09:00 CET report.
pub fn format_daily_report(m: &DailyMetrics) -> String {
    let events = if m.critical_events == 0 {
        "Nessun evento critico.".to_owned()
    } else {
        format!("{} eventi critici rilevati.", m.critical_events)
    };
    format!(
        "Buongiorno. Ieri: {} piani attivi, {} task completati, costo ${:.2}. \
         Agenti usati: {}. {}",
        m.active_plans, m.completed_tasks, m.cost_yesterday_usd, m.agents_used, events,
    )
}

/// Format the plan-completion event report.
pub fn format_plan_complete(name: &str, cost: &str, duration: &str) -> String {
    format!("Piano {name} completato. Costo {cost}, durata {duration}.")
}

/// Format the wave-merge event report.
pub fn format_wave_merge(info: &WaveInfo) -> String {
    let next = match &info.next_wave {
        Some(w) => format!(" Prossima wave: {w}."),
        None => " Ultima wave — piano completato.".to_owned(),
    };
    format!("Wave {} del piano {} mergiata.{}", info.wave_id, info.plan_name, next)
}

/// Format the weekly Sunday 20:00 CET report.
pub fn format_weekly_report(
    active_plans: u32,
    total_cost: &str,
    completed_plans: u32,
    learnings: &str,
) -> String {
    format!(
        "Riepilogo settimanale. Piani attivi: {active_plans}, completati: {completed_plans}. \
         Costo totale: {total_cost}. Apprendimenti: {learnings}."
    )
}

// ----- Internal send helper --------------------------------------------------

async fn send_report(text: &str) {
    let token = match std::env::var("CONVERGIO_TELEGRAM_TOKEN") {
        Ok(v) => Some(v),
        Err(_) => None,
    };
    let chat_id: Option<i64> = match std::env::var("CONVERGIO_TELEGRAM_CHAT_ID") {
        Ok(v) => match v.parse() {
            Ok(id) => Some(id),
            Err(e) => {
                warn!("kernel.reports: CONVERGIO_TELEGRAM_CHAT_ID parse error: {e}");
                None
            }
        },
        Err(_) => None,
    };

    match (token.as_deref(), chat_id) {
        (Some(tok), Some(cid)) => {
            if let Err(e) = send_text(tok, cid, text, None).await {
                warn!("kernel.reports: send failed: {e}");
            } else {
                info!("kernel.reports: sent — {}", &text[..text.len().min(60)]);
            }
        }
        _ => warn!("kernel.reports: CONVERGIO_TELEGRAM_TOKEN or CHAT_ID not set — skipped"),
    }
}

/// Returns true when quiet hours are active (delegates to QuietHoursConfig).
fn is_quiet() -> bool {
    QuietHoursConfig::from_env().map(|q| q.is_active_now()).unwrap_or(false)
}

// ----- Event-driven public API -----------------------------------------------

/// Send a plan-completion report. Called by other modules on plan finish.
/// Skips if KERNEL_REPORTS does not include "completion".
pub async fn report_plan_complete(name: &str, cost: &str, duration: &str) {
    let cfg = report_config_from_env();
    if !cfg.completion {
        return;
    }
    if is_quiet() {
        info!("kernel.reports: plan_complete suppressed — quiet hours");
        return;
    }
    send_report(&format_plan_complete(name, cost, duration)).await;
}

/// Send a wave-merge report. Called by other modules after wave merge.
/// Skips if KERNEL_REPORTS does not include "completion".
pub async fn report_wave_merge(info: &WaveInfo) {
    let cfg = report_config_from_env();
    if !cfg.completion {
        return;
    }
    if is_quiet() {
        info!("kernel.reports: wave_merge suppressed — quiet hours");
        return;
    }
    send_report(&format_wave_merge(info)).await;
}

// ----- Scheduled report loop -------------------------------------------------

/// Spawn the background report loop.
/// Ticks every 60 s; fires daily at 09:00 CET (UTC+1=08:00 UTC) and weekly Sunday 20:00 CET.
/// `daemon_url` is reserved for future /api/overview + /api/metrics integration.
pub fn spawn_report_loop(
    _token: String,
    _chat_id: i64,
    _daemon_url: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        // Why 60 s tick: cheap enough to detect the target minute without drift.
        let mut tick = interval(Duration::from_secs(60));
        tick.tick().await; // consume the immediate first tick
        loop {
            tick.tick().await;
            let cfg = report_config_from_env();
            let now_utc = chrono::Utc::now();
            // CET = UTC+1 (no DST adjustment — env owner sets quiet hours in UTC offset)
            let cet_hour = ((now_utc.hour() + 1) % 24) as u8;
            let cet_min = now_utc.minute() as u8;
            let weekday = now_utc.weekday(); // Mon=0 … Sun=6

            // Daily report — 09:00 CET (= 08:00 UTC)
            if cfg.daily && cet_hour == 9 && cet_min == 0 && !is_quiet() {
                let metrics = DailyMetrics {
                    active_plans: 0,
                    completed_tasks: 0,
                    cost_yesterday_usd: 0.0,
                    agents_used: 0,
                    critical_events: 0,
                };
                send_report(&format_daily_report(&metrics)).await;
            }

            // Weekly report — Sunday 20:00 CET (= 19:00 UTC)
            if cfg.weekly
                && weekday == chrono::Weekday::Sun
                && cet_hour == 20
                && cet_min == 0
                && !is_quiet()
            {
                let msg = format_weekly_report(0, "$0.00", 0, "Nessuno");
                send_report(&msg).await;
            }
        }
    })
}

// ----- Tests (external file) -------------------------------------------------

#[cfg(test)]
#[path = "reports_tests.rs"]
mod tests;
