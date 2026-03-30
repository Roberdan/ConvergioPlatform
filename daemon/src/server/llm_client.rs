use super::state::ApiError;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::pin::Pin;
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    ClaudeSubscription,    // `claude -p` CLI (OAuth subscription)
    CopilotSubscription,   // `gh copilot -p` CLI (GitHub subscription)
    LocalLLM,              // Ollama/MLX at localhost:8321
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

pub fn stream_chat(provider: Provider, model: &str, messages: Vec<ChatMessage>) -> ChatStream {
    let model = model.to_string();
    let (tx, rx) = mpsc::channel::<StreamChunk>(64);
    tokio::spawn(async move {
        let result = match provider {
            Provider::ClaudeSubscription => {
                stream_cli_subscription(&tx, "claude", &model, &messages).await
            }
            Provider::CopilotSubscription => {
                stream_cli_subscription(&tx, "gh", &model, &messages).await
            }
            Provider::LocalLLM => stream_local_llm(&tx, &model, &messages).await,
        };
        if let Err(e) = result {
            if let Err(send_err) = tx.send(StreamChunk::Error(e.to_string())).await {
                warn!("llm error chunk send failed: {send_err}");
            }
        }
    });
    Box::pin(ReceiverStream::new(rx))
}

fn build_prompt(messages: &[ChatMessage]) -> String {
    messages.iter().map(|msg| match msg.role.as_str() {
        "system" => format!("[System]: {}", msg.content),
        "assistant" => format!("[Assistant]: {}", msg.content),
        _ => msg.content.clone(),
    }).collect::<Vec<_>>().join("\n\n")
}

async fn stream_cli_subscription(
    tx: &mpsc::Sender<StreamChunk>,
    cli: &str,
    model: &str,
    messages: &[ChatMessage],
) -> Result<(), ApiError> {
    let prompt = build_prompt(messages);

    let mut cmd = if cli == "gh" {
        let mut c = tokio::process::Command::new("gh");
        c.args(["copilot", "-p", &prompt]);
        c
    } else {
        let mut c = tokio::process::Command::new("claude");
        c.args(["-p", &prompt, "--model", model]);
        c
    };

    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| {
        ApiError::internal(format!(
            "failed to spawn {cli} subprocess: {e}. Is '{cli}' installed?"
        ))
    })?;

    info!(cli = cli, model = model, "spawned CLI subprocess");
    let stdout = child.stdout.take()
        .ok_or_else(|| ApiError::internal("no stdout from CLI subprocess"))?;

    let reader = tokio::io::BufReader::new(stdout);
    let mut lines = reader.lines();
    let mut char_count: u64 = 0;

    while let Some(line) = lines.next_line().await.map_err(|e| {
        ApiError::internal(format!("reading CLI stdout: {e}"))
    })? {
        char_count += line.len() as u64 + 1;
        let text = format!("{line}\n");
        if tx.send(StreamChunk::Text(text)).await.is_err() {
            break; // client disconnected
        }
    }

    let status = child.wait().await.map_err(|e| {
        ApiError::internal(format!("waiting for {cli} subprocess: {e}"))
    })?;

    if !status.success() {
        let stderr_handle = child.stderr.take();
        let stderr_msg = if let Some(stderr) = stderr_handle {
            let mut buf = String::new();
            let mut reader = tokio::io::BufReader::new(stderr);
            let _ = reader.read_line(&mut buf).await; // intentional: best-effort stderr capture
            buf
        } else {
            String::new()
        };
        return Err(ApiError::internal(format!(
            "{cli} exited with {status}: {stderr_msg}"
        )));
    }

    let approx_tokens = char_count / 4; // ~4 chars/token
    if approx_tokens > 0 {
        // intentional: best-effort usage report
        let _ = tx.send(StreamChunk::Usage(TokenUsage {
            input_tokens: 0, output_tokens: approx_tokens,
        })).await;
    }
    Ok(())
}

async fn consume_sse<F>(tx: &mpsc::Sender<StreamChunk>, resp: reqwest::Response, parse_fn: F) -> Result<(), ApiError>
where F: Fn(&str) -> Option<StreamChunk> {
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| ApiError::internal(format!("stream read: {e}")))?;
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

fn sse_data(block: &str) -> Option<&str> {
    block.lines().find_map(|l| l.strip_prefix("data: "))
}

async fn stream_local_llm(
    tx: &mpsc::Sender<StreamChunk>,
    model: &str,
    messages: &[ChatMessage],
) -> Result<(), ApiError> {
    let port = std::env::var("LOCAL_LLM_PORT").unwrap_or_else(|_| "8321".into());
    let url = format!("http://localhost:{port}/v1/chat/completions");
    let body = json!({"model": model, "stream": true, "messages": messages});

    let resp = Client::new()
        .post(&url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::internal(format!("LocalLLM request to {url}: {e}")))?;

    if !resp.status().is_success() {
        let s = resp.status();
        return Err(ApiError::internal(format!(
            "LocalLLM {s}: {}",
            resp.text().await.unwrap_or_default()
        )));
    }
    consume_sse(tx, resp, parse_openai_sse).await
}

pub fn parse_claude_sse(block: &str) -> Option<StreamChunk> {
    let data = sse_data(block)?;
    // intentional: malformed SSE frames are skipped so streaming can continue
    let parsed: Value = serde_json::from_str(data).ok()?;
    let event_type = parsed.get("type").and_then(Value::as_str).unwrap_or("");
    if event_type == "content_block_delta" {
        let text = parsed.get("delta")?.get("text")?.as_str()
            .filter(|s| !s.is_empty())?;
        return Some(StreamChunk::Text(text.to_string()));
    }
    if event_type == "message_start" {
        if let Some(usage) = parsed.get("message").and_then(|m| m.get("usage")) {
            let inp = usage.get("input_tokens").and_then(Value::as_u64).unwrap_or(0);
            let out = usage.get("output_tokens").and_then(Value::as_u64).unwrap_or(0);
            if inp > 0 || out > 0 {
                return Some(StreamChunk::Usage(TokenUsage {
                    input_tokens: inp,
                    output_tokens: out,
                }));
            }
        }
    }
    None
}

pub fn parse_openai_sse(block: &str) -> Option<StreamChunk> {
    let data = sse_data(block)?;
    if data.trim() == "[DONE]" {
        return None;
    }
    // intentional: malformed SSE frames are skipped so streaming can continue
    let parsed: Value = serde_json::from_str(data).ok()?;
    if let Some(u) = parsed.get("usage") {
        let inp = u.get("prompt_tokens").and_then(Value::as_u64).unwrap_or(0);
        let out = u.get("completion_tokens").and_then(Value::as_u64).unwrap_or(0);
        if inp > 0 || out > 0 {
            return Some(StreamChunk::Usage(TokenUsage {
                input_tokens: inp, output_tokens: out,
            }));
        }
    }
    let content = parsed.get("choices")?.get(0)?.get("delta")?
        .get("content")?.as_str()?;
    if content.is_empty() {
        return None;
    }
    Some(StreamChunk::Text(content.to_string()))
}

#[cfg(test)]
#[path = "llm_client_tests.rs"]
mod tests;
