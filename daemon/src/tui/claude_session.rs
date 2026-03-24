// Persistent Claude CLI session — NDJSON over stdin/stdout.
// Spawns `claude` once, reuses for all chat messages. Streams token deltas.

use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

/// Events emitted by the Claude session reader task.
#[derive(Debug, Clone)]
pub enum ChatEvent {
    /// Incremental text delta (token-level streaming).
    TextDelta(String),
    /// Complete assistant message (final aggregated text).
    MessageComplete(String),
    /// Session initialization with session_id.
    SessionReady(String),
    /// Error from the session process.
    Error(String),
}

/// Manages a persistent `claude` subprocess with NDJSON I/O.
pub struct ClaudeSession {
    child: Child,
    stdin: Option<tokio::process::ChildStdin>,
    pub event_rx: mpsc::UnboundedReceiver<ChatEvent>,
}

impl ClaudeSession {
    /// Spawn a persistent `claude` session with stream-json I/O.
    pub fn spawn() -> Result<Self, String> {
        let mut child = Command::new("claude")
            .args([
                "--input-format", "stream-json",
                "--output-format", "stream-json",
                "--verbose",
                "--agent", "ali-chief-of-staff",
                "--model", "haiku",
                "--dangerously-skip-permissions",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to spawn claude: {e}"))?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take()
            .ok_or_else(|| "failed to capture claude stdout".to_string())?;

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Reader task: parse NDJSON lines from stdout.
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut partial = String::new();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                let parsed: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let msg_type = parsed.get("type").and_then(|v| v.as_str())
                    .unwrap_or("");

                match msg_type {
                    "system" => {
                        // Extract session_id from init message.
                        if let Some(sid) = parsed.get("session_id")
                            .and_then(|v| v.as_str())
                        {
                            let _ = event_tx.send(ChatEvent::SessionReady(
                                sid.to_string(),
                            ));
                        }
                    }
                    "content_block_delta" => {
                        // Token-level streaming delta.
                        if let Some(delta) = parsed.get("delta")
                            .and_then(|d| d.get("text"))
                            .and_then(|t| t.as_str())
                        {
                            partial.push_str(delta);
                            let _ = event_tx.send(ChatEvent::TextDelta(
                                delta.to_string(),
                            ));
                        }
                    }
                    "assistant" => {
                        // Complete assistant message.
                        let text = extract_assistant_text(&parsed);
                        if !text.is_empty() {
                            partial.clear();
                            let _ = event_tx.send(ChatEvent::MessageComplete(
                                text,
                            ));
                        }
                    }
                    "result" => {
                        // Final result — extract text if present.
                        let text = if !partial.is_empty() {
                            std::mem::take(&mut partial)
                        } else {
                            extract_result_text(&parsed)
                        };
                        if !text.is_empty() {
                            let _ = event_tx.send(ChatEvent::MessageComplete(
                                text,
                            ));
                        }
                    }
                    "error" => {
                        let msg = parsed.get("error")
                            .and_then(|e| e.get("message"))
                            .and_then(|m| m.as_str())
                            .or_else(|| parsed.get("message")
                                .and_then(|m| m.as_str()))
                            .unwrap_or("unknown error");
                        let _ = event_tx.send(ChatEvent::Error(
                            msg.to_string(),
                        ));
                    }
                    _ => {
                        // Ignore other message types (tool_use, user, etc.)
                    }
                }
            }

            // Process exited — notify.
            let _ = event_tx.send(ChatEvent::Error(
                "Claude session ended".to_string(),
            ));
        });

        Ok(Self { child, stdin, event_rx })
    }

    /// Send a user message to the persistent session.
    pub async fn send(&mut self, content: &str) -> Result<(), String> {
        let stdin = self.stdin.as_mut()
            .ok_or_else(|| "stdin not available".to_string())?;

        let msg = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [{"type": "text", "text": content}]
            }
        });
        let mut line = serde_json::to_string(&msg)
            .map_err(|e| format!("json serialize: {e}"))?;
        line.push('\n');

        stdin.write_all(line.as_bytes()).await
            .map_err(|e| format!("write to claude stdin: {e}"))?;
        stdin.flush().await
            .map_err(|e| format!("flush claude stdin: {e}"))?;

        Ok(())
    }

    /// Check if the subprocess is still running.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for ClaudeSession {
    fn drop(&mut self) {
        // Best-effort kill on drop.
        let _ = self.child.start_kill();
    }
}

/// Extract text content from an "assistant" NDJSON message.
fn extract_assistant_text(val: &serde_json::Value) -> String {
    // Try content array with text blocks.
    if let Some(content) = val.get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    {
        let parts: Vec<&str> = content.iter()
            .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
            .collect();
        if !parts.is_empty() {
            return parts.join("");
        }
    }
    // Fallback: direct content string.
    val.get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string()
}

/// Extract text from a "result" NDJSON message.
fn extract_result_text(val: &serde_json::Value) -> String {
    val.get("result")
        .and_then(|r| r.as_str())
        .or_else(|| val.get("text").and_then(|t| t.as_str()))
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_assistant_text_from_content_blocks() {
        let val = serde_json::json!({
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Plan 708 is "},
                    {"type": "text", "text": "complete."}
                ]
            }
        });
        assert_eq!(extract_assistant_text(&val), "Plan 708 is complete.");
    }

    #[test]
    fn extract_assistant_text_from_string_content() {
        let val = serde_json::json!({
            "type": "assistant",
            "message": {
                "role": "assistant",
                "content": "3 agents running."
            }
        });
        assert_eq!(extract_assistant_text(&val), "3 agents running.");
    }

    #[test]
    fn extract_result_text_from_result_field() {
        let val = serde_json::json!({
            "type": "result",
            "result": "Done"
        });
        assert_eq!(extract_result_text(&val), "Done");
    }

    #[test]
    fn extract_result_text_empty_when_missing() {
        let val = serde_json::json!({"type": "result"});
        assert_eq!(extract_result_text(&val), "");
    }
}
