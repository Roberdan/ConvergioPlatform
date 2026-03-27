// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// TDD tests for kernel/telegram_voice.rs — written BEFORE implementation (RED phase).
// Inbound Telegram voice: download OGG, convert to WAV, transcribe, route, reply.
//
// This file is included by telegram_voice.rs via:
//   #[cfg(test)] #[path = "telegram_voice_tests.rs"] mod tests;
// so `super::` refers to telegram_voice module.

use super::{
    build_download_url, build_get_file_url, extract_voice_file_id,
    extract_voice_file_id_for_chat, resolve_ffmpeg, VoiceMessage,
};

// ----- ffmpeg resolution -------------------------------------------------------

/// On M1 Pro (or any machine with Homebrew) the daemon may run under launchd
/// without /opt/homebrew/bin in PATH.  resolve_ffmpeg must return a valid path.
#[test]
fn test_resolve_ffmpeg_returns_a_path() {
    // resolve_ffmpeg returns Ok on any machine that has ffmpeg somewhere — if it
    // returns Err the binary is genuinely absent, which is also a valid outcome
    // (the error message instructs the user to install it).
    match resolve_ffmpeg() {
        Ok(p) => {
            let display = p.display().to_string();
            assert!(
                display.contains("ffmpeg"),
                "resolved path should contain 'ffmpeg', got: {display}"
            );
        }
        Err(e) => {
            // ffmpeg absent on this CI machine — verify the error is helpful.
            assert!(
                e.contains("brew install ffmpeg") || e.contains("not found"),
                "error should guide installation, got: {e}"
            );
        }
    }
}

/// Absolute candidate paths must be probed before the bare "ffmpeg" fallback.
/// On Homebrew ARM the function must return /opt/homebrew/bin/ffmpeg when it exists.
#[test]
#[cfg(target_os = "macos")]
fn test_resolve_ffmpeg_prefers_homebrew_arm_path() {
    let result = resolve_ffmpeg();
    // If the Homebrew ARM path exists it must be chosen first.
    let homebrew_arm = std::path::Path::new("/opt/homebrew/bin/ffmpeg");
    if homebrew_arm.exists() {
        assert_eq!(result.unwrap(), homebrew_arm);
    } else {
        // Path absent on this machine — result is either another absolute path or Err.
        // Either is acceptable; we just verify resolve_ffmpeg doesn't panic.
        drop(result);
    }
}

// ----- URL construction -------------------------------------------------------

#[test]
fn test_build_get_file_url() {
    let url = build_get_file_url("TOKEN123", "file_id_abc", "https://api.telegram.org");
    assert_eq!(
        url,
        "https://api.telegram.org/botTOKEN123/getFile?file_id=file_id_abc"
    );
}

#[test]
fn test_build_download_url() {
    let url = build_download_url("TOKEN123", "voice/abc.oga", "https://api.telegram.org");
    assert_eq!(
        url,
        "https://api.telegram.org/file/botTOKEN123/voice/abc.oga"
    );
}

// ----- VoiceMessage extraction -----------------------------------------------

#[test]
fn test_extract_voice_message_struct() {
    let msg = VoiceMessage {
        chat_id: 42,
        file_id: "AgACAgIABQAD".to_string(),
        duration_secs: 3,
    };
    assert_eq!(msg.file_id, "AgACAgIABQAD");
    assert_eq!(msg.chat_id, 42);
    assert_eq!(msg.duration_secs, 3);
}

#[test]
fn test_extract_voice_file_id_from_json() {
    // Simulate Telegram wire format for a voice update.
    let json = serde_json::json!({
        "update_id": 999,
        "message": {
            "chat": { "id": 42 },
            "voice": {
                "file_id": "AgACAgIABQAD",
                "duration": 5
            }
        }
    });
    let result = extract_voice_file_id(&json);
    assert!(result.is_some(), "expected Some for voice message");
    let vm = result.unwrap();
    assert_eq!(vm.file_id, "AgACAgIABQAD");
    assert_eq!(vm.chat_id, 42);
    assert_eq!(vm.duration_secs, 5);
}

#[test]
fn test_extract_voice_file_id_no_voice_returns_none() {
    let json = serde_json::json!({
        "update_id": 1000,
        "message": {
            "chat": { "id": 42 },
            "text": "hello"
        }
    });
    let result = extract_voice_file_id(&json);
    assert!(result.is_none(), "text message must not extract as voice");
}

#[test]
fn test_extract_voice_file_id_missing_message_returns_none() {
    let json = serde_json::json!({ "update_id": 1003 });
    assert!(extract_voice_file_id(&json).is_none());
}

#[test]
fn test_extract_voice_file_id_wrong_chat_returns_none() {
    // Security: only process voice from the authorised chat_id.
    let json = serde_json::json!({
        "update_id": 1001,
        "message": {
            "chat": { "id": 999 },
            "voice": { "file_id": "abc", "duration": 2 }
        }
    });
    // When filtered against chat_id=42, must return None.
    let result = extract_voice_file_id_for_chat(&json, 42);
    assert!(result.is_none(), "voice from wrong chat must be ignored");
}

#[test]
fn test_extract_voice_file_id_for_correct_chat() {
    let json = serde_json::json!({
        "update_id": 1002,
        "message": {
            "chat": { "id": 42 },
            "voice": { "file_id": "abc", "duration": 2 }
        }
    });
    let result = extract_voice_file_id_for_chat(&json, 42);
    assert!(result.is_some());
    assert_eq!(result.unwrap().file_id, "abc");
}

#[test]
fn test_extract_voice_duration_defaults_to_zero() {
    // duration field absent → defaults to 0
    let json = serde_json::json!({
        "update_id": 1004,
        "message": {
            "chat": { "id": 42 },
            "voice": { "file_id": "xyz" }
        }
    });
    let result = extract_voice_file_id(&json).unwrap();
    assert_eq!(result.duration_secs, 0);
}
