use super::*;
use std::fs;
use tempfile::TempDir;

fn make_lock_file(dir: &TempDir, name: &str, age_secs: u64) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, b"locked").unwrap();
    if age_secs == 0 {
        // fresh — leave mtime as-is
    }
    path
}

#[test]
fn reap_report_is_clean_when_empty() {
    let report = ReapReport::default();
    assert!(report.is_clean());
}

#[test]
fn reap_report_not_clean_with_action() {
    let mut report = ReapReport::default();
    report.actions.push(ReapAction {
        kind: ReapKind::LockFile,
        target: "/tmp/test.lock".into(),
        reason: "stale".into(),
    });
    assert!(!report.is_clean());
}

#[test]
fn reap_report_not_clean_with_error() {
    let mut report = ReapReport::default();
    report.errors.push("some error".into());
    assert!(!report.is_clean());
}

#[test]
fn reap_lock_files_missing_dir_returns_error() {
    let report = reap_lock_files("/nonexistent_dir_xyz/tmp", Duration::from_secs(1), true);
    assert!(!report.errors.is_empty(), "expected error for missing dir");
}

#[test]
fn reap_lock_files_fresh_file_not_reaped() {
    let dir = TempDir::new().unwrap();
    make_lock_file(&dir, "agent.lock", 0);
    let report = reap_lock_files(dir.path().to_str().unwrap(), Duration::from_secs(1), true);
    assert!(
        report.actions.is_empty(),
        "fresh lock should not be reaped: {:?}",
        report.actions
    );
}

#[test]
fn reap_lock_files_ignores_non_lock_files() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("state.json");
    fs::write(&path, b"data").unwrap();
    let report = reap_lock_files(dir.path().to_str().unwrap(), Duration::from_millis(1), true);
    assert!(
        report.actions.is_empty(),
        "non-lock file should be ignored: {:?}",
        report.actions
    );
}

#[test]
fn reap_action_fields_are_correct() {
    let action = ReapAction {
        kind: ReapKind::Worktree,
        target: "/tmp/wt".into(),
        reason: "stale>24h".into(),
    };
    assert_eq!(action.kind, ReapKind::Worktree);
    assert_eq!(action.target, "/tmp/wt");
    assert!(action.reason.contains("stale"));
}

#[test]
fn reap_worktrees_nonexistent_dir_returns_error() {
    let report = reap_worktrees("/nonexistent_path_xyz_worktree", Duration::from_secs(1), true);
    assert!(!report.errors.is_empty(), "expected error for nonexistent path");
}

#[test]
fn reap_merged_branches_nonexistent_dir_returns_error() {
    let report = reap_merged_branches("/nonexistent_path_xyz_branch", true);
    assert!(!report.errors.is_empty(), "expected error for nonexistent path");
}
