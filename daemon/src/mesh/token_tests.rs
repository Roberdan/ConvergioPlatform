// Tests for mesh token signing and verification.

use super::*;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rusqlite::Connection;

const SECRET: &[u8] = b"test-secret-key-for-hmac";

fn setup_db() -> Connection {
    let db = Connection::open_in_memory().expect("in-memory DB");
    init_token_db(&db).expect("init_token_db");
    db
}

fn make_token(_db: &Connection) -> (String, String) {
    let token = generate_token(
        SECRET,
        "coordinator",
        vec!["claude".into(), "copilot".into()],
        "100.64.0.1",
        60,
    )
    .expect("generate_token");

    let payload_b64 = token.split('.').next().unwrap();
    let payload_bytes = URL_SAFE_NO_PAD.decode(payload_b64).unwrap();
    let payload: TokenPayload = serde_json::from_slice(&payload_bytes).unwrap();

    (token, payload.nonce)
}

#[test]
fn token_has_dot_separator() {
    let db = setup_db();
    let (token, _) = make_token(&db);
    assert!(token.contains('.'), "token must contain '.' separator");
    let parts: Vec<&str> = token.splitn(2, '.').collect();
    assert_eq!(parts.len(), 2);
    URL_SAFE_NO_PAD
        .decode(parts[0])
        .expect("payload valid base64url");
    URL_SAFE_NO_PAD
        .decode(parts[1])
        .expect("signature valid base64url");
}

#[test]
fn valid_token_validates_and_returns_payload() {
    let db = setup_db();
    let (token, _) = make_token(&db);
    let payload = validate_token(&token, SECRET, &db).expect("should validate");
    assert_eq!(payload.role, "coordinator");
    assert!(payload.capabilities.contains(&"claude".to_string()));
    assert_eq!(payload.coordinator_ip, "100.64.0.1");
}

#[test]
fn token_is_single_use() {
    let db = setup_db();
    let (token, _) = make_token(&db);
    validate_token(&token, SECRET, &db).expect("first use ok");
    let err = validate_token(&token, SECRET, &db).expect_err("second use must fail");
    assert!(matches!(err, TokenError::AlreadyUsed));
}

#[test]
fn wrong_secret_is_rejected() {
    let db = setup_db();
    let (token, _) = make_token(&db);
    let err = validate_token(&token, b"wrong-secret", &db).expect_err("wrong secret must fail");
    assert!(matches!(err, TokenError::InvalidSignature));
}

#[test]
fn expired_token_is_rejected() {
    let db = setup_db();
    let token = generate_token(SECRET, "worker", vec![], "100.64.0.2", -1)
        .expect("generate expired token");
    let err = validate_token(&token, SECRET, &db).expect_err("expired must fail");
    assert!(matches!(err, TokenError::Expired));
}

#[test]
fn revoked_token_is_rejected() {
    let db = setup_db();
    let (token, nonce) = make_token(&db);
    revoke_token(&nonce, &db).expect("revoke");
    let err = validate_token(&token, SECRET, &db).expect_err("revoked must fail");
    assert!(matches!(err, TokenError::AlreadyUsed));
}

#[test]
fn revoke_is_idempotent() {
    let db = setup_db();
    let (_, nonce) = make_token(&db);
    revoke_token(&nonce, &db).expect("first revoke");
    revoke_token(&nonce, &db).expect("second revoke must not error");
}

#[test]
fn malformed_token_is_rejected() {
    let db = setup_db();
    let err = validate_token("notavalidtoken", SECRET, &db).expect_err("malformed must fail");
    assert!(matches!(
        err,
        TokenError::MalformedToken | TokenError::InvalidSignature
    ));
}

#[test]
fn payload_tampering_detected() {
    let db = setup_db();
    let (token, _) = make_token(&db);
    let parts: Vec<&str> = token.splitn(2, '.').collect();
    let fake = r#"{"role":"admin","capabilities":[],"coordinator_ip":"1.1.1.1","nonce":"fake","expires_at":"2099-01-01T00:00:00Z"}"#;
    let fake_b64 = URL_SAFE_NO_PAD.encode(fake.as_bytes());
    let tampered = format!("{}.{}", fake_b64, parts[1]);
    let err = validate_token(&tampered, SECRET, &db).expect_err("tamper must fail");
    assert!(matches!(err, TokenError::InvalidSignature));
}

#[test]
fn signature_tampering_detected() {
    let db = setup_db();
    let (token, _) = make_token(&db);
    let parts: Vec<&str> = token.splitn(2, '.').collect();
    let mut bad_sig = parts[1].to_string();
    let last = bad_sig.pop().unwrap_or('A');
    bad_sig.push(if last == 'A' { 'B' } else { 'A' });
    let tampered = format!("{}.{}", parts[0], bad_sig);
    let err = validate_token(&tampered, SECRET, &db).expect_err("sig tamper must fail");
    assert!(matches!(
        err,
        TokenError::InvalidSignature | TokenError::MalformedToken
    ));
}
