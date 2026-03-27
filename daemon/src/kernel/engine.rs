// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// KernelEngine — wraps AppleFmBridge for situational classification.
// Extend AppleFmBridge; do not spawn a parallel subprocess.

use crate::ipc::models::apple_fm::{AppleFmBridge, InferenceRequest};
use crate::kernel::tools;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Classification severity from kernel inference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KernelSeverity {
    Ok,
    Warn,
    Critical,
}

/// Structured output of a classification call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelAction {
    pub severity: KernelSeverity,
    /// Recommended action (e.g. "throttle", "restart", "alert", "none").
    pub action: String,
    pub reason: String,
}

/// Snapshot of the kernel engine state for the status endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelStatus {
    pub models_loaded: u32,
    pub ram_gb: f64,
    pub uptime_secs: u64,
    pub active_node: Option<String>,
    pub last_check: Option<String>,
}

/// Configuration for KernelEngine.
#[derive(Debug, Clone)]
pub struct KernelConfig {
    /// Name of the node this engine runs on (persisted to kernel_config table).
    pub active_node: String,
    /// Default MLX model identifier used for classification inference.
    pub default_model: String,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            active_node: hostname::get()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            default_model: "mlx-community/Llama-3.2-1B-Instruct-4bit".to_string(),
        }
    }
}

/// Core kernel engine: holds an AppleFmBridge reference, config, and loaded model state.
pub struct KernelEngine {
    bridge: AppleFmBridge,
    config: KernelConfig,
    loaded_model: Option<String>,
    started_at: Instant,
    last_check_ts: Option<String>,
}

impl KernelEngine {
    /// Create a new engine. Bridge is unavailable on non-Apple-Silicon — that is expected.
    pub fn new(config: KernelConfig) -> Self {
        Self {
            bridge: AppleFmBridge::new(),
            config,
            loaded_model: None,
            started_at: Instant::now(),
            last_check_ts: None,
        }
    }

    /// Switch the active model. Only one model is held at a time (single-GPU constraint).
    pub fn load_model(&mut self, name: &str) {
        self.loaded_model = Some(name.to_string());
    }

    /// Returns true when a model has been loaded via `load_model`.
    pub fn is_loaded(&self) -> bool {
        self.loaded_model.is_some()
    }

    /// Classify a situation string via MLX inference.
    ///
    /// Falls back to heuristic keyword-based classification when the bridge is
    /// unavailable (e.g. not Apple Silicon or mlx_lm not installed) so the API
    /// never returns an error from mere unavailability.
    pub fn classify(&self, situation: &str) -> KernelAction {
        // Try MLX inference first.
        if self.bridge.is_available() {
            if let Some(model) = &self.loaded_model {
                return self.classify_via_bridge(situation, model);
            }
        }
        // Heuristic fallback — avoids hard dependency on Apple Silicon in CI.
        heuristic_classify(situation)
    }

