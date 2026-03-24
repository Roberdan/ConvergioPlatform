// Additional ReleaseAgent tests — PR description and struct construction.
// Why: Plan 698 T4-01; split from release_agent_tests.rs to stay under 250 lines.

use crate::server::state_init::ConnPool;
use crate::workspace::events::{EventLogger, WorkspaceAction};
use crate::workspace::git_connector::{
    AsyncResult, GitConnector, GitError, MergeMethod, PrInfo, PrReadiness,
};
use crate::workspace::release_agent::ReleaseAgent;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::path::Path;

struct MockOk;

impl GitConnector for MockOk {
    fn commit(&self, _path: &Path, _message: &str) -> Result<String, GitError> {
        Ok("abc123".to_string())
    }
    fn push(&self, _path: &Path, _branch: &str, _fwl: bool) -> Result<(), GitError> {
        Ok(())
    }
    fn create_pr<'a>(
        &'a self,
        repo: &'a str,
        _branch: &'a str,
        _base: &'a str,
        _title: &'a str,
        _body: &'a str,
    ) -> AsyncResult<'a, PrInfo> {
        let repo = repo.to_string();
        Box::pin(async move {
            Ok(PrInfo {
                number: 42,
                url: format!("https://github.com/{repo}/pull/42"),
            })
        })
    }
    fn merge_pr<'a>(
        &'a self,
        _repo: &'a str,
        _pr_number: i64,
        _method: MergeMethod,
    ) -> AsyncResult<'a, ()> {
        Box::pin(async { Ok(()) })
    }
    fn pr_readiness<'a>(&'a self, _repo: &'a str, _pr_number: i64) -> AsyncResult<'a, PrReadiness> {
        Box::pin(async {
            Ok(PrReadiness {
                mergeable: true,
                ci_passed: true,
                pending_checks: 0,
                unresolved_threads: 0,
                review_status: "clean".into(),
            })
        })
    }
    fn rebase(&self, _path: &Path, _onto: &str) -> Result<(), GitError> {
        Ok(())
    }
}

fn make_pool() -> ConnPool {
    let pool = Pool::builder()
        .max_size(4)
        .build(SqliteConnectionManager::memory())
        .unwrap();
    pool.get()
        .unwrap()
        .execute_batch(
            "CREATE TABLE workspaces (
                id INTEGER PRIMARY KEY AUTOINCREMENT, workspace_id TEXT NOT NULL UNIQUE,
                plan_id INTEGER, wave_db_id INTEGER, path TEXT NOT NULL,
                branch TEXT NOT NULL, status TEXT NOT NULL DEFAULT 'active',
                created_at TEXT NOT NULL DEFAULT (datetime('now')), deleted_at TEXT
            );
            CREATE TABLE workspace_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT, workspace_id TEXT NOT NULL,
                agent TEXT NOT NULL, action TEXT NOT NULL, file_path TEXT,
                detail TEXT, metadata TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();
    pool
}

#[test]
fn test_generate_pr_description_contains_workspace_id() {
    let pool = make_pool();
    let logger = EventLogger::new(pool.clone());
    logger
        .record_event(
            "ws-pr",
            "task-executor",
            WorkspaceAction::FileWrite,
            Some("src/lib.rs"),
            Some("wrote 50 lines"),
            None,
        )
        .unwrap();
    logger
        .record_event(
            "ws-pr",
            "release-agent",
            WorkspaceAction::GitCommit,
            None,
            None,
            None,
        )
        .unwrap();

    let agent = ReleaseAgent::new(Box::new(MockOk), EventLogger::new(pool.clone()), pool);
    let desc = agent.generate_pr_description("ws-pr");
    assert!(
        desc.contains("ws-pr"),
        "description must include workspace_id"
    );
    assert!(
        desc.contains("Workspace Release") || desc.contains("workspace"),
        "must have header"
    );
}

#[tokio::test]
async fn test_release_agent_new_and_struct() {
    let pool = make_pool();
    let logger = EventLogger::new(pool.clone());
    let agent = ReleaseAgent::new(Box::new(MockOk), logger, pool);
    // ReleaseAgent exists and release method is callable — missing workspace returns Err
    assert!(agent.release("missing-ws", "org/repo").await.is_err());
}
