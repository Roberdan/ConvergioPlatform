use super::*;

#[test]
fn websocket_accept_matches_rfc_example() {
    let actual = websocket_accept("dGhlIHNhbXBsZSBub25jZQ==");
    assert_eq!(actual, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
}

#[test]
fn text_frame_writes_fin_opcode_and_payload() {
    let frame = text_frame("mesh");
    assert_eq!(frame[0], 0x81);
    assert_eq!(frame[1], 4);
    assert_eq!(&frame[2..], b"mesh");
}

// ── Additional tests ─────────────────────────────────────────────────────────

#[test]
fn text_frame_empty_payload() {
    let frame = text_frame("");
    assert_eq!(frame[0], 0x81); // FIN + text opcode
    assert_eq!(frame[1], 0); // zero length
    assert_eq!(frame.len(), 2);
}

#[test]
fn text_frame_126_byte_payload_uses_extended_length() {
    let payload = "a".repeat(126);
    let frame = text_frame(&payload);
    assert_eq!(frame[0], 0x81);
    assert_eq!(frame[1], 126); // extended 16-bit length marker
    let len = u16::from_be_bytes([frame[2], frame[3]]) as usize;
    assert_eq!(len, 126);
    assert_eq!(&frame[4..], payload.as_bytes());
}

#[test]
fn text_frame_125_byte_payload_fits_in_single_byte() {
    let payload = "b".repeat(125);
    let frame = text_frame(&payload);
    assert_eq!(frame[0], 0x81);
    assert_eq!(frame[1], 125);
    assert_eq!(&frame[2..], payload.as_bytes());
}

#[test]
fn text_frame_medium_payload_16bit_length() {
    let payload = "x".repeat(500);
    let frame = text_frame(&payload);
    assert_eq!(frame[1], 126);
    let len = u16::from_be_bytes([frame[2], frame[3]]) as usize;
    assert_eq!(len, 500);
}

#[test]
fn websocket_accept_different_key() {
    // Known vector: key "x3JJHMbDL1EzLkh9GBhXDw==" should produce a valid accept
    let result = websocket_accept("x3JJHMbDL1EzLkh9GBhXDw==");
    assert!(!result.is_empty());
    // Base64 encoded result should only contain valid chars
    assert!(result.chars().all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '='));
}

#[test]
fn websocket_accept_trims_whitespace() {
    let with_space = websocket_accept("  dGhlIHNhbXBsZSBub25jZQ==  ");
    let without = websocket_accept("dGhlIHNhbXBsZSBub25jZQ==");
    assert_eq!(with_space, without);
}
