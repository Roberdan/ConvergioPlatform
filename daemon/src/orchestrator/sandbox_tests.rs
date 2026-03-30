// Tests for sandbox: command validation, profile CRUD, audit log, settings generation.

use super::*;
use rusqlite::Connection;

fn test_profile(cmds: &[&str]) -> AgentProfile {
    AgentProfile {
        name: "test-agent".to_string(),
        filesystem_allowlist: vec![],
        network_allowlist: vec![],
        allowed_commands: cmds.iter().map(|s| s.to_string()).collect(),
    }
}

fn in_memory_conn() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    conn
}

#[test]
fn test_validate_command_allowed() {
    let profile = test_profile(&["cargo test", "git status"]);
    assert!(validate_command(&profile, "cargo test").is_ok());
    assert!(validate_command(&profile, "git status").is_ok());
}

#[test]
fn test_validate_command_blocked() {
    let profile = test_profile(&["cargo test"]);
    let err = validate_command(&profile, "rm -rf /").unwrap_err();
    assert!(err.contains("not permitted"));
    assert!(err.contains("rm -rf /"));
    assert!(err.contains("test-agent"));
}

#[test]
fn test_validate_command_exact_match_required() {
    // Only exact strings are matched; "cargo" does NOT grant "cargo build".
    let profile = test_profile(&["cargo"]);
    assert!(validate_command(&profile, "cargo").is_ok());
    let err = validate_command(&profile, "cargo build").unwrap_err();
    assert!(err.contains("not permitted"));
    assert!(err.contains("cargo build"));
}

#[test]
fn test_get_profile_missing() {
    let conn = in_memory_conn();
    assert!(get_profile(&conn, "no-such-agent").is_none());
}

#[test]
fn test_get_profile_roundtrip() {
    use rusqlite::params;
    let conn = in_memory_conn();
    conn.execute(
        "INSERT INTO agent_profiles (name, filesystem_allowlist, network_allowlist, allowed_commands) \
         VALUES (?1, ?2, ?3, ?4)",
        params![
            "builder",
            r#"["/workspace"]"#,
            r#"["github.com"]"#,
            r#"["cargo build","cargo test"]"#
        ],
    )
    .unwrap();

    let profile = get_profile(&conn, "builder").unwrap();
    assert_eq!(profile.name, "builder");
    assert_eq!(profile.filesystem_allowlist, vec![std::path::PathBuf::from("/workspace")]);
    assert_eq!(profile.network_allowlist, vec!["github.com"]);
    assert_eq!(
        profile.allowed_commands,
        vec!["cargo build".to_string(), "cargo test".to_string()]
    );
}

#[test]
fn test_log_violation_inserts_row() {
    let conn = in_memory_conn();
    log_violation(&conn, "agent-x", "rm -rf /", "command not in allowlist");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_generate_worktree_settings() {
    let profile = test_profile(&["cargo build", "git commit"]);
    let settings = generate_worktree_settings(&profile);
    let v: serde_json::Value = serde_json::from_str(&settings).unwrap();
    let allow = v["permissions"]["allow"].as_array().unwrap();
    assert_eq!(allow.len(), 2);
    assert_eq!(allow[0]["command"], "cargo build");
    assert_eq!(allow[1]["command"], "git commit");
}

#[test]
fn test_migrations_idempotent() {
    let conn = Connection::open_in_memory().unwrap();
    // Running twice must not error (CREATE TABLE IF NOT EXISTS)
    run_migrations(&conn).unwrap();
    run_migrations(&conn).unwrap();
}

// ── Sandbox delegation tests ────────────────────────────────────────────

#[test]
fn test_profile_blocks_delegate_command() {
    // A profile that does NOT include "delegate" must block it.
    let profile = test_profile(&["cargo test", "git status"]);
    let result = validate_command(&profile, "delegate");
    assert!(result.is_err(), "delegate must be blocked when not in allowlist");
    let msg = result.unwrap_err();
    assert!(msg.contains("delegate"), "error must mention the blocked command");
}

#[test]
fn test_no_profile_means_delegation_allowed() {
    // executor.rs: `profile = get_profile(&conn, peer)` — None means no restriction.
    // Verify that get_profile returns None for an unknown agent, which executor.rs
    // treats as backward-compatible allow (no profile = no sandbox).
    let conn = in_memory_conn();
    let profile = get_profile(&conn, "unknown-peer");
    assert!(
        profile.is_none(),
        "unknown agent must have no profile (backward compat = allow)"
    );
}

#[test]
fn test_profile_with_delegate_allows_delegation() {
    // When "delegate" is explicitly allowed, validate_command must pass.
    let profile = test_profile(&["cargo test", "delegate"]);
    assert!(validate_command(&profile, "delegate").is_ok());
}
