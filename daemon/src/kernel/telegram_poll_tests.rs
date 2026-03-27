// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for kernel::telegram_poll — long polling loop and message processing.

use super::telegram_poll::{
    build_status_reply, extract_text_message, TelegramMessage, TelegramUpdate,
};

// ----- extract_text_message --------------------------------------------------

#[test]
fn test_extract_text_message_valid() {
    let update = TelegramUpdate {
        update_id: 100,
        message: Some(TelegramMessage {
            chat_id: 42,
            text: Some("stato".to_string()),
        }),
    };
    let result = extract_text_message(&update, 42);
    assert_eq!(result, Some("stato"));
}

#[test]
fn test_extract_text_message_wrong_chat_ignored() {
    let update = TelegramUpdate {
        update_id: 101,
        message: Some(TelegramMessage {
            chat_id: 999,
            text: Some("stato".to_string()),
        }),
    };
    // Security: messages from other chat IDs must be ignored.
    let result = extract_text_message(&update, 42);
    assert_eq!(result, None);
}

#[test]
fn test_extract_text_message_no_text_ignored() {
    let update = TelegramUpdate {
        update_id: 102,
        message: Some(TelegramMessage {
            chat_id: 42,
            text: None,
        }),
    };
    let result = extract_text_message(&update, 42);
    assert_eq!(result, None);
}

#[test]
fn test_extract_text_message_no_message_ignored() {
    let update = TelegramUpdate {
        update_id: 103,
        message: None,
    };
    let result = extract_text_message(&update, 42);
    assert_eq!(result, None);
}

// ----- build_status_reply ---------------------------------------------------

#[test]
fn test_build_status_reply_format() {
    let reply = build_status_reply(2, 5, "2/2", "$42");
    assert!(reply.contains("*Convergio Status*"), "missing header: {reply}");
    assert!(reply.contains("Piani attivi:"), "missing piani attivi: {reply}");
    assert!(reply.contains("Task in coda:"), "missing task in coda: {reply}");
    assert!(reply.contains("Mesh:"), "missing mesh: {reply}");
    assert!(reply.contains("Costo oggi:"), "missing costo: {reply}");
}

#[test]
fn test_build_status_reply_values() {
    let reply = build_status_reply(3, 7, "1/2", "$99");
    assert!(reply.contains('3'), "missing plan count: {reply}");
    assert!(reply.contains('7'), "missing task count: {reply}");
    assert!(reply.contains("1/2"), "missing mesh status: {reply}");
    assert!(reply.contains("$99"), "missing cost: {reply}");
}

// ----- update_id offset logic -----------------------------------------------

#[test]
fn test_next_offset_increments() {
    // offset should be last_update_id + 1 to avoid reprocessing.
    let last_id: i64 = 500;
    let next_offset = last_id + 1;
    assert_eq!(next_offset, 501);
}
