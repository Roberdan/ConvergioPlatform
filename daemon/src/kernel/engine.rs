// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// KernelEngine — wraps AppleFmBridge for situational classification.
// Extend AppleFmBridge; do not spawn a parallel subprocess.

use crate::ipc::models::apple_fm::{AppleFmBridge, InferenceRequest};
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

    /// Ask the local model a question with real system context.
    /// Fetches live data from daemon API, injects into prompt, then asks Mistral.
    /// Strips mlx_lm debug output (=====, Prompt:, Generation:, Peak memory:).
    pub fn ask(&self, question: &str) -> String {
        if !self.bridge.is_available() || self.loaded_model.is_none() {
            return "Il modello locale non e' disponibile. Riprova piu' tardi.".to_string();
        }
        let model = self.loaded_model.as_ref().unwrap();
        // Gather real context from daemon API
        let context = gather_system_context();
        let prompt = format!(
            "Sei l'assistente del sistema Convergio, una piattaforma di orchestrazione AI.\n\
             Rispondi in italiano, in modo conciso e preciso.\n\
             Usa SOLO i dati di contesto forniti. Non inventare informazioni.\n\n\
             === Stato attuale del sistema ===\n\
             {context}\n\n\
             === Domanda dell'utente ===\n\
             {question}\n\n\
             === Risposta (basata solo sui dati sopra) ==="
        );
        let req = InferenceRequest {
            prompt,
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

/// Fetch live system state from daemon API for context injection.
fn gather_system_context() -> String {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new());
    let base = "http://localhost:8420";
    let mut ctx = String::new();

    // Plans
    if let Ok(r) = client.get(format!("{base}/api/plan-db/list")).send() {
        if let Ok(v) = r.json::<serde_json::Value>() {
            if let Some(plans) = v.get("plans").and_then(|p| p.as_array()) {
                let doing: Vec<_> = plans.iter()
                    .filter(|p| p.get("status").and_then(|s| s.as_str()) == Some("doing"))
                    .collect();
                ctx += &format!("Piani attivi: {}\n", doing.len());
                for p in &doing {
                    let name = p.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                    let done = p.get("tasks_done").and_then(|n| n.as_u64()).unwrap_or(0);
                    let total = p.get("tasks_total").and_then(|n| n.as_u64()).unwrap_or(0);
                    let id = p.get("id").and_then(|n| n.as_u64()).unwrap_or(0);
                    ctx += &format!("  - Piano {id}: {name} ({done}/{total} task)\n");
                }
                ctx += &format!("Piani totali nel DB: {}\n", plans.len());
            }
        }
    }

    // Kernel status
    if let Ok(r) = client.get(format!("{base}/api/kernel/status")).send() {
        if let Ok(v) = r.json::<serde_json::Value>() {
            let models = v.get("models_loaded").and_then(|n| n.as_u64()).unwrap_or(0);
            let uptime = v.get("uptime_secs").and_then(|n| n.as_u64()).unwrap_or(0);
            ctx += &format!("Kernel: {models} modello caricato, uptime {uptime}s\n");
        }
    }

    // Node readiness
    if let Ok(r) = client.get(format!("{base}/api/node/readiness")).send() {
        if let Ok(v) = r.json::<serde_json::Value>() {
            let ok = v.get("ok").and_then(|b| b.as_bool()).unwrap_or(false);
            let node = v.get("node").and_then(|s| s.as_str()).unwrap_or("?");
            ctx += &format!("Nodo: {node}, readiness: {}\n", if ok { "tutto OK" } else { "problemi rilevati" });
        }
    }

    if ctx.is_empty() {
        ctx = "Nessun dato disponibile dal daemon.".to_string();
    }
    ctx
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
