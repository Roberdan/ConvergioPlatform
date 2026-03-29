// Tests for recover_escalation: classify_problem, escalate_to_ali, triage_and_recover.
// All tests use dry_run=true to skip external commands.

use super::recover_escalation::{classify_problem, escalate_to_ali, triage_and_recover, ProblemClass};
use super::recover::RecoveryConfig;

fn dry_cfg() -> RecoveryConfig {
    RecoveryConfig { ntfy_topic: "test".to_string(), channels: vec![], dry_run: true, db_path: None }
}

// ----- classify_problem -------------------------------------------------------

#[test]
fn classify_daemon_crash() {
    assert_eq!(classify_problem("daemon_crash"), ProblemClass::DaemonCrash);
    assert_eq!(classify_problem("daemon_health check failed"), ProblemClass::DaemonCrash);
}

#[test]
fn classify_telegram_poll() {
    assert_eq!(classify_problem("telegram_poll timeout"), ProblemClass::TelegramPollDead);
}

#[test]
fn classify_db_locked() {
    assert_eq!(classify_problem("db_locked"), ProblemClass::DbLocked);
    assert_eq!(classify_problem("database is locked"), ProblemClass::DbLocked);
}

#[test]
fn classify_stale_worktrees() {
    assert_eq!(classify_problem("stale_worktree detected"), ProblemClass::StaleWorktrees);
}

#[test]
fn classify_high_fd() {
    assert_eq!(classify_problem("high_fd usage"), ProblemClass::HighFdCount);
    assert_eq!(classify_problem("file descriptor limit"), ProblemClass::HighFdCount);
}

#[test]
fn classify_unknown() {
    assert_eq!(classify_problem("some random weird error"), ProblemClass::Unknown);
    assert_eq!(classify_problem(""), ProblemClass::Unknown);
}

// ----- escalate_to_ali --------------------------------------------------------

#[tokio::test]
async fn escalate_dry_run_returns_plan_name() {
    let cfg = dry_cfg();
    let result = escalate_to_ali("disk full", "no space left on device", &cfg).await;
    // dry_run: no external command, should return Ok with the plan name
    assert!(result.is_ok(), "expected Ok, got: {result:?}");
    let plan_name = result.unwrap();
    assert!(plan_name.contains("disk full"), "plan name should contain problem: {plan_name}");
}

#[tokio::test]
async fn escalate_plan_name_truncated_at_80() {
    let cfg = dry_cfg();
    let long_problem = "x".repeat(100);
    let result = escalate_to_ali(&long_problem, "details", &cfg).await;
    assert!(result.is_ok());
    let plan_name = result.unwrap();
    // "Jarvis Alert: " = 15 chars; total <= 80
    assert!(plan_name.len() <= 80, "plan name too long: {} chars", plan_name.len());
}

// ----- triage_and_recover -----------------------------------------------------

#[tokio::test]
async fn triage_daemon_crash_dry_run() {
    let cfg = dry_cfg();
    let result = triage_and_recover("daemon_crash", "crash details", &cfg).await;
    assert!(result.is_ok(), "expected Ok, got: {result:?}");
    let action = result.unwrap();
    assert!(action.contains("start.sh") || action.contains("daemon"), "action={action}");
}

#[tokio::test]
async fn triage_telegram_poll_dry_run() {
    let cfg = dry_cfg();
    let result = triage_and_recover("telegram_poll", "poll died", &cfg).await;
    assert!(result.is_ok());
    let action = result.unwrap();
    assert!(action.contains("monitor") || action.contains("telegram"), "action={action}");
}

#[tokio::test]
async fn triage_db_locked_dry_run() {
    let cfg = dry_cfg();
    let result = triage_and_recover("db_locked", "sqlite locked", &cfg).await;
    assert!(result.is_ok());
    let action = result.unwrap();
    assert!(action.contains("busy_timeout") || action.contains("db"), "action={action}");
}

#[tokio::test]
async fn triage_stale_worktrees_dry_run() {
    let cfg = dry_cfg();
    let result = triage_and_recover("stale_worktree", "old worktrees", &cfg).await;
    assert!(result.is_ok());
    let action = result.unwrap();
    assert!(action.contains("prune") || action.contains("worktree"), "action={action}");
}

#[tokio::test]
async fn triage_high_fd_dry_run() {
    let cfg = dry_cfg();
    let result = triage_and_recover("high_fd", "fd 1024/1024", &cfg).await;
    assert!(result.is_ok());
    let action = result.unwrap();
    assert!(action.contains("fd") || action.contains("restart") || action.contains("high"), "action={action}");
}

#[tokio::test]
async fn triage_unknown_escalates_to_ali() {
    let cfg = dry_cfg();
    let result = triage_and_recover("something weird", "no idea", &cfg).await;
    assert!(result.is_ok(), "expected Ok, got: {result:?}");
    let action = result.unwrap();
    assert!(
        action.contains("Jarvis Alert") || action.contains("ali") || action.contains("escalat"),
        "expected Ali escalation in action: {action}"
    );
}
