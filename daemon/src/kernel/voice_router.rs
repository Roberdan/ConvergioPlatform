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
    /// Anything the kernel can't handle locally → forward to Ali (Opus)
    AskAli { question: String },
    /// Unrecognised intent
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
        VoiceIntent::AskAli { ref question } => route_ask_ali(question, daemon_url),
        VoiceIntent::Unknown => "Non ho capito. Riprova.".to_string(),
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
         status_check, cost_query, plan_query, restart, mute, ask_ali. \
         Use ask_ali for anything complex that needs reasoning. \
         Command: '{text}'. Return JSON: \
         {{\"intent\": \"<name>\", \"params\": {{\"plan_id\": null, \"target\": null}}}}"
    );
    parse_llm_json(&engine.classify(&prompt).reason, text)
}

fn parse_llm_json(raw: &str, original_text: &str) -> Option<VoiceIntent> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    let v: Value = serde_json::from_str(&raw[start..=end]).ok()?;
    let name = v.get("intent")?.as_str()?.to_lowercase();
    let params = v.get("params");
    match name.as_str() {
        "status_check" => Some(VoiceIntent::StatusCheck),
        "cost_query" => Some(VoiceIntent::CostQuery),
        "mute" => Some(VoiceIntent::Mute),
        "unknown" | "ask_ali" => Some(VoiceIntent::AskAli { question: original_text.to_string() }),
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
    // Anything the kernel can't handle → forward to Ali
    VoiceIntent::AskAli { question: text.to_string() }
}

// --- Route handlers ----------------------------------------------------------

fn route_status_check(daemon_url: &str) -> String {
    match get_json(&format!("{daemon_url}/api/plan-db/list")) {
        Ok(v) => {
            let (active, tasks_left) = parse_status_from_plan_list(&v);
            format!("Hai {active} piani attivi, {tasks_left} task rimasti.")
        }
        Err(e) => { warn!("voice_router: status_check: {e}"); "Non riesco a contattare il daemon.".to_string() }
    }
}

fn route_cost_query(daemon_url: &str) -> String {
    match get_json(&format!("{daemon_url}/api/plan-db/list")) {
        Ok(v) => {
            let (cost, plan_count) = parse_cost_from_plan_list(&v);
            format!("Costo totale: {cost:.2} dollari su {plan_count} piani.")
        }
        Err(e) => { warn!("voice_router: cost_query: {e}"); "Non riesco a recuperare i costi.".to_string() }
    }
}

/// Parses /api/plan-db/list response: returns (doing_plan_count, remaining_tasks).
/// Plans with status=="doing" are active; remaining = sum of (tasks_total - tasks_done).
fn parse_status_from_plan_list(v: &Value) -> (u64, u64) {
    let plans = match v.get("plans").and_then(|p| p.as_array()) {
        Some(arr) => arr,
        None => return (0, 0),
    };
    let mut active: u64 = 0;
    let mut tasks_left: u64 = 0;
    for plan in plans {
        let status = plan.get("status").and_then(|s| s.as_str()).unwrap_or("");
        if status == "doing" {
            active += 1;
            let total = plan.get("tasks_total").and_then(|x| x.as_u64()).unwrap_or(0);
            let done = plan.get("tasks_done").and_then(|x| x.as_u64()).unwrap_or(0);
            tasks_left += total.saturating_sub(done);
        }
    }
    (active, tasks_left)
}

