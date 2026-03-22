// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Tests for cli_support (checkpoint, lock, review).
use super::*;

#[test]
fn checkpoint_save_variant() {
    let cmd = CheckpointCommands::Save { plan_id: 685, human: false, api_url: "http://localhost:8420".into() };
    assert!(matches!(cmd, CheckpointCommands::Save { plan_id: 685, .. }));
}

#[test]
fn checkpoint_restore_variant() {
    let cmd = CheckpointCommands::Restore { plan_id: 42, human: true, api_url: "http://localhost:8420".into() };
    assert!(matches!(cmd, CheckpointCommands::Restore { plan_id: 42, .. }));
}

#[test]
fn lock_acquire_variant() {
    let cmd = LockCommands::Acquire { file_path: "src/main.rs".into(), task_id: 8796, agent: "task-executor".into(), human: false, api_url: "http://localhost:8420".into() };
    assert!(matches!(cmd, LockCommands::Acquire { task_id: 8796, .. }));
}

#[test]
fn lock_release_variant() {
    let cmd = LockCommands::Release { file_path: "src/main.rs".into(), task_id: 8796, human: false, api_url: "http://localhost:8420".into() };
    assert!(matches!(cmd, LockCommands::Release { task_id: 8796, .. }));
}

#[test]
fn lock_list_variant() {
    let cmd = LockCommands::List { human: true, api_url: "http://localhost:8420".into() };
    assert!(matches!(cmd, LockCommands::List { human: true, .. }));
}

#[test]
fn review_register_variant() {
    let cmd = ReviewCommands::Register { plan_id: 685, reviewer_agent: "plan-reviewer".into(), verdict: "approved".into(), suggestions: None, human: false, api_url: "http://localhost:8420".into() };
    assert!(matches!(cmd, ReviewCommands::Register { plan_id: 685, .. }));
}

#[test]
fn review_check_variant() {
    let cmd = ReviewCommands::Check { plan_id: 100, human: false, api_url: "http://localhost:8420".into() };
    assert!(matches!(cmd, ReviewCommands::Check { plan_id: 100, .. }));
}

#[test]
fn review_reset_variant() {
    let cmd = ReviewCommands::Reset { plan_id: 1, human: true, api_url: "http://localhost:8420".into() };
    assert!(matches!(cmd, ReviewCommands::Reset { plan_id: 1, .. }));
}

#[test]
fn checkpoint_save_body() {
    let body = serde_json::json!({ "plan_id": 685_i64 });
    assert_eq!(body["plan_id"], 685);
}

#[test]
fn lock_acquire_body() {
    let body = serde_json::json!({"file_path":"src/main.rs","task_id":8796_i64,"agent":"task-executor"});
    assert_eq!(body["task_id"], 8796);
}

#[test]
fn review_register_body() {
    let body = serde_json::json!({"plan_id":685_i64,"reviewer_agent":"plan-reviewer","verdict":"approved","suggestions":null});
    assert_eq!(body["verdict"], "approved");
}
