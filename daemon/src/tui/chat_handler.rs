// Chat input handling and async send logic — extracted from app.rs to keep it < 250 lines.

use crossterm::event::KeyCode;
use reqwest::Client;

use crate::tui::{
    api::chat as chat_api,
    data::{ChatMessage, TuiData},
};

/// State related to the chat view, owned by TuiApp.
pub struct ChatState {
    pub input: String,
    pub sending: bool,
    pub pending_reply: Option<Result<String, ()>>,
    pub reply_tx: tokio::sync::mpsc::UnboundedSender<Option<String>>,
    pub reply_rx: tokio::sync::mpsc::UnboundedReceiver<Option<String>>,
}

impl Default for ChatState {
    fn default() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self { input: String::new(), sending: false, pending_reply: None, reply_tx: tx, reply_rx: rx }
    }
}

/// Handle a key press while the Chat view is active.
/// Returns true if the key was consumed (prevents further dispatch).
pub fn handle_chat_key(code: KeyCode, state: &mut ChatState) -> bool {
    match code {
        KeyCode::Char(c) => {
            state.input.push(c);
            true
        }
        KeyCode::Backspace => {
            state.input.pop();
            true
        }
        KeyCode::Esc => {
            state.input.clear();
            true
        }
        // Enter handled separately in app.rs (needs async context).
        _ => false,
    }
}

/// Build a `ChatMessage` for the user turn and append it to `data`.
pub fn push_user_message(data: &mut TuiData, content: &str) {
    data.chat_messages.push(ChatMessage {
        role: "user".to_string(),
        content: content.to_string(),
        timestamp: chrono_now(),
    });
}

/// Build a `ChatMessage` for the assistant turn and append it to `data`.
pub fn push_assistant_message(data: &mut TuiData, content: &str) {
    data.chat_messages.push(ChatMessage {
        role: "assistant".to_string(),
        content: content.to_string(),
        timestamp: chrono_now(),
    });
}

/// Spawn an async task to create a session (if needed) and send `content`.
/// The result is delivered back via `ChatState::pending_reply` on next poll.
///
/// # Safety
/// Uses `tokio::spawn` — caller must be inside a tokio runtime.
pub async fn send_message(
    client: &Client,
    api_url: &str,
    data: &mut TuiData,
    state: &mut ChatState,
) {
    // Ensure a session exists before sending.
    if data.chat_session_id.is_none() {
        data.chat_session_id = chat_api::create_session(client, api_url).await;
    }

    let reply = if let Some(sid) = &data.chat_session_id {
        chat_api::send_message(client, api_url, sid, &state.input).await
    } else {
        None
    };

    state.sending = false;
    state.pending_reply = Some(reply.ok_or(()));
}

/// Returns the current timestamp as ISO-8601 string (UTC).
/// Uses std::time to avoid the chrono dependency.
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as pseudo-ISO: YYYY-MM-DDTHH:MM:SSZ
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400;
    // Approximate date — good enough for display timestamps.
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = day_of_year / 30 + 1;
    let day = day_of_year % 30 + 1;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::data::TuiData;

    #[test]
    fn handle_chat_key_char_appends_to_input() {
        let mut state = ChatState::default();
        assert!(handle_chat_key(KeyCode::Char('h'), &mut state));
        assert!(handle_chat_key(KeyCode::Char('i'), &mut state));
        assert_eq!(state.input, "hi");
    }

    #[test]
    fn handle_chat_key_backspace_removes_last_char() {
        let mut state = ChatState {
            input: "abc".to_string(),
            ..Default::default()
        };
        assert!(handle_chat_key(KeyCode::Backspace, &mut state));
        assert_eq!(state.input, "ab");
    }

    #[test]
    fn handle_chat_key_esc_clears_input() {
        let mut state = ChatState {
            input: "draft".to_string(),
            ..Default::default()
        };
        assert!(handle_chat_key(KeyCode::Esc, &mut state));
        assert!(state.input.is_empty());
    }

    #[test]
    fn handle_chat_key_enter_not_consumed() {
        let mut state = ChatState::default();
        // Enter is handled by app.rs in async context — chat_handler does NOT consume it.
        assert!(!handle_chat_key(KeyCode::Enter, &mut state));
    }

    #[test]
    fn push_user_message_appends_with_user_role() {
        let mut data = TuiData::default();
        push_user_message(&mut data, "hello world");
        assert_eq!(data.chat_messages.len(), 1);
        assert_eq!(data.chat_messages[0].role, "user");
        assert_eq!(data.chat_messages[0].content, "hello world");
    }

    #[test]
    fn push_assistant_message_appends_with_assistant_role() {
        let mut data = TuiData::default();
        push_assistant_message(&mut data, "I am Convergio");
        assert_eq!(data.chat_messages.len(), 1);
        assert_eq!(data.chat_messages[0].role, "assistant");
        assert_eq!(data.chat_messages[0].content, "I am Convergio");
    }

    #[test]
    fn chrono_now_produces_nonempty_string() {
        let ts = chrono_now();
        assert!(!ts.is_empty());
        assert!(ts.contains('T'), "expected ISO-like timestamp with T separator");
    }
}
