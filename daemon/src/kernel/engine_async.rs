// Async wrappers for KernelEngine inference methods.
// WHY: classify() and ask_with_history() call blocking subprocesses (mlx_lm via
// AppleFmBridge). Running them directly in an async handler stalls the tokio
// executor thread. These wrappers use spawn_blocking to offload to the blocking
// thread pool, keeping the async runtime responsive.

use std::sync::{Arc, Mutex};

use crate::ipc::models::apple_fm::{AppleFmBridge, InferenceRequest};
use crate::kernel::cloud_escalation;
use crate::kernel::engine::{
    heuristic_classify, parse_inference_response, KernelAction, KernelEngine, KernelSeverity,
};
use crate::kernel::engine_context::smart_context_gather;
use crate::kernel::engine_tool_loop;

/// Async-safe classify: runs MLX inference in the blocking thread pool.
///
/// Clones only the data required (cli_path, model name, situation text)
/// so the closure is Send + 'static without transferring KernelEngine ownership.
pub async fn classify_async(
    engine: &Arc<Mutex<KernelEngine>>,
    situation: &str,
) -> KernelAction {
    let (cli_path, model, situation) = {
        let eng = engine.lock().unwrap_or_else(|p| p.into_inner());
        let cli_path = if eng.is_local_available() {
            Some(eng.bridge.cli_path.clone())
        } else {
            None
        };
        let model = eng.loaded_model.clone();
        (cli_path, model, situation.to_string())
    };

    let (Some(cli_path_opt), Some(model)) = (cli_path, model) else {
        let s = situation.clone();
        return tokio::task::spawn_blocking(move || heuristic_classify(&s))
            .await
            .unwrap_or_else(|_| heuristic_classify(&situation));
    };

    tokio::task::spawn_blocking(move || {
        let bridge = AppleFmBridge { cli_path: cli_path_opt };
        let prompt = format!(
            "Classify this situation as OK, WARN, or CRITICAL and give a one-sentence reason \
             and a one-word action (none/throttle/alert/restart).\nSituation: {situation}\nAnswer:"
        );
        let req = InferenceRequest { prompt, model: Some(model), timeout_secs: 30 };
        match bridge.infer(&req) {
            Ok(resp) => parse_inference_response(&resp.text),
            Err(_) => heuristic_classify(&situation),
        }
    })
    .await
    .unwrap_or_else(|_| KernelAction {
        severity: KernelSeverity::Ok,
        action: "none".to_string(),
        reason: "classify task panicked; safe fallback".to_string(),
    })
}

/// Async-safe ask: runs tool-loop inference in the blocking thread pool.
pub async fn ask_async(
    engine: &Arc<Mutex<KernelEngine>>,
    question: &str,
    history_chatml: &str,
) -> String {
    let (cli_path_opt, model) = {
        let eng = engine.lock().unwrap_or_else(|p| p.into_inner());
        let cli_path = if eng.is_local_available() {
            Some(eng.bridge.cli_path.clone())
        } else {
            None
        };
        (cli_path, eng.loaded_model.clone())
    };

    let (Some(cli_path), Some(model)) = (cli_path_opt, model) else {
        return "Il modello locale non e' disponibile. Riprova piu' tardi.".to_string();
    };

    let question = question.to_string();
    let history = history_chatml.to_string();
    let daemon_url = cloud_escalation::daemon_url();

    tokio::task::spawn_blocking(move || {
        let bridge = AppleFmBridge { cli_path };
        let context = smart_context_gather(&question, &daemon_url);
        let tools_block = engine_tool_loop::tool_descriptions_block();
        let prompt = engine_tool_loop::build_ask_prompt(
            &context, &question, &tools_block, &history,
        );
        engine_tool_loop::run_tool_loop(&bridge, &model, prompt, &daemon_url)
    })
    .await
    .unwrap_or_else(|e| format!("inference task failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::engine::KernelConfig;

    fn make_engine() -> Arc<Mutex<KernelEngine>> {
        Arc::new(Mutex::new(KernelEngine::new(KernelConfig {
            active_node: "test".to_string(),
            default_model: "mlx-community/Qwen2.5-7B-Instruct-4bit".to_string(),
        })))
    }

    #[tokio::test]
    async fn classify_async_no_model_returns_heuristic() {
        let engine = make_engine();
        let action = classify_async(&engine, "CPU at 95%").await;
        assert!(!action.reason.is_empty());
    }

    #[tokio::test]
    async fn ask_async_no_model_returns_unavailable_msg() {
        let engine = make_engine();
        let result = ask_async(&engine, "hello", "").await;
        assert!(!result.is_empty());
    }
}
