// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Voice command intent classifier + router — strictly command-response, no memory.
// Italian output formatted for macOS Alice TTS voice.

use crate::kernel::engine::KernelEngine;
use crate::kernel::voice_router_helpers;
use serde_json::Value;
use tracing::debug;

pub use crate::kernel::voice_routes::route_intent;

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
    /// Explicit escalation to Ali via chat API (async polling, up to 60s)
    EscalateToAli { question: String },
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

/// Full pipeline: text → classify → route → Italian response.
/// Returns empty string when muted; designed for TtsEngine + SttEngine integration.
pub fn voice_command(text: &str, engine: &KernelEngine, daemon_url: &str) -> String {
    if voice_router_helpers::is_muted() {
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

pub(crate) fn keyword_classify(text: &str) -> VoiceIntent {
    let s = text.to_lowercase();
    // Explicit escalation to Ali — MUST be checked FIRST (before "stato"/"cost" etc.)
    if s.contains("chiedi ad ali") || s.starts_with("ali ") || s.starts_with("ali,") || s == "ali"
        || s.contains(" ali ") || s.contains("opus") || s.contains("cloud")
    {
        return VoiceIntent::EscalateToAli { question: text.to_string() };
    }
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
    // Anything the kernel can't handle → forward to Mistral with MCP tools
    VoiceIntent::AskAli { question: text.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- keyword_classify sanity ---

    #[test]
    fn keyword_status_check_triggers_on_stato() {
        assert_eq!(keyword_classify("qual è lo stato"), VoiceIntent::StatusCheck);
    }

    #[test]
    fn keyword_cost_query_triggers_on_costo() {
        assert_eq!(keyword_classify("mostrami la spesa"), VoiceIntent::CostQuery);
    }

    // --- EscalateToAli keyword classification ---

    #[test]
    fn keyword_escalate_to_ali_triggers_on_chiedi_ad_ali() {
        assert!(matches!(
            keyword_classify("chiedi ad ali cos'è successo"),
            VoiceIntent::EscalateToAli { .. }
        ));
    }

    #[test]
    fn keyword_escalate_to_ali_triggers_on_ali() {
        assert!(matches!(
            keyword_classify("ali dimmi lo stato"),
            VoiceIntent::EscalateToAli { .. }
        ));
    }

    #[test]
    fn keyword_escalate_to_ali_triggers_on_opus() {
        assert!(matches!(
            keyword_classify("chiedi a opus"),
            VoiceIntent::EscalateToAli { .. }
        ));
    }

    #[test]
    fn keyword_escalate_to_ali_triggers_on_cloud() {
        assert!(matches!(
            keyword_classify("usa il cloud per analizzare"),
            VoiceIntent::EscalateToAli { .. }
        ));
    }

    #[test]
    fn keyword_escalate_preserves_original_question() {
        let text = "chiedi ad ali il costo totale";
        match keyword_classify(text) {
            VoiceIntent::EscalateToAli { question } => assert_eq!(question, text),
            other => panic!("expected EscalateToAli, got {other:?}"),
        }
    }
}
