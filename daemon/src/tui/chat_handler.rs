// Chat input handling and persistent Claude session — extracted from app.rs.

use crossterm::event::KeyCode;

use crate::tui::claude_session::{ChatEvent, ClaudeSession};

// Re-export message helpers so callers keep the same import path.
pub use crate::tui::chat_messages::{
    enrich_with_context, push_assistant_message, push_user_message,
};

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
        KeyCode::Home => {
            state.scroll_offset = u16::MAX; // scroll to top
            true
        }
        KeyCode::End => {
            state.scroll_offset = 0; // scroll to bottom
            true
        }
        // Enter handled separately in app.rs (needs async context).
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn chat_state_default_has_no_session() {
        let state = ChatState::default();
        assert!(state.session.is_none());
        assert!(!state.sending);
        assert!(!state.streaming);
    }
}
