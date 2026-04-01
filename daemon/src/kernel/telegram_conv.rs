// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Per-chat sliding window conversation memory for Telegram integration.
// Stores the last N (user, assistant) exchange pairs keyed by chat_id.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

/// Maximum number of (user, assistant) exchange pairs retained per chat.
const MAX_EXCHANGES: usize = 5;

/// Single exchange: user message + assistant response.
#[derive(Debug, Clone)]
pub struct Exchange {
    pub user: String,
    pub assistant: String,
}

/// Per-chat sliding window buffer.
#[derive(Debug, Default)]
struct ChatBuffer {
    exchanges: VecDeque<Exchange>,
}

impl ChatBuffer {
    fn push(&mut self, exchange: Exchange) {
        if self.exchanges.len() >= MAX_EXCHANGES {
            self.exchanges.pop_front();
        }
        self.exchanges.push_back(exchange);
    }

    fn history(&self) -> Vec<Exchange> {
        self.exchanges.iter().cloned().collect()
    }
}

/// Global conversation memory, keyed by Telegram chat_id.
/// Uses a Mutex because writes are infrequent (one per message pair)
/// and contention is near-zero for single-user Telegram bots.
static CONV_MEMORY: std::sync::LazyLock<Mutex<HashMap<i64, ChatBuffer>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Record a completed exchange for a chat.
pub fn record_exchange(chat_id: i64, user: String, assistant: String) {
    if let Ok(mut map) = CONV_MEMORY.lock() {
        map.entry(chat_id)
            .or_default()
            .push(Exchange { user, assistant });
    }
}

/// Retrieve conversation history for a chat (oldest first).
pub fn get_history(chat_id: i64) -> Vec<Exchange> {
    match CONV_MEMORY.lock() {
        Ok(map) => map.get(&chat_id).map_or_else(Vec::new, |b| b.history()),
        Err(_) => Vec::new(),
    }
}

/// Format conversation history as ChatML turns for prompt injection.
/// Returns an empty string when there is no history.
pub fn format_history_chatml(chat_id: i64) -> String {
    let history = get_history(chat_id);
    if history.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for ex in &history {
        out.push_str(&format!(
            "<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n{}<|im_end|>\n",
            ex.user, ex.assistant
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_respects_max_capacity() {
        let mut buf = ChatBuffer::default();
        for i in 0..7 {
            buf.push(Exchange {
                user: format!("q{i}"),
                assistant: format!("a{i}"),
            });
        }
        let h = buf.history();
        assert_eq!(h.len(), MAX_EXCHANGES);
        assert_eq!(h[0].user, "q2");
        assert_eq!(h[4].user, "q6");
    }

    #[test]
    fn empty_history_returns_empty_vec() {
        let buf = ChatBuffer::default();
        assert!(buf.history().is_empty());
    }

    #[test]
    fn format_chatml_empty_when_no_history() {
        // Use a chat_id unlikely to collide with other tests.
        let result = format_history_chatml(999_999);
        assert!(result.is_empty());
    }

    #[test]
    fn record_and_retrieve_round_trip() {
        let cid = 111_222;
        record_exchange(cid, "ciao".into(), "buongiorno".into());
        let h = get_history(cid);
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].user, "ciao");
        assert_eq!(h[0].assistant, "buongiorno");
    }

    #[test]
    fn format_chatml_includes_turns() {
        let cid = 333_444;
        record_exchange(cid, "domanda".into(), "risposta".into());
        let chatml = format_history_chatml(cid);
        assert!(chatml.contains("<|im_start|>user"));
        assert!(chatml.contains("domanda"));
        assert!(chatml.contains("risposta"));
        assert!(chatml.contains("<|im_end|>"));
    }
}
