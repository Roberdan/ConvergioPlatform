// Why: Extracted from telegram.rs to keep files under 250 lines.
// Telegram Bot API request/response types used by TelegramAdapter.

use serde::{Deserialize, Serialize};

/// Telegram API response wrapper.
#[derive(Debug, Deserialize)]
pub(super) struct TelegramResponse<T> {
    pub ok: bool,
    pub result: Option<T>,
    pub description: Option<String>,
}

/// Telegram User from API.
// Fields are deserialized from Telegram API response; not all are used in current logic.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(super) struct TelegramUser {
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
    pub username: Option<String>,
}

/// Telegram Chat from API.
#[derive(Debug, Deserialize, Serialize)]
pub(super) struct TelegramChat {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: Option<String>,
}

/// Telegram Message from API.
// Fields are deserialized from Telegram API response; not all are used in current logic.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(super) struct TelegramMsg {
    pub message_id: i64,
    pub date: i64,
    pub chat: TelegramChat,
    pub from: Option<TelegramUser>,
    pub text: Option<String>,
}

/// Telegram Update from getUpdates.
#[derive(Debug, Deserialize)]
pub(super) struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramMsg>,
}

/// Send message request body.
#[derive(Debug, Serialize)]
pub(super) struct SendMessageBody {
    pub chat_id: i64,
    pub text: String,
    pub parse_mode: String,
}