    /// Ask the local model a question using tool-augmented inference.
    ///
    /// Flow: build prompt with tool descriptions → call Mistral → parse <tool_call> tags →
    /// execute tool via daemon API → feed result back → return final answer.
    /// Max 2 tool-call rounds to prevent runaway loops.
    /// Strips mlx_lm debug output (=====, Prompt:, Generation:, Peak memory:).
    pub fn ask(&self, question: &str) -> String {
        if !self.bridge.is_available() || self.loaded_model.is_none() {
            return "Il modello locale non e' disponibile. Riprova piu' tardi.".to_string();
        }
        let model = self.loaded_model.as_ref().unwrap();
        let daemon_url = "http://localhost:8420";

        // Build tool descriptions block for prompt injection.
        let tool_list: String = tools::tool_definitions()
            .iter()
            .map(|t| format!("- {}: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n");

        let initial_prompt = format!(
            "[INST] Sei l'assistente Convergio, una piattaforma di orchestrazione AI.\n\
             Hai questi strumenti per ottenere dati reali:\n\
             {tool_list}\n\n\
             Per usare uno strumento scrivi: \
             <tool_call>{{\"name\":\"get_plans\",\"arguments\":{{}}}}</tool_call>\n\
             Per strumenti con parametri: \
             <tool_call>{{\"name\":\"get_plan_detail\",\"arguments\":{{\"plan_id\":729}}}}</tool_call>\n\n\
             Domanda dell'utente: {question}\n\n\
             Rispondi SEMPRE in italiano. Usa gli strumenti per ottenere dati reali prima di rispondere.\n\
             Non inventare dati. Se non sai qualcosa, dillo. [/INST]"
        );

        let mut current_prompt = initial_prompt;
        // Max 2 tool-call rounds.
        for _round in 0..2 {
            let req = InferenceRequest {
                prompt: current_prompt.clone(),
                model: Some(model.to_string()),
                timeout_secs: 60,
            };
            let raw = match self.bridge.infer(&req) {
                Ok(resp) => resp.text,
                Err(e) => return format!("Errore dal modello locale: {e}"),
            };

            // Check for tool call in response.
            if let Some((tool_name, tool_args)) = extract_tool_call(&raw) {
                let args: serde_json::Value = serde_json::from_str(&tool_args)
                    .unwrap_or(serde_json::Value::Object(Default::default()));
                let tool_result = tools::call_tool(&tool_name, daemon_url, &args)
                    .unwrap_or_else(|| format!("{{\"error\":\"unknown tool: {tool_name}\"}}"));

                // Build follow-up prompt with tool result.
                current_prompt = format!(
                    "[INST] Risultato dello strumento {tool_name}:\n\
                     {tool_result}\n\n\
                     Basandoti su questi dati reali, rispondi alla domanda: {question}\n\
                     Rispondi in italiano, in modo conciso. [/INST]"
                );
                // Continue to next round with enriched prompt.
            } else {
                // No tool call — return cleaned final answer.
                return strip_mlx_debug(&raw);
            }
        }

        // Final inference after max rounds.
        let req = InferenceRequest {
            prompt: current_prompt,
            model: Some(model.to_string()),
            timeout_secs: 60,
        };
        match self.bridge.infer(&req) {
            Ok(resp) => strip_mlx_debug(&resp.text),
            Err(e) => format!("Errore dal modello locale: {e}"),
        }
    }

    /// Snapshot the current engine state.
    pub fn status(&self) -> KernelStatus {
        KernelStatus {
            models_loaded: if self.loaded_model.is_some() { 1 } else { 0 },
            ram_gb: query_ram_gb(),
            uptime_secs: self.started_at.elapsed().as_secs(),
            active_node: Some(self.config.active_node.clone()),
            last_check: self.last_check_ts.clone(),
        }
    }

    // --- private helpers ---

    fn classify_via_bridge(&self, situation: &str, model: &str) -> KernelAction {
        let prompt = format!(
            "Classify this situation as OK, WARN, or CRITICAL and give a one-sentence reason \
             and a one-word action (none/throttle/alert/restart).\nSituation: {situation}\nAnswer:"
        );
        let req = InferenceRequest {
            prompt,
            model: Some(model.to_string()),
            timeout_secs: 30,
        };
        match self.bridge.infer(&req) {
            Ok(resp) => parse_inference_response(&resp.text),
            Err(_) => heuristic_classify(situation),
        }
    }
}

/// Parse the raw text from the model into a KernelAction.
fn parse_inference_response(text: &str) -> KernelAction {
    let upper = text.to_uppercase();
    let severity = if upper.contains("CRITICAL") {
        KernelSeverity::Critical
    } else if upper.contains("WARN") {
        KernelSeverity::Warn
    } else {
        KernelSeverity::Ok
    };
    let action = if upper.contains("RESTART") {
        "restart"
    } else if upper.contains("THROTTLE") {
        "throttle"
    } else if upper.contains("ALERT") {
        "alert"
    } else {
        "none"
    };
    KernelAction {
        severity,
        action: action.to_string(),
        reason: text.chars().take(200).collect(),
    }
}

/// Fast keyword heuristic used when MLX is unavailable or no model is loaded.
fn heuristic_classify(situation: &str) -> KernelAction {
    let lower = situation.to_lowercase();
    if lower.contains("95%")
        || lower.contains("critical")
        || lower.contains("down")
        || lower.contains("crash")
        || lower.contains("error")
    {
        return KernelAction {
            severity: KernelSeverity::Critical,
            action: "alert".to_string(),
            reason: "heuristic: critical keyword detected".to_string(),
        };
    }
    if lower.contains("high")
        || lower.contains("warn")
        || lower.contains("slow")
        || lower.contains("90%")
        || lower.contains("cpu usage")
    {
        return KernelAction {
            severity: KernelSeverity::Warn,
            action: "throttle".to_string(),
            reason: "heuristic: warning keyword detected".to_string(),
        };
    }
    KernelAction {
        severity: KernelSeverity::Ok,
        action: "none".to_string(),
        reason: "heuristic: no concerning keywords".to_string(),
    }
}

/// Read total installed RAM in GB using sysinfo.
fn query_ram_gb() -> f64 {
    use sysinfo::System;
    let sys = System::new_all();
    sys.total_memory() as f64 / 1_073_741_824.0 // bytes → GB
}

/// Parse the first <tool_call>...</tool_call> block from model output.
/// Returns (tool_name, arguments_json_string) or None if no call found.
fn extract_tool_call(text: &str) -> Option<(String, String)> {
    let start_tag = "<tool_call>";
    let end_tag = "</tool_call>";
    let start = text.find(start_tag)?;
    let end = text.find(end_tag)?;
    let json_str = text[start + start_tag.len()..end].trim();
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let name = v.get("name")?.as_str()?.to_string();
    let args = v
        .get("arguments")
        .map(|a| a.to_string())
        .unwrap_or_else(|| "{}".to_string());
    Some((name, args))
}

/// Strip mlx_lm debug output from model response.
/// Removes lines starting with "=", "Prompt:", "Generation:", "Peak memory:".
fn strip_mlx_debug(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let t = line.trim();
            !t.starts_with('=')
                && !t.starts_with("Prompt:")
                && !t.starts_with("Generation:")
                && !t.starts_with("Peak memory:")
                && !t.starts_with("Calling `python")
                && !t.starts_with("Use `mlx_lm")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_tool_call_parses_no_args() {
        let text = r#"Sure! <tool_call>{"name":"get_plans","arguments":{}}</tool_call>"#;
        let result = extract_tool_call(text);
        assert!(result.is_some());
        let (name, args) = result.unwrap();
        assert_eq!(name, "get_plans");
        assert_eq!(args, "{}");
    }

    #[test]
    fn extract_tool_call_parses_with_args() {
        let text =
            r#"<tool_call>{"name":"get_plan_detail","arguments":{"plan_id":729}}</tool_call>"#;
        let result = extract_tool_call(text);
        assert!(result.is_some());
        let (name, args) = result.unwrap();
        assert_eq!(name, "get_plan_detail");
        // arguments JSON must contain plan_id 729
        let v: serde_json::Value = serde_json::from_str(&args).unwrap();
        assert_eq!(v.get("plan_id").and_then(|v| v.as_u64()), Some(729));
    }

    #[test]
    fn extract_tool_call_returns_none_when_absent() {
        let text = "Ecco la risposta senza tool call.";
        assert!(extract_tool_call(text).is_none());
    }

    #[test]
    fn extract_tool_call_returns_none_on_malformed_json() {
        let text = "<tool_call>not json at all</tool_call>";
        assert!(extract_tool_call(text).is_none());
    }

    #[test]
    fn strip_mlx_debug_removes_debug_lines() {
        let raw = "=====\nPrompt: 10\nGeneration: 5\nPeak memory: 1.2 GB\nRisposta utile";
        let cleaned = strip_mlx_debug(raw);
        assert_eq!(cleaned, "Risposta utile");
    }

    #[test]
    fn strip_mlx_debug_keeps_normal_lines() {
        let raw = "Ci sono 3 piani attivi.\nIl piano 734 e' in corso.";
        let cleaned = strip_mlx_debug(raw);
        assert_eq!(cleaned, raw);
    }
}
