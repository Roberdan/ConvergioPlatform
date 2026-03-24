// Tests for merge_ops module — MockGitConnector verifies pipeline call order.
use super::*;
use crate::workspace::git_connector::{AsyncResult, GitError, MergeMethod, PrInfo, PrReadiness};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::cell::RefCell;
use std::path::Path;
use std::sync::Mutex;

/// Records call order to verify pipeline sequence.
struct MockGitConnector {
    calls: Mutex<RefCell<Vec<String>>>,
    pr_ready: bool,
}

impl MockGitConnector {
    fn new(pr_ready: bool) -> Self {
        Self {
            calls: Mutex::new(RefCell::new(Vec::new())),
            pr_ready,
        }
    }

    fn recorded_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().borrow().clone()
    }

    fn push_call(&self, name: &str) {
        self.calls
            .lock()
            .unwrap()
            .borrow_mut()
            .push(name.to_string());
    }
}

impl GitConnector for MockGitConnector {
    fn commit(&self, _path: &Path, _message: &str) -> std::result::Result<String, GitError> {
        self.push_call("commit");
        Ok("abc1234567890123456789012345678901234567890".to_string())
    }

    fn push(
        &self,
        _path: &Path,
        _branch: &str,
        _force_with_lease: bool,
    ) -> std::result::Result<(), GitError> {
        self.push_call("push");
        Ok(())
    }

    fn rebase(&self, _path: &Path, _onto: &str) -> std::result::Result<(), GitError> {
        self.push_call("rebase");
        Ok(())
    }

    fn create_pr<'a>(
        &'a self,
        _repo: &'a str,
        _branch: &'a str,
        _base: &'a str,
        _title: &'a str,
        _body: &'a str,
    ) -> AsyncResult<'a, PrInfo> {
        self.push_call("create_pr");
        Box::pin(async {
            Ok(PrInfo {
                number: 42,
                url: "https://github.com/example/repo/pull/42".to_string(),
            })
        })
    }

    fn merge_pr<'a>(
        &'a self,
        _repo: &'a str,
        _pr_number: i64,
        _method: MergeMethod,
    ) -> AsyncResult<'a, ()> {
        self.push_call("merge_pr");
        Box::pin(async { Ok(()) })
    }

    fn pr_readiness<'a>(&'a self, _repo: &'a str, _pr_number: i64) -> AsyncResult<'a, PrReadiness> {
        self.push_call("pr_readiness");
        let ready = self.pr_ready;
        Box::pin(async move {
            Ok(PrReadiness {
                mergeable: ready,
                ci_passed: ready,
                pending_checks: 0,
                unresolved_threads: 0,
                review_status: if ready {
                    "clean".into()
                } else {
                    "blocked".into()
                },
            })
        })
    }
}

fn make_pool_with_schema() -> ConnPool {
    let manager = SqliteConnectionManager::memory();
    let pool = Pool::builder().max_size(4).build(manager).unwrap();
    pool.get()
        .unwrap()
        .execute_batch(
            "CREATE TABLE workspace_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace_id TEXT NOT NULL,
                agent TEXT NOT NULL,
                action TEXT NOT NULL,
                file_path TEXT,
                detail TEXT,
                metadata TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE waves (
                id INTEGER PRIMARY KEY,
                plan_id INTEGER NOT NULL,
                wave_id TEXT NOT NULL DEFAULT 'W1',
                status TEXT NOT NULL DEFAULT 'in_progress',
                pr_number INTEGER,
                pr_url TEXT,
                worktree_path TEXT,
                branch_name TEXT
            );
            CREATE TABLE workspaces (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workspace_id TEXT NOT NULL,
                wave_db_id INTEGER,
                plan_id INTEGER,
                path TEXT NOT NULL,
                branch TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active'
            );",
        )
        .unwrap();
    pool
}

fn seed_wave_and_workspace(pool: &ConnPool, wave_db_id: i64, plan_id: i64) {
    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO waves (id, plan_id, wave_id, status) VALUES (?1, ?2, 'W1', 'in_progress')",
        params![wave_db_id, plan_id],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO workspaces (workspace_id, wave_db_id, plan_id, path, branch, status) \
         VALUES ('ws-test-1', ?1, ?2, '/tmp/fake-worktree', 'plan/698-W1', 'active')",
        params![wave_db_id, plan_id],
    )
    .unwrap();
}

#[tokio::test]
async fn merge_pipeline_calls_in_correct_order() {
    let pool = make_pool_with_schema();
    seed_wave_and_workspace(&pool, 1, 698);
    let connector = MockGitConnector::new(true);
    let event_logger = EventLogger::new(pool.clone());

    let result = merge_wave(1, "example/repo", &connector, &event_logger, &pool)
        .await
        .unwrap();

    let calls = connector.recorded_calls();
    assert_eq!(calls[0], "commit", "commit must be first");
    assert_eq!(calls[1], "rebase", "rebase must follow commit");
    assert_eq!(calls[2], "push", "push must follow rebase");
    assert_eq!(calls[3], "create_pr", "create_pr must follow push");
    assert_eq!(calls[4], "pr_readiness", "pr_readiness must be polled");
    assert_eq!(calls[5], "merge_pr", "merge_pr must follow readiness");

    assert_eq!(result.pr_number, 42);
    assert!(result.merged);
    assert!(!result.pr_url.is_empty());
}

#[tokio::test]
async fn merge_pipeline_updates_waves_table() {
    let pool = make_pool_with_schema();
    seed_wave_and_workspace(&pool, 2, 698);
    let connector = MockGitConnector::new(true);
    let event_logger = EventLogger::new(pool.clone());

    merge_wave(2, "example/repo", &connector, &event_logger, &pool)
        .await
        .unwrap();

    let conn = pool.get().unwrap();
    let (status, pr_number, pr_url): (String, Option<i64>, Option<String>) = conn
        .query_row(
            "SELECT status, pr_number, pr_url FROM waves WHERE id = 2",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(status, "done");
    assert_eq!(pr_number, Some(42));
    assert!(pr_url.is_some());
}

#[tokio::test]
async fn merge_pipeline_fails_when_no_workspace() {
    let pool = make_pool_with_schema();
    let connector = MockGitConnector::new(true);
    let event_logger = EventLogger::new(pool.clone());

    let err = merge_wave(99, "example/repo", &connector, &event_logger, &pool)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("wave 99"),
        "expected workspace error, got: {err}"
    );
}

#[tokio::test]
async fn pr_readiness_check_delegates_to_connector() {
    let connector = MockGitConnector::new(true);
    let readiness = pr_readiness_check(&connector, "example/repo", 42)
        .await
        .unwrap();
    assert!(readiness.mergeable);
    assert!(readiness.ci_passed);
    assert_eq!(readiness.review_status, "clean");
}

#[test]
fn merge_result_struct_fields() {
    let r = MergeResult {
        pr_number: 7,
        pr_url: "https://github.com/example/repo/pull/7".into(),
        merged: true,
    };
    assert_eq!(r.pr_number, 7);
    assert!(r.merged);
}
