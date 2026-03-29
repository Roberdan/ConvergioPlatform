// NDJSON parsing helpers for the Claude session reader task.
// Extracted from claude_session.rs — stateless text extraction functions.

use crate::tui::claude_session::ChatEvent;

/// Resolve MCP config path relative to the repo root.
pub(super) fn mcp_config_path() -> String {
    // Try DASHBOARD_DB parent (repo root), fallback to cwd.
    if let Ok(db) = std::env::var("DASHBOARD_DB") {
        if let Some(parent) = std::path::Path::new(&db).parent() {
            let p = parent.join("../config/mcp-ali.json");
            if p.exists() {
                return p.to_string_lossy().to_string();
            }
        }
    }
    // Fallback: relative to cwd
    "config/mcp-ali.json".to_string()
}

/// Process a single parsed NDJSON line and emit ChatEvents.
/// Returns the updated `partial` buffer after processing.
pub(super) fn process_ndjson_line(
    parsed: &serde_json::Value,
    partial: &mut String,
    event_tx: &tokio::sync::mpsc::UnboundedSender<ChatEvent>,
) {
    let msg_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match msg_type {
        "system" => {
            // Extract session_id from init message.
            if let Some(sid) = parsed.get("session_id").and_then(|v| v.as_str()) {
                if let Err(e) = event_tx.send(ChatEvent::SessionReady(sid.to_string())) {
                    tracing::warn!("event send (session ready): {e}");
                }
            }
        }
        "content_block_delta" => {
            // Token-level streaming delta.
            if let Some(delta) = parsed
                .get("delta")
                .and_then(|d| d.get("text"))
                .and_then(|t| t.as_str())
            {
                partial.push_str(delta);
                if let Err(e) = event_tx.send(ChatEvent::TextDelta(delta.to_string())) {
                    tracing::warn!("event send (text delta): {e}");
                }
            }
        }
        "assistant" => {
            // Complete assistant message.
            let text = extract_assistant_text(parsed);
            if !text.is_empty() {
                partial.clear();
                if let Err(e) = event_tx.send(ChatEvent::MessageComplete(text)) {
                    tracing::warn!("event send (message complete): {e}");
                }
            }
        }
        "result" => {
            // Final result — extract text if present.
            let text = if !partial.is_empty() {
                std::mem::take(partial)
            } else {
                extract_result_text(parsed)
            };
            if !text.is_empty() {
                if let Err(e) = event_tx.send(ChatEvent::MessageComplete(text)) {
                    tracing::warn!("event send (result complete): {e}");
                }
            }
        }
        "error" => {
            let msg = parsed
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .or_else(|| parsed.get("message").and_then(|m| m.as_str()))
                .unwrap_or("unknown error");
            if let Err(e) = event_tx.send(ChatEvent::Error(msg.to_string())) {
                tracing::warn!("event send (error): {e}");
            }
        }
        _ => {
            // Ignore other message types (tool_use, user, etc.)
        }
    }
}

/// Extract text content from an "assistant" NDJSON message.
pub(super) fn extract_assistant_text(val: &serde_json::Value) -> String {
    // Try content array with text blocks.
    if let Some(content) = val
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    {
        let parts: Vec<&str> = content
            .iter()
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
pub(super) fn extract_result_text(val: &serde_json::Value) -> String {
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
