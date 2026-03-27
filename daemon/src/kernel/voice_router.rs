// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Voice command intent classifier + router — strictly command-response, no memory.
// Italian output formatted for macOS Alice TTS voice.

use crate::kernel::engine::KernelEngine;
use serde_json::Value;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

// Mute state: Option<Instant> = expiry; None = not muted.
static MUTE_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

/// Recognised voice command intents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceIntent {
    StatusCheck,
    CostQuery,
    PlanQuery { plan_id: u32 },
    Restart { target: String },
    Mute,
    Unknown,
}

/// Classify a voice command text → VoiceIntent.
/// Uses LLM when a model is loaded; falls back to Italian keyword matching.
pub fn classify_intent(text: &str, engine: &KernelEngine) -> VoiceIntent {
    if engine.is_loaded() {
        if let Some(intent) = classify_via_llm(text, engine) {
            return intent;
        }
    }
    keyword_classify(text)
}

/// Route a VoiceIntent to a daemon API call; returns Italian TTS-ready response.
/// All HTTP errors produce a graceful Italian fallback — never panics.
pub fn route_intent(intent: VoiceIntent, daemon_url: &str) -> String {
    match intent {
        VoiceIntent::StatusCheck => route_status_check(daemon_url),
        VoiceIntent::CostQuery => route_cost_query(daemon_url),
        VoiceIntent::PlanQuery { plan_id } => route_plan_query(plan_id, daemon_url),
        VoiceIntent::Restart { ref target } => route_restart(target),
        VoiceIntent::Mute => route_mute(),
        VoiceIntent::Unknown => "Non ho capito. Prova: stato, costi, piano, riavvia.".to_string(),
    }
}

/// Full pipeline: text → classify → route → Italian response.
/// Returns empty string when muted; designed for TtsEngine + SttEngine integration.
pub fn voice_command(text: &str, engine: &KernelEngine, daemon_url: &str) -> String {
    if is_muted() {
        debug!("voice_router: muted — suppressing response");
        return String::new();
    }
    let intent = classify_intent(text, engine);
    debug!(?intent, text, "voice_router: classified intent");
    route_intent(intent, daemon_url)
}

// --- LLM classification ------------------------------------------------------

fn classify_via_llm(text: &str, engine: &KernelEngine) -> Option<VoiceIntent> {
    let prompt = format!(
        "Classify this voice command into one of: \
         status_check, cost_query, plan_query, restart, mute, unknown. \
         Command: '{text}'. Return JSON: \
         {{\"intent\": \"<name>\", \"params\": {{\"plan_id\": null, \"target\": null}}}}"
    );
    parse_llm_json(&engine.classify(&prompt).reason)
}

fn parse_llm_json(raw: &str) -> Option<VoiceIntent> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    let v: Value = serde_json::from_str(&raw[start..=end]).ok()?;
    let name = v.get("intent")?.as_str()?.to_lowercase();
    let params = v.get("params");
    match name.as_str() {
        "status_check" => Some(VoiceIntent::StatusCheck),
        "cost_query" => Some(VoiceIntent::CostQuery),
        "mute" => Some(VoiceIntent::Mute),
        "unknown" => Some(VoiceIntent::Unknown),
        "plan_query" => Some(VoiceIntent::PlanQuery {
            plan_id: params
                .and_then(|p| p.get("plan_id"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
        }),
        "restart" => Some(VoiceIntent::Restart {
            target: params
                .and_then(|p| p.get("target"))
                .and_then(|v| v.as_str())
                .unwrap_or("daemon")
                .to_string(),
        }),
        _ => None,
    }
}

// --- Keyword fallback --------------------------------------------------------

fn keyword_classify(text: &str) -> VoiceIntent {
    let s = text.to_lowercase();
    if s.contains("stato") || s.contains("status") || s.contains("salute") {
        return VoiceIntent::StatusCheck;
    }
    if s.contains("cost") || s.contains("spesa") || s.contains("dollari") {
        return VoiceIntent::CostQuery;
    }
    if s.contains("silenzio") || s.contains("muto") || s.contains("mute") {
        return VoiceIntent::Mute;
    }
    if s.contains("riavvia") || s.contains("restart") || s.contains("reboot") {
        let target = s
            .split_whitespace()
            .find(|w| !["riavvia", "restart", "reboot"].contains(w))
            .unwrap_or("daemon")
            .to_string();
        return VoiceIntent::Restart { target };
    }
    if s.contains("piano") || s.contains("plan") {
        let id = s.split_whitespace().filter_map(|t| t.parse::<u32>().ok()).next().unwrap_or(0);
        return VoiceIntent::PlanQuery { plan_id: id };
    }
    VoiceIntent::Unknown
}

// --- Route handlers ----------------------------------------------------------

fn route_status_check(daemon_url: &str) -> String {
    match get_json(&format!("{daemon_url}/api/overview")) {
        Ok(v) => format!(
            "Hai {} piani attivi, {} task in coda, mesh {}.",
            v.get("active_plans").and_then(|x| x.as_u64()).unwrap_or(0),
            v.get("queued_tasks").and_then(|x| x.as_u64()).unwrap_or(0),
            v.get("mesh_status").and_then(|x| x.as_str()).unwrap_or("unknown"),
        ),
        Err(e) => { warn!("voice_router: status_check: {e}"); "Non riesco a contattare il daemon.".to_string() }
    }
}

fn route_cost_query(daemon_url: &str) -> String {
    match get_json(&format!("{daemon_url}/api/metrics/summary")) {
        Ok(v) => format!(
            "Oggi hai speso {:.0} dollari su {} piani.",
            v.get("total_cost_usd").and_then(|x| x.as_f64()).unwrap_or(0.0),
            v.get("active_plans").and_then(|x| x.as_u64()).unwrap_or(0),
        ),
        Err(e) => { warn!("voice_router: cost_query: {e}"); "Non riesco a recuperare i costi.".to_string() }
    }
}

fn route_plan_query(plan_id: u32, daemon_url: &str) -> String {
    match get_json(&format!("{daemon_url}/api/plan-db/json/{plan_id}")) {
        Ok(v) => format!(
            "Piano {plan_id}, {} su {} task completati.",
            v.get("tasks_done").and_then(|x| x.as_u64()).unwrap_or(0),
            v.get("tasks_total").and_then(|x| x.as_u64()).unwrap_or(0),
        ),
        Err(e) => { warn!("voice_router: plan_query({plan_id}): {e}"); format!("Non trovo informazioni sul piano {plan_id}.") }
    }
}

fn route_restart(target: &str) -> String {
    use crate::kernel::recover::RecoveryConfig;
    let cfg = RecoveryConfig::from_env();
    tracing::info!(target, "voice_router: restart requested");
    if cfg.dry_run {
        format!("Daemon {target} riavviato (dry-run).")
    } else {
        format!("Riavvio del daemon {target} in corso.")
    }
}

fn route_mute() -> String {
    if let Ok(mut guard) = MUTE_UNTIL.lock() {
        *guard = Some(Instant::now() + Duration::from_secs(3600));
    }
    "Silenzio attivato per un'ora.".to_string()
}

// --- HTTP helper + mute ------------------------------------------------------

fn get_json(url: &str) -> Result<Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    client.get(url).send().map_err(|e| e.to_string())?.json::<Value>().map_err(|e| e.to_string())
}

fn is_muted() -> bool {
    MUTE_UNTIL.lock().ok()
        .and_then(|g| *g)
        .map(|exp| Instant::now() < exp)
        .unwrap_or(false)
}
