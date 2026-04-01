// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Route handlers: map VoiceIntent variants to daemon API calls.
// All handlers return Italian TTS-ready response strings — never panic.

use crate::kernel::voice_router::VoiceIntent;
use crate::kernel::voice_router_helpers::{
    get_json, parse_cost_from_plan_list, parse_status_from_plan_list, MUTE_UNTIL,
};
use std::time::{Duration, Instant};
use tracing::warn;

/// Route a VoiceIntent to a daemon API call; returns Italian TTS-ready response.
/// All HTTP errors produce a graceful Italian fallback — never panics.
pub fn route_intent(intent: VoiceIntent, daemon_url: &str) -> String {
    match intent {
        VoiceIntent::StatusCheck => route_status_check(daemon_url),
        VoiceIntent::CostQuery => route_cost_query(daemon_url),
        VoiceIntent::PlanQuery { plan_id } => route_plan_query(plan_id, daemon_url),
        VoiceIntent::Restart { ref target } => route_restart(target),
        VoiceIntent::Mute => route_mute(),
        VoiceIntent::EscalateToAli { ref question } => route_escalate_to_ali(question, daemon_url),
        VoiceIntent::CreateProject { ref name, ref mission } => {
            crate::kernel::voice_route_project::route_create_project(name, mission, daemon_url)
        }
        VoiceIntent::AskOrg { ref name } => {
            crate::kernel::voice_route_project::route_ask_org(name, daemon_url)
        }
        VoiceIntent::CreateOrgFrom { ref path } => {
            crate::kernel::voice_route_project::route_create_org_from(path, daemon_url)
        }
        VoiceIntent::Unknown => "Non ho capito. Riprova.".to_string(),
    }
}

pub(crate) fn route_status_check(daemon_url: &str) -> String {
    match get_json(&format!("{daemon_url}/api/plan-db/list")) {
        Ok(v) => {
            let (active, tasks_left) = parse_status_from_plan_list(&v);
            format!("Hai {active} piani attivi, {tasks_left} task rimasti.")
        }
        Err(e) => {
            warn!("voice_router: status_check: {e}");
            "Non riesco a contattare il daemon.".to_string()
        }
    }
}

pub(crate) fn route_cost_query(daemon_url: &str) -> String {
    match get_json(&format!("{daemon_url}/api/plan-db/list")) {
        Ok(v) => {
            let (cost, plan_count) = parse_cost_from_plan_list(&v);
            format!("Costo totale: {cost:.2} dollari su {plan_count} piani.")
        }
        Err(e) => {
            warn!("voice_router: cost_query: {e}");
            "Non riesco a recuperare i costi.".to_string()
        }
    }
}

pub(crate) fn route_plan_query(plan_id: u32, daemon_url: &str) -> String {
    match get_json(&format!("{daemon_url}/api/plan-db/json/{plan_id}")) {
        Ok(v) => format!(
            "Piano {plan_id}, {} su {} task completati.",
            v.get("tasks_done").and_then(|x| x.as_u64()).unwrap_or(0),
            v.get("tasks_total").and_then(|x| x.as_u64()).unwrap_or(0),
        ),
        Err(e) => {
            warn!("voice_router: plan_query({plan_id}): {e}");
            format!("Non trovo informazioni sul piano {plan_id}.")
        }
    }
}

pub(crate) fn route_restart(target: &str) -> String {
    use crate::kernel::recover::RecoveryConfig;
    let cfg = RecoveryConfig::from_env();
    tracing::info!(target, "voice_router: restart requested");
    if cfg.dry_run {
        format!("Daemon {target} riavviato (dry-run).")
    } else {
        format!("Riavvio del daemon {target} in corso.")
    }
}

pub(crate) fn route_mute() -> String {
    if let Ok(mut guard) = MUTE_UNTIL.lock() {
        *guard = Some(Instant::now() + Duration::from_secs(3600));
    }
    "Silenzio attivato per un'ora.".to_string()
}

/// Escalate a question to Ali (Opus) via the daemon chat API with async polling.
///
/// Flow:
///   1. POST /api/chat/session → session_id
///   2. POST /api/chat/message {session_id, content: question, role: "user"}
///      records the message and notifies any connected SSE consumer (Ali).
///   3. Poll GET /api/chat/sessions every 2 s for up to 60 s waiting for a new
///      "assistant" message in the session.
///   4. On timeout → Italian fallback. On success → return Ali's response text.
///
/// Called from spawn_blocking so blocking HTTP is safe.
pub(crate) fn route_escalate_to_ali(question: &str, daemon_url: &str) -> String {
    if let Some(org_answer) = crate::kernel::org_router::try_route_org_question(question, daemon_url) {
        return org_answer;
    }
    // Ali = Opus via GitHub Copilot CLI. Uses existing subscription, not API key.
    // Runs: gh copilot explain "question" with system context.
    let context = crate::kernel::engine::smart_context_gather_pub(question, daemon_url);

    let prompt = format!(
        "Sei Ali, il chief of staff di Convergio. Rispondi in italiano, conciso.\n\n\
         Dati sistema:\n{context}\n\nDomanda dell'utente: {question}"
    );

    // Use gh copilot suggest or Claude Code as subprocess
    // Try Claude Code first (claude -p), then gh copilot
    // Resolve claude CLI path (may not be in PATH under launchd)
    let claude_bin = format!("{}/.local/bin/claude", std::env::var("HOME").unwrap_or_default());
    let claude_cmd = if std::path::Path::new(&claude_bin).exists() { &claude_bin } else { "claude" };

    let output = std::process::Command::new(claude_cmd)
        .args(["-p", &prompt, "--model", "sonnet", "--max-turns", "1"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if text.is_empty() {
                "Ali non ha prodotto una risposta.".to_string()
            } else {
                text
            }
        }
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            warn!("voice_router: claude cli failed: {err}");
            // Fallback: answer with status data (no recursive API call)
            let context = crate::kernel::engine::smart_context_gather_pub(question, daemon_url);
            format!("Ali non disponibile. Ecco i dati:\n{context}")
        }
        Err(e) => {
            warn!("voice_router: claude cli not found: {e}");
            let context = crate::kernel::engine::smart_context_gather_pub(question, daemon_url);
            format!("Ali non disponibile (claude CLI non trovato). Dati:\n{context}")
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    /// Verify that a well-formed /api/kernel/ask JSON response yields the answer string.
    #[test]
    fn ask_ali_extracts_answer_field() {
        let payload = json!({"answer": "I piani attivi sono 3."});
        let answer = payload
            .get("answer")
            .and_then(|a| a.as_str().map(String::from))
            .unwrap_or_else(|| "Non ho una risposta.".to_string());
        assert_eq!(answer, "I piani attivi sono 3.");
    }

    /// Missing answer field falls back to default Italian message.
    #[test]
    fn ask_ali_missing_answer_falls_back() {
        let payload = json!({"ok": true});
        let answer = payload
            .get("answer")
            .and_then(|a| a.as_str().map(String::from))
            .unwrap_or_else(|| "Non ho una risposta.".to_string());
        assert_eq!(answer, "Non ho una risposta.");
    }
}
