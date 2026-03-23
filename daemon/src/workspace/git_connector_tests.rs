// Tests for git_connector module — split to stay under 250 lines per file.
// Why: Plan 706 T3-01; verifies GitError, async methods, git ops, and trait contract.
use super::*;
use std::fs;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn merge_method_default_is_squash() {
    assert!(matches!(MergeMethod::default(), MergeMethod::Squash));
}

#[test]
fn pr_readiness_construction() {
    let r = PrReadiness {
        mergeable: true,
        ci_passed: true,
        pending_checks: 0,
        unresolved_threads: 0,
        review_status: "clean".into(),
    };
    assert!(r.mergeable && r.ci_passed);
}

#[test]
fn run_git_in_temp_repo() {
    let dir = tempdir().unwrap();
    run_git(dir.path(), &["init"]).unwrap();
    run_git(dir.path(), &["config", "user.email", "test@example.com"]).unwrap();
    run_git(dir.path(), &["config", "user.name", "Test"]).unwrap();
    fs::write(dir.path().join("test.txt"), "hello").unwrap();
    run_git(dir.path(), &["add", "."]).unwrap();
    run_git(dir.path(), &["commit", "-m", "init"]).unwrap();
    let sha = run_git(dir.path(), &["rev-parse", "HEAD"]).unwrap();
    assert_eq!(sha.len(), 40);
}

#[test]
fn commit_returns_sha() {
    let dir = tempdir().unwrap();
    run_git(dir.path(), &["init"]).unwrap();
    run_git(dir.path(), &["config", "user.email", "test@example.com"]).unwrap();
    run_git(dir.path(), &["config", "user.name", "Test"]).unwrap();
    fs::write(dir.path().join("file.txt"), "content").unwrap();
    let connector = GitHubConnector { github_token: "fake".into() };
    let sha = connector.commit(dir.path(), "test commit").unwrap();
    assert_eq!(sha.len(), 40);
}

#[test]
fn pr_info_fields_accessible() {
    let info = PrInfo { number: 42, url: "https://github.com/example/repo/pull/42".into() };
    assert_eq!(info.number, 42);
    assert!(!info.url.is_empty());
}

#[test]
fn merge_method_variants_serialize() {
    let squash = serde_json::to_string(&MergeMethod::Squash).unwrap();
    let merge = serde_json::to_string(&MergeMethod::Merge).unwrap();
    let rebase = serde_json::to_string(&MergeMethod::Rebase).unwrap();
    assert!(squash.contains("Squash") && merge.contains("Merge") && rebase.contains("Rebase"));
}

#[test]
fn run_git_invalid_dir_returns_error() {
    assert!(run_git(std::path::Path::new("/nonexistent_path_xyz"), &["status"]).is_err());
}

#[test]
fn git_error_api_display() {
    let e = GitError::Api { status: 422, body: "Unprocessable Entity".into() };
    let msg = e.to_string();
    assert!(msg.contains("422") && msg.contains("Unprocessable Entity"), "got: {msg}");
}

#[test]
fn git_error_parse_display() {
    assert!(GitError::Parse("bad json".into()).to_string().contains("bad json"));
}

#[test]
fn git_error_git_display() {
    assert!(GitError::Git("not a repo".into()).to_string().contains("not a repo"));
}

/// Verify async methods return futures — expect network error (not panic) within 100ms.
#[tokio::test]
async fn connector_create_pr_returns_future() {
    let c = GitHubConnector { github_token: "invalid-token".into() };
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        c.create_pr("org/repo", "feat/x", "main", "title", "body"),
    )
    .await;
    match result {
        Ok(Err(GitError::Request(_))) | Ok(Err(GitError::Api { .. })) | Err(_) => {}
        Ok(Ok(_)) => panic!("should not succeed with invalid token"),
        Ok(Err(e)) => panic!("unexpected error variant: {e}"),
    }
}

#[tokio::test]
async fn connector_merge_pr_returns_future() {
    let c = GitHubConnector { github_token: "invalid-token".into() };
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        c.merge_pr("org/repo", 1, MergeMethod::Squash),
    )
    .await;
    match result {
        Ok(Err(GitError::Request(_))) | Ok(Err(GitError::Api { .. })) | Err(_) => {}
        Ok(Ok(())) => panic!("should not succeed with invalid token"),
        Ok(Err(e)) => panic!("unexpected error variant: {e}"),
    }
}

#[tokio::test]
async fn connector_pr_readiness_returns_future() {
    let c = GitHubConnector { github_token: "invalid-token".into() };
    let result = tokio::time::timeout(
        Duration::from_millis(100),
        c.pr_readiness("org/repo", 1),
    )
    .await;
    match result {
        Ok(Err(GitError::Request(_))) | Ok(Err(GitError::Api { .. })) | Err(_) => {}
        Ok(Ok(_)) => panic!("should not succeed with invalid token"),
        Ok(Err(e)) => panic!("unexpected error variant: {e}"),
    }
}
