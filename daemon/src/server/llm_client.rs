use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    Claude,  // api.anthropic.com
    LiteLLM, // localhost:4000, OpenAI-compatible
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone)]
pub enum StreamChunk {
    Text(String),
    Usage(TokenUsage),
    Error(String),
}

pub type ChatStream = Pin<Box<dyn futures_util::Stream<Item = StreamChunk> + Send>>;

/// Stream a chat completion. Returns text deltas, usage, or errors.
pub fn stream_chat(provider: Provider, model: &str, messages: Vec<ChatMessage>) -> ChatStream {
    let model = model.to_string();
    let (tx, rx) = mpsc::channel::<StreamChunk>(64);
    tokio::spawn(async move {
        let result = match provider {
            Provider::Claude => stream_claude(&tx, &model, &messages).await,
            Provider::LiteLLM => stream_litellm(&tx, &model, &messages).await,
        };
        if let Err(e) = result {
            let _ = tx.send(StreamChunk::Error(e)).await;
        }
    });
    Box::pin(ReceiverStream::new(rx))
}

/// Consume an SSE byte stream, split on double-newlines, parse each block with `parse_fn`.
async fn consume_sse<F>(
    tx: &mpsc::Sender<StreamChunk>,
    resp: reqwest::Response,
    parse_fn: F,
) -> Result<(), String>
where
    F: Fn(&str) -> Option<StreamChunk>,
{
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("stream read error: {e}"))?;
        buf.push_str(&String::from_utf8_lossy(&bytes));
        while let Some(pos) = buf.find("\n\n") {
            let block = buf[..pos].to_string();
            buf = buf[pos + 2..].to_string();
            if let Some(c) = parse_fn(&block) {
                if tx.send(c).await.is_err() {
                    return Ok(());
                }
            }
        }
    }
    Ok(())
}

/// Extract the `data: ...` payload from an SSE block.
fn sse_data(block: &str) -> Option<&str> {
    block.lines().find_map(|l| l.strip_prefix("data: "))
}

// -- Claude direct API: DISABLED --
// SECURITY: Never call Anthropic API directly with API keys.
// All LLM calls MUST go through LiteLLM proxy (Provider::LiteLLM)
// or through Claude Code/Copilot subscriptions (OAuth).
// API keys are NOT allowed in Convergio.

async fn stream_claude(
    _tx: &mpsc::Sender<StreamChunk>,
    model: &str,
    _messages: &[ChatMessage],
) -> Result<(), String> {
    warn!("BLOCKED: Direct Anthropic API call attempted for model '{}'. Use LiteLLM proxy instead. Start with: convergio-llm.sh start", model);
    Err(format!(
        "BLOCKED: Direct API calls disabled. Route through LiteLLM proxy (localhost:4000) or use Claude Code/Copilot subscription. Model: {model}"
    ))
}

// -- LiteLLM proxy (POST http://localhost:4000/v1/chat/completions) --

async fn stream_litellm(
    tx: &mpsc::Sender<StreamChunk>,
    model: &str,
    messages: &[ChatMessage],
) -> Result<(), String> {
    let api_key = std::env::var("LITELLM_API_KEY").unwrap_or_default();
    let body = json!({"model": model, "stream": true, "messages": messages});
    let mut req = Client::new()
        .post("http://localhost:4000/v1/chat/completions")
        .header("accept", "text/event-stream")
        .header("content-type", "application/json")
        .json(&body);
    if !api_key.is_empty() {
        req = req.header("authorization", format!("Bearer {api_key}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("LiteLLM request failed: {e}"))?;
    if !resp.status().is_success() {
        let s = resp.status();
        return Err(format!(
            "LiteLLM {s}: {}",
            resp.text().await.unwrap_or_default()
        ));
    }
    consume_sse(tx, resp, parse_openai_sse).await
}

/// Parse choices[0].delta.content and usage from OpenAI-compatible SSE.
fn parse_openai_sse(block: &str) -> Option<StreamChunk> {
    let data = sse_data(block)?;
    if data.trim() == "[DONE]" {
        return None;
    }
    let parsed: Value = serde_json::from_str(data).ok()?;
    if let Some(u) = parsed.get("usage") {
        let inp = u.get("prompt_tokens").and_then(Value::as_u64).unwrap_or(0);
        let out = u
            .get("completion_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if inp > 0 || out > 0 {
            return Some(StreamChunk::Usage(TokenUsage {
                input_tokens: inp,
                output_tokens: out,
            }));
        }
    }
    let content = parsed
        .get("choices")?
        .get(0)?
        .get("delta")?
        .get("content")?
        .as_str()?;
    if content.is_empty() {
        return None;
    }
    Some(StreamChunk::Text(content.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_claude_content_block_delta() {
        let block = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}";
        assert!(matches!(parse_claude_sse(block), Some(StreamChunk::Text(t)) if t == "Hello"));
    }

    #[test]
    fn parse_claude_message_start_usage() {
        let block = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":42,\"output_tokens\":0}}}";
        assert!(
            matches!(parse_claude_sse(block), Some(StreamChunk::Usage(u)) if u.input_tokens == 42)
        );
    }

    #[test]
    fn parse_openai_delta_content() {
        let block = "data: {\"choices\":[{\"delta\":{\"content\":\"world\"},\"index\":0}]}";
        assert!(matches!(parse_openai_sse(block), Some(StreamChunk::Text(t)) if t == "world"));
    }

    #[test]
    fn parse_openai_done() {
        assert!(parse_openai_sse("data: [DONE]").is_none());
    }

    #[test]
    fn parse_openai_usage_chunk() {
        let block = "data: {\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":20}}";
        assert!(
            matches!(parse_openai_sse(block), Some(StreamChunk::Usage(u)) if u.input_tokens == 10 && u.output_tokens == 20)
        );
    }
}
