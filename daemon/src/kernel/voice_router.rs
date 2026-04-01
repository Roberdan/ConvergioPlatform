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
    /// Escalation to Ali via chat API (async polling, up to 60s)
    EscalateToAli { question: String },
    /// Create a new project (org + plan + tasks) from natural language
    CreateProject { name: String, mission: String },
    /// Query an existing org's status by name
    AskOrg { name: String },
    /// Scan a repo and create an org from its profile
    CreateOrgFrom { path: String },
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
         status_check, cost_query, plan_query, restart, mute, \
         create_project, create_org_from, ask_org, ask_ali. \
         Use create_project when user wants to create/launch a new project or org. \
         Use create_org_from when user wants to scan/analyze a repo to create an org. \
         Use ask_org when user asks about an existing project/org status. \
         Use ask_ali for anything complex that needs reasoning. \
         Command: '{text}'. Return JSON: \
         {{\"intent\": \"<name>\", \"params\": {{\"plan_id\": null, \"target\": null, \
         \"name\": null, \"mission\": null}}}}"
    );
    parse_llm_json(&engine.classify(&prompt).reason, text)
}

fn parse_llm_json(raw: &str, original_text: &str) -> Option<VoiceIntent> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    let v: Value = match serde_json::from_str(&raw[start..=end]) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let name = v.get("intent")?.as_str()?.to_lowercase();
    let params = v.get("params");
    match name.as_str() {
        "status_check" => Some(VoiceIntent::StatusCheck),
        "cost_query" => Some(VoiceIntent::CostQuery),
        "mute" => Some(VoiceIntent::Mute),
        "unknown" | "ask_ali" => Some(VoiceIntent::EscalateToAli { question: original_text.to_string() }),
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
        "create_project" => {
            let (n, m) = extract_project_name_mission(original_text);
            Some(VoiceIntent::CreateProject { name: n, mission: m })
        }
        "ask_org" => {
            let n = extract_org_name_from_text(original_text);
            Some(VoiceIntent::AskOrg { name: n })
        }
        "create_org_from" => {
            let p = extract_path_from_text(original_text);
            Some(VoiceIntent::CreateOrgFrom { path: p })
        }
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
    // CreateOrgFrom — repo scanning, checked before CreateProject
    if s.contains("analizza repo") || s.contains("scan repo")
        || s.contains("crea org da") || s.contains("analyze repo")
        || s.contains("scan project")
    {
        let path = extract_path_from_text(text);
        return VoiceIntent::CreateOrgFrom { path };
    }
    // CreateProject — checked before generic status/plan keywords
    if s.contains("crea progetto") || s.contains("create project")
        || s.contains("lancia progetto") || s.contains("nuovo progetto")
        || s.contains("avvia progetto") || s.contains("start project")
        || s.contains("crea org") || s.contains("crea organizzazione")
        || s.contains("create org")
    {
        let (name, mission) = extract_project_name_mission(text);
        return VoiceIntent::CreateProject { name, mission };
    }
    // AskOrg — "come sta il X?", "status di X", "aggiorna su X", "update on X"
    if s.contains("come sta") || s.contains("status di")
        || s.contains("aggiorna su") || s.contains("update on")
    {
        let name = extract_org_name_from_text(text);
        return VoiceIntent::AskOrg { name };
    }
    // Report/analysis/execution keywords → escalate to Ali
    if ["report", "analisi", "analizza", "riassumi", "summary", "briefing",
        "lancia", "esegui", "fai", "run", "execute", "deploy"]
        .iter().any(|kw| s.contains(kw))
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
        let id = s.split_whitespace().filter_map(|t| match t.parse::<u32>() {
            Ok(v) => Some(v),
            Err(_) => None,
        }).next().unwrap_or(0);
        return VoiceIntent::PlanQuery { plan_id: id };
    }
    // Anything the kernel can't handle → escalate to Ali
    VoiceIntent::EscalateToAli { question: text.to_string() }
}

// --- Project/Org text extraction helpers -------------------------------------

/// Extract project name and mission from natural language.
/// Pattern: "crea progetto FITNESS con obiettivo PERDERE 5KG" -> ("FITNESS", "PERDERE 5KG")
pub(crate) fn extract_project_name_mission(text: &str) -> (String, String) {
    let s = text.to_lowercase();
    let triggers = [
        "crea progetto", "create project", "lancia progetto",
        "nuovo progetto", "avvia progetto", "start project",
        "crea organizzazione", "crea org", "create org",
    ];
    let after = triggers.iter()
        .filter_map(|t| s.find(t).map(|pos| &text[pos + t.len()..]))
        .next()
        .unwrap_or(text)
        .trim();
    let mission_seps = ["con obiettivo", "with goal", "obiettivo:"];
    for sep in &mission_seps {
        if let Some(idx) = after.to_lowercase().find(sep) {
            let name = after[..idx].trim().to_string();
            let mission = after[idx + sep.len()..].trim().to_string();
            if !name.is_empty() {
                return (name, mission);
            }
        }
    }
    let mut parts = after.splitn(2, char::is_whitespace);
    let name = parts.next().unwrap_or("project").trim().to_string();
    let mission = parts.next().unwrap_or("").trim().to_string();
    (if name.is_empty() { "project".to_string() } else { name }, mission)
}

/// Extract a filesystem path from repo-scanning commands.
/// Pattern: "analizza repo /path/to/repo" -> "/path/to/repo"
pub(crate) fn extract_path_from_text(text: &str) -> String {
    // First try: find a token that looks like a filesystem path
    if let Some(p) = text.split_whitespace().find(|w| w.starts_with('/') || w.starts_with("~/")) {
        return p.to_string();
    }
    // Fallback: take everything after the trigger keyword
    let s = text.to_lowercase();
    let triggers = [
        "analizza repo", "scan repo", "crea org da",
        "analyze repo", "scan project",
    ];
    triggers.iter()
        .filter_map(|t| s.find(t).map(|pos| text[pos + t.len()..].trim()))
        .find(|after| !after.is_empty())
        .unwrap_or(".")
        .to_string()
}

/// Extract org/project name from a status query like "come sta il fitness?"
pub(crate) fn extract_org_name_from_text(text: &str) -> String {
    let s = text.to_lowercase();
    let prefixes = [
        "come sta il ", "come sta ", "status di ",
        "aggiorna su ", "update on ",
    ];
    let after = prefixes.iter()
        .filter_map(|p| s.find(p).map(|pos| &text[pos + p.len()..]))
        .next()
        .unwrap_or(text);
    after.trim().trim_matches(|c: char| c == '?' || c == '!' || c == '.').trim().to_string()
}
