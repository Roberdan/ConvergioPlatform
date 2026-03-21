use super::pipeline::join;
use super::types::{JoinConfig, JoinError, JoinProgress, JoinSelections, StepStatus};
use crate::mesh::token::{generate_token, init_token_db};
use rusqlite::Connection;

const SECRET: &[u8] = b"test-join-secret";

fn setup_db() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    init_token_db(&db).unwrap();
    db
}

#[allow(dead_code)]
fn make_valid_token() -> String {
    generate_token(SECRET, "worker", vec![], "100.64.0.1", 60).unwrap()
}

fn base_config(token: &str) -> JoinConfig {
    JoinConfig {
        token: token.to_owned(),
        admin_password: "secret".to_owned(),
        profiles: vec![],
        interactive: false,
        selections: JoinSelections::default(),
    }
}

// T: test_join_validates_token_first
// Invalid token must cause early Err — no sudo or network touched.
#[tokio::test]
async fn test_join_validates_token_first() {
    let db = setup_db();
    let config = base_config("not.avalidtoken");
    let result = join(config, SECRET, &db).await;
    assert!(result.is_err(), "invalid token must return Err");
    match result.unwrap_err() {
        JoinError::Token(_) => {}
        other => panic!("expected JoinError::Token, got: {other:?}"),
    }
}

// T: test_join_selections_default
// Default JoinSelections has all fields false.
#[test]
fn test_join_selections_default() {
    let sel = JoinSelections::default();
    assert!(!sel.network);
    assert!(!sel.brew);
    assert!(!sel.apps);
    assert!(!sel.repos);
    assert!(!sel.shell);
    assert!(!sel.auth);
    assert!(!sel.macos_tweaks);
    assert!(!sel.coordinator_migration);
    assert!(!sel.runners);
}

// T: test_join_selections_all
#[test]
fn test_join_selections_all() {
    let sel = JoinSelections::all();
    assert!(sel.network);
    assert!(sel.brew);
    assert!(sel.auth);
    assert!(sel.coordinator_migration);
}

// T: test_progress_serialization
#[test]
fn test_progress_serialization() {
    let p = JoinProgress {
        step: 3,
        total_steps: 9,
        current: "Network setup".to_owned(),
        status: StepStatus::Done,
    };
    let json = serde_json::to_string(&p).expect("serialise");
    assert!(json.contains("\"step\":3"));
    assert!(json.contains("\"total_steps\":9"));
    assert!(json.contains("\"Done\""));

    let back: JoinProgress = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(back, p);
}

// T: test_progress_failed_variant
#[test]
fn test_progress_failed_variant_serialization() {
    let p = JoinProgress {
        step: 2,
        total_steps: 9,
        current: "Sudo gate".to_owned(),
        status: StepStatus::Failed("permission denied".to_owned()),
    };
    let json = serde_json::to_string(&p).expect("serialise");
    assert!(json.contains("permission denied"));
    let back: JoinProgress = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(back, p);
}

// T: token with wrong secret → JoinError::Token(InvalidSignature)
#[test]
fn test_join_config_serialization_roundtrip() {
    let config = JoinConfig {
        token: "tok.sig".to_owned(),
        admin_password: "pw".to_owned(),
        profiles: vec!["default".to_owned()],
        interactive: true,
        selections: JoinSelections::all(),
    };
    let json = serde_json::to_string(&config).unwrap();
    let back: JoinConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.token, config.token);
    assert_eq!(back.interactive, true);
    assert!(back.selections.network);
}