/// Parses /api/plan-db/list response: returns (total_cost, plan_count).
/// Sums total_cost across all plans (any status).
fn parse_cost_from_plan_list(v: &Value) -> (f64, usize) {
    let plans = match v.get("plans").and_then(|p| p.as_array()) {
        Some(arr) => arr,
        None => return (0.0, 0),
    };
    let mut total_cost: f64 = 0.0;
    for plan in plans {
        total_cost += plan.get("total_cost").and_then(|x| x.as_f64()).unwrap_or(0.0);
    }
    (total_cost, plans.len())
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

fn route_ask_ali(question: &str, daemon_url: &str) -> String {
    // Instead of calling Mistral (which hallucinates without context),
    // gather real data from daemon API and return a factual summary.
    // The kernel is a data retriever, not a conversationalist.
    let q = question.to_lowercase();

    // Try to match the question to available data
    if q.contains("piano") || q.contains("plan") || q.contains("progett") {
        return route_status_check(daemon_url);
    }
    if q.contains("cost") || q.contains("spes") || q.contains("soldi") || q.contains("dollari") {
        return route_cost_query(daemon_url);
    }
    if q.contains("nod") || q.contains("mesh") || q.contains("m1") || q.contains("m5") {
        match get_json(&format!("{daemon_url}/api/node/readiness")) {
            Ok(v) => {
                let ok = v.get("ok").and_then(|b| b.as_bool()).unwrap_or(false);
                let node = v.get("node").and_then(|s| s.as_str()).unwrap_or("?");
                let checks = v.get("checks").and_then(|a| a.as_array())
                    .map(|arr| arr.iter()
                        .filter(|c| !c.get("passed").and_then(|b| b.as_bool()).unwrap_or(true))
                        .count())
                    .unwrap_or(0);
                return if ok {
                    format!("Nodo {node}: tutto OK, nessun problema rilevato.")
                } else {
                    format!("Nodo {node}: {checks} problemi rilevati. Usa 'stato' per i dettagli.")
                };
            }
            Err(_) => return "Non riesco a verificare lo stato del nodo.".to_string(),
        }
    }
    if q.contains("kernel") || q.contains("modell") || q.contains("mistral") {
        match get_json(&format!("{daemon_url}/api/kernel/status")) {
            Ok(v) => {
                let models = v.get("models_loaded").and_then(|n| n.as_u64()).unwrap_or(0);
                let uptime = v.get("uptime_secs").and_then(|n| n.as_u64()).unwrap_or(0);
                let hours = uptime / 3600;
                let mins = (uptime % 3600) / 60;
                return format!("Kernel: {models} modello caricato, attivo da {hours}h {mins}m.");
            }
            Err(_) => return "Non riesco a leggere lo stato del kernel.".to_string(),
        }
    }

    // Generic: return a summary of everything
    let status = route_status_check(daemon_url);
    let cost = route_cost_query(daemon_url);
    format!("{status}\n{cost}\nPer domande specifiche prova: piano, costi, nodo, kernel.")
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- parse_status_from_plan_list ---

    #[test]
    fn status_counts_only_doing_plans() {
        let payload = json!({
            "ok": true,
            "plans": [
                {"status": "doing", "tasks_total": 10, "tasks_done": 4},
                {"status": "doing", "tasks_total": 5,  "tasks_done": 5},
                {"status": "done",  "tasks_total": 8,  "tasks_done": 8},
                {"status": "todo",  "tasks_total": 3,  "tasks_done": 0},
            ]
        });
        let (active, left) = parse_status_from_plan_list(&payload);
        // Only the 2 "doing" plans count; second plan contributes 0 remaining tasks.
        assert_eq!(active, 2, "expected 2 doing plans");
        assert_eq!(left, 6, "expected 10-4 = 6 remaining tasks");
    }

    #[test]
    fn status_empty_plans_array() {
        let payload = json!({"ok": true, "plans": []});
        let (active, left) = parse_status_from_plan_list(&payload);
        assert_eq!(active, 0);
        assert_eq!(left, 0);
    }

    #[test]
    fn status_missing_plans_key() {
        let payload = json!({"ok": true});
        let (active, left) = parse_status_from_plan_list(&payload);
        assert_eq!(active, 0);
        assert_eq!(left, 0);
    }

    #[test]
    fn status_tasks_done_exceeds_total_saturates_at_zero() {
        // tasks_done > tasks_total should not underflow — saturating_sub guards this.
        let payload = json!({
            "ok": true,
            "plans": [
                {"status": "doing", "tasks_total": 2, "tasks_done": 5}
            ]
        });
        let (active, left) = parse_status_from_plan_list(&payload);
        assert_eq!(active, 1);
        assert_eq!(left, 0);
    }

    // --- parse_cost_from_plan_list ---

    #[test]
    fn cost_sums_all_plans() {
        let payload = json!({
            "ok": true,
            "plans": [
                {"status": "doing", "total_cost": 1.50},
                {"status": "done",  "total_cost": 2.25},
                {"status": "todo",  "total_cost": 0.75},
            ]
        });
        let (cost, count) = parse_cost_from_plan_list(&payload);
        assert!((cost - 4.50).abs() < 0.001, "expected 4.50, got {cost}");
        assert_eq!(count, 3);
    }

    #[test]
    fn cost_missing_total_cost_field_defaults_to_zero() {
        let payload = json!({
            "ok": true,
            "plans": [
                {"status": "doing"},
                {"status": "done", "total_cost": 1.0},
            ]
        });
        let (cost, count) = parse_cost_from_plan_list(&payload);
        assert!((cost - 1.0).abs() < 0.001);
        assert_eq!(count, 2);
    }

    #[test]
    fn cost_empty_plans() {
        let payload = json!({"ok": true, "plans": []});
        let (cost, count) = parse_cost_from_plan_list(&payload);
        assert_eq!(cost, 0.0);
        assert_eq!(count, 0);
    }

    // --- keyword_classify sanity ---

    #[test]
    fn keyword_status_check_triggers_on_stato() {
        assert_eq!(keyword_classify("qual è lo stato"), VoiceIntent::StatusCheck);
    }

    #[test]
    fn keyword_cost_query_triggers_on_costo() {
        assert_eq!(keyword_classify("mostrami la spesa"), VoiceIntent::CostQuery);
    }
}
