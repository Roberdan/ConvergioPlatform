// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Multi-round tool calling loop for KernelEngine.ask().
// Parses <tool_call> tags from LLM output, dispatches via tools::call_tool,
// and re-invokes inference with the tool result appended.

use crate::ipc::models::apple_fm::{AppleFmBridge, InferenceRequest};
use crate::kernel::engine_context::{extract_tool_call, strip_mlx_debug};
use crate::kernel::tools;

/// Maximum tool-call rounds before returning (prevents infinite loops).
const MAX_TOOL_ROUNDS: u32 = 3;

/// Build the tool description block for the system prompt.
/// Format matches what `extract_tool_call` expects: <tool_call>JSON</tool_call>.
pub(crate) fn tool_descriptions_block() -> String {
    let defs = tools::tool_definitions();
    let mut block = String::from(
        "Strumenti disponibili. Per usarli rispondi con:\n\
         <tool_call>{\"name\":\"tool_name\",\"arguments\":{...}}</tool_call>\n\n",
    );
    for def in &defs {
        block += &format!("- {}: {}\n", def.name, def.description);
    }
    block += "\nUsa gli strumenti SOLO quando servono dati che non hai nel contesto.\n";
    block
}

/// Build the ChatML prompt for ask() with context, tools, history, and question.
pub(crate) fn build_ask_prompt(
    context: &str,
    question: &str,
    tools_block: &str,
    history_chatml: &str,
) -> String {
    format!(
        "<|im_start|>system\n\
         Sei l'assistente Convergio, una piattaforma di orchestrazione AI.\n\
         Rispondi SEMPRE in italiano, in modo conciso.\n\
         Analizza e ragiona — non elencare dati grezzi. Dai insight.\n\
         Usa SOLO i dati forniti o gli strumenti. Non inventare nulla.\n\n\
         {tools_block}\
         <|im_end|>\n\
         {history_chatml}\
         <|im_start|>user\n\
         Ecco i dati attuali del sistema:\n\n\
         {context}\n\n\
         Domanda: {question}\n\
         <|im_end|>\n\
         <|im_start|>assistant\n"
    )
}

/// Run the multi-round tool calling loop.
///
/// Given an initial prompt, invokes the model. If the response contains a
/// `<tool_call>`, dispatches it, appends the result, and re-invokes up to
/// `MAX_TOOL_ROUNDS` times. Returns the final text response.
pub(crate) fn run_tool_loop(
    bridge: &AppleFmBridge,
    model: &str,
    initial_prompt: String,
    daemon_url: &str,
) -> String {
    let mut prompt = initial_prompt;
    let mut last_text = String::new();

    for round in 0..MAX_TOOL_ROUNDS {
        let req = InferenceRequest {
            prompt: prompt.clone(),
            model: Some(model.to_string()),
            timeout_secs: 60,
        };
        let raw = match bridge.infer(&req) {
            Ok(resp) => strip_mlx_debug(&resp.text),
            Err(e) => return format!("Errore dal modello locale: {e}"),
        };

        // Check for a tool call in the response
        let Some((tool_name, args_str)) = extract_tool_call(&raw) else {
            // No tool call — return the clean response
            return strip_tool_preamble(&raw);
        };

        tracing::info!(
            "kernel.tool_loop: round {round} calling tool={tool_name}"
        );

        // Parse args and dispatch
        let args: serde_json::Value =
            serde_json::from_str(&args_str).unwrap_or(serde_json::json!({}));
        let tool_result = tools::call_tool(&tool_name, daemon_url, &args)
            .unwrap_or_else(|| format!("Tool '{tool_name}' not found or failed."));

        // Append tool result to conversation and re-invoke
        last_text = raw.clone();
        prompt = format!(
            "{prompt}{raw}\n<|im_end|>\n\
             <|im_start|>tool\n\
             Risultato di {tool_name}:\n{tool_result}\n\
             <|im_end|>\n\
             <|im_start|>assistant\n"
        );
    }

    // Exhausted rounds — return whatever the model last said
    if last_text.is_empty() {
        "Raggiunto il limite di chiamate strumenti.".to_string()
    } else {
        strip_tool_preamble(&last_text)
    }
}

/// Remove any text before the tool call tag so the user sees only the
/// final natural-language answer. If there is text after the tool_call
/// closing tag, return that; otherwise return the full text.
fn strip_tool_preamble(text: &str) -> String {
    if let Some(end_pos) = text.find("</tool_call>") {
        let after = text[end_pos + "</tool_call>".len()..].trim();
        if !after.is_empty() {
            return after.to_string();
        }
    }
    text.to_string()
}

#[cfg(test)]
#[path = "engine_tool_loop_tests.rs"]
mod tests;
