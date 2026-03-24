// Chat input handling and persistent Claude session — extracted from app.rs.

use crossterm::event::KeyCode;

use crate::tui::claude_session::{ChatEvent, ClaudeSession};
use crate::tui::data::{ChatMessage, TuiData};

/// State related to the chat view, owned by TuiApp.
pub struct ChatState {
    pub input: String,
    pub sending: bool,
    /// Persistent Claude session (spawned lazily on first message).
    pub session: Option<ClaudeSession>,
    /// True while streaming a response (show partial text).
    pub streaming: bool,
    /// Manual scroll offset from bottom (0 = auto-scroll to bottom).
    pub scroll_offset: u16,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            input: String::new(),
            sending: false,
            session: None,
            streaming: false,
            scroll_offset: 0,
        }
    }
}

impl ChatState {
    /// Ensure the persistent session is running. Returns true if ready.
    pub fn ensure_session(&mut self) -> bool {
        // Check if existing session is alive.
        if let Some(ref mut s) = self.session {
            if s.is_alive() {
                return true;
            }
            tracing::warn!("claude session died, respawning");
            self.session = None;
        }
        // Spawn new session.
        match ClaudeSession::spawn() {
            Ok(session) => {
                tracing::info!("claude session spawned");
                self.session = Some(session);
                true
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to spawn claude session");
                false
            }
        }
    }

    /// Send a message through the persistent session.
    pub async fn send_to_session(&mut self, content: &str) -> bool {
        if !self.ensure_session() {
            return false;
        }
        if let Some(ref mut session) = self.session {
            match session.send(content).await {
                Ok(()) => true,
                Err(e) => {
                    tracing::error!(error = %e, "failed to send to claude");
                    false
                }
            }
        } else {
            false
        }
    }

    /// Poll for streaming events from the session (non-blocking).
    /// Returns events received this tick.
    pub fn poll_events(&mut self) -> Vec<ChatEvent> {
        let mut events = Vec::new();
        if let Some(ref mut session) = self.session {
            while let Ok(event) = session.event_rx.try_recv() {
                events.push(event);
            }
        }
        events
    }
}

/// Handle a key press while the Chat view is active.
/// Returns true if the key was consumed (prevents further dispatch).
pub fn handle_chat_key(code: KeyCode, state: &mut ChatState) -> bool {
    match code {
        KeyCode::Char(c) => {
            state.input.push(c);
            // Reset scroll to bottom when typing.
            state.scroll_offset = 0;
            true
        }
        KeyCode::Backspace => {
            state.input.pop();
            true
        }
        KeyCode::Esc => {
            state.input.clear();
            state.scroll_offset = 0;
            true
        }
        KeyCode::Up => {
            state.scroll_offset = state.scroll_offset.saturating_add(3);
            true
        }
        KeyCode::Down => {
            state.scroll_offset = state.scroll_offset.saturating_sub(3);
            true
        }
        KeyCode::PageUp => {
            state.scroll_offset = state.scroll_offset.saturating_add(20);
            true
        }
        KeyCode::PageDown => {
            state.scroll_offset = state.scroll_offset.saturating_sub(20);
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

/// Returns the current timestamp as ISO-8601 string (UTC).
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hour = (s / 3600) % 24;
    let days = s / 86400;
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
        assert!(ts.contains('T'), "expected ISO-like timestamp");
    }

    #[test]
    fn chat_state_default_has_no_session() {
        let state = ChatState::default();
        assert!(state.session.is_none());
        assert!(!state.sending);
        assert!(!state.streaming);
    }
}
