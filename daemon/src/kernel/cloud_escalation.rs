// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Cloud escalation — async LLM via Convergio provider/fallback pipeline.
// Used when local model unavailable or for write-intent operations.

use crate::kernel::engine_context::{extract_tool_call, smart_context_gather_pub};
use crate::kernel::tools::ToolCatalog;
use crate::server::llm_client::{ChatMessage, StreamChunk};
use crate::server::provider::stream_with_fallback;
use futures_util::StreamExt;
use tracing::{info, warn};

/// Default cloud model for kernel escalation.
pub const CLOUD_MODEL: &str = "claude-opus-4-20250514";
const MAX_CLOUD_ROUNDS: usize = 5;

fn daemon_url() -> String {
    std::env::var("DAEMON_URL").unwrap_or_else(|_| "http://localhost:8420".to_string())
}

/// Ask via cloud provider — no tool loop.
pub async fn cloud_ask(question: &str, history_chatml: &str) -> String {
    let q = question.to_string();
    let context = match tokio::task::spawn_blocking(move || {
        smart_context_gather_pub(&q, &daemon_url())
    })
    .await
    {
        Ok(ctx) => ctx,
        Err(e) => {
            warn!(error = %e, "kernel: context gather failed");
            String::from("Nessun contesto disponibile.")
        }
    };
    let messages = build_messages(&context, question, history_chatml, false);
    collect_stream(messages).await
}

/// Ask via cloud provider WITH tool calling loop (up to 5 rounds).
pub async fn cloud_ask_with_tools(question: &str, history: &str) -> String {
    let q = question.to_string();
    let context = match tokio::task::spawn_blocking(move || {
        smart_context_gather_pub(&q, &daemon_url())
    })
    .await
    {
        Ok(ctx) => ctx,
        Err(e) => {
            warn!(error = %e, "kernel: context gather failed");
            String::from("Nessun contesto disponibile.")
        }
    };

    let mut messages = build_messages(&context, question, history, true);
    let du = daemon_url();

    for round in 0..MAX_CLOUD_ROUNDS {
        let response = collect_stream(messages.clone()).await;

        let Some((tool_name, args_str)) = extract_tool_call(&response) else {
            return strip_after_tool_call(&response);
        };

        info!(round, tool = %tool_name, "kernel.cloud: tool call");
        let args: serde_json::Value =
            serde_json::from_str(&args_str).unwrap_or(serde_json::json!({}));
        let url = du.clone();
        let tn = tool_name.clone();
        let tool_result = tokio::task::spawn_blocking(move || {
            ToolCatalog::all()
                .call_tool(&tn, &url, &args)
                .unwrap_or_else(|| format!("Tool '{tn}' not found."))
        })
        .await
        .unwrap_or_else(|e| format!("Tool dispatch error: {e}"));

        messages.push(ChatMessage {
            role: "assistant".into(),
            content: response,
        });
        messages.push(ChatMessage {
            role: "user".into(),
            content: format!("[Tool result: {tool_name}]\n{tool_result}"),
        });
    }
    "Raggiunto il limite di chiamate strumenti cloud.".into()
}

fn build_messages(
    context: &str,
    question: &str,
    history: &str,
    with_tools: bool,
) -> Vec<ChatMessage> {
    let tools_section = if with_tools {
        ToolCatalog::all().descriptions_block()
    } else {
        String::new()
    };
    let system = format!(
        "Sei Jarvis, l'assistente AI di Convergio. Rispondi in italiano, conciso.\n\n\
         Dati sistema:\n{context}\n\n{tools_section}"
    );
    let mut msgs = vec![ChatMessage {
        role: "system".into(),
        content: system,
    }];
    if !history.is_empty() {
        msgs.push(ChatMessage {
            role: "user".into(),
            content: format!("[Conversazione precedente]\n{history}"),
        });
    }
    msgs.push(ChatMessage {
        role: "user".into(),
        content: question.to_string(),
    });
    msgs
}

async fn collect_stream(messages: Vec<ChatMessage>) -> String {
    let (provider, _) = crate::server::provider::provider_for_model(CLOUD_MODEL);
    info!(model = CLOUD_MODEL, "kernel: cloud inference");
    let mut stream = stream_with_fallback(provider, CLOUD_MODEL, messages);
    let mut text = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            StreamChunk::Text(t) => text.push_str(&t),
            StreamChunk::Error(e) => {
                warn!(error = %e, "kernel: cloud stream error");
                if text.is_empty() {
                    return format!("Errore cloud: {e}");
                }
            }
            StreamChunk::Usage(_) => {}
        }
    }
    if text.is_empty() {
        "Nessuna risposta dal provider cloud.".into()
    } else {
        text.trim().to_string()
    }
}

fn strip_after_tool_call(text: &str) -> String {
    if let Some(pos) = text.find("</tool_call>") {
        let after = text[pos + "</tool_call>".len()..].trim();
        if !after.is_empty() {
            return after.to_string();
        }
    }
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_model_is_opus() {
        assert!(CLOUD_MODEL.contains("opus"));
    }

    #[test]
    fn cloud_model_routes_to_claude() {
        let (p, _) = crate::server::provider::provider_for_model(CLOUD_MODEL);
        assert_eq!(p, crate::server::llm_client::Provider::ClaudeSubscription);
    }

    #[test]
    fn max_rounds_is_5() {
        assert_eq!(MAX_CLOUD_ROUNDS, 5);
    }
}
