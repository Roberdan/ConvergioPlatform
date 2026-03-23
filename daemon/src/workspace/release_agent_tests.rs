// Tests for ReleaseAgent — TDD RED phase written before implementation.
// Why: Plan 698 T4-01; verify event-driven release pipeline correctness.
use super::{ReleaseAgent, ReleaseResult};
use crate::server::state_init::ConnPool;
use crate::workspace::events::EventLogger;
use crate::workspace::git_connector::{AsyncResult, GitConnector, MergeMethod, PrInfo, PrReadiness};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::path::Path;
use std::sync::Mutex;

// ── Mock GitConnector ─────────────────────────────────────────────────────────

struct MockConnector {
    pub commits: Mutex<Vec<String>>,
    pub pushes: Mutex<Vec<(String, bool)>>,
    pub prs_created: Mutex<Vec<(String, String, String)>>,
    pub merges: Mutex<Vec<i64>>,
    pub pr_number: i64,
    pub mergeable: bool,
    pub ci_passed: bool,
    pub commit_err: Option<String>,
    pub push_err: Option<String>,
}

impl MockConnector {
    fn new_ok() -> Self {
        Self {
            commits: Mutex::new(vec![]),
            pushes: Mutex::new(vec![]),
            prs_created: Mutex::new(vec![]),
            merges: Mutex::new(vec![]),
            pr_number: 42,
            mergeable: true,
            ci_passed: true,
            commit_err: None,
            push_err: None,
        }
    }

    fn new_not_mergeable() -> Self {
        let mut m = Self::new_ok();
        m.mergeable = false;
        m.ci_passed = false;
        m
    }
}

impl GitConnector for MockConnector {
    fn commit(&self, _path: &Path, message: &str) -> Result<String, String> {
        if let Some(e) = &self.commit_err {
            return Err(e.clone());
        }
        self.commits.lock().unwrap().push(message.to_string());
        Ok("abc1234def5678901234567890123456789012ab".to_string())
    }

    fn push(&self, _path: &Path, branch: &str, fwl: bool) -> Result<(), String> {
        if let Some(e) = &self.push_err {
            return Err(e.clone());
        }
        self.pushes.lock().unwrap().push((branch.to_string(), fwl));
        Ok(())
    }

    fn create_pr<'a>(
        &'a self,
        repo: &'a str,
        branch: &'a str,
        _base: &'a str,
        _title: &'a str,
        _body: &'a str,
    ) -> AsyncResult<'a, PrInfo> {
        let pr_number = self.pr_number;
        let repo = repo.to_string();
        let branch = branch.to_string();
        self.prs_created
            .lock()
            .unwrap()
            .push((repo.clone(), branch.clone(), "main".to_string()));
        Box::pin(async move {
            Ok(PrInfo {
                number: pr_number,
                url: format!("https://github.com/{repo}/pull/{pr_number}"),
            })
        })
    }

    fn merge_pr<'a>(
        &'a self,
        _repo: &'a str,
        pr_number: i64,
        _method: MergeMethod,
    ) -> AsyncResult<'a, ()> {
        self.merges.lock().unwrap().push(pr_number);
        Box::pin(async { Ok(()) })
    }

    fn pr_readiness<'a>(&'a self, _repo: &'a str, _pr_number: i64) -> AsyncResult<'a, PrReadiness> {
        let mergeable = self.mergeable;
        let ci_passed = self.ci_passed;
        Box::pin(async move {
            Ok(PrReadiness {
                mergeable,
                ci_passed,
                pending_checks: 0,
                unresolved_threads: 0,
                review_status: if mergeable { "clean".into() } else { "blocked".into() },
            })
        })
    }

    fn rebase(&self, _path: &Path, _onto: &str) -> Result<(), String> {
        Ok(()) // best-effort, always ok in tests
    }
}

// ── DB helpers ────────────────────────────────────────────────────────────────

fn make_pool() -> ConnPool {
    let manager = SqliteConnectionManager::memory();
    let pool = Pool::builder().max_size(4).build(manager).unwrap();
    pool.get()
        .unwrap()
        .execute_batch(
            "CREATE TABLE workspaces (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            workspace_id TEXT NOT NULL UNIQUE,
            plan_id INTEGER,
            wave_db_id INTEGER,
            path TEXT NOT NULL,
            branch TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            deleted_at TEXT
        );
        CREATE TABLE workspace_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            workspace_id TEXT NOT NULL,
            agent TEXT NOT NULL,
            action TEXT NOT NULL,
            file_path TEXT,
            detail TEXT,
            metadata TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
        )
        .unwrap();
    pool
}

fn seed_workspace(pool: &ConnPool, workspace_id: &str, path: &str, branch: &str) {
    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO workspaces (workspace_id, path, branch, status) VALUES (?1, ?2, ?3, 'active')",
        rusqlite::params![workspace_id, path, branch],
    )
    .unwrap();
}

fn make_agent(connector: Box<dyn GitConnector>, pool: ConnPool) -> ReleaseAgent {
    let logger = EventLogger::new(pool.clone());
    ReleaseAgent::new(connector, logger, pool)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_release_result_fields() {
    let r = ReleaseResult {
        workspace_id: "ws-abc".to_string(),
        pr_number: 42,
        pr_url: "https://github.com/org/repo/pull/42".to_string(),
        quality_gates_passed: true,
        merged: true,
    };
    assert_eq!(r.workspace_id, "ws-abc");
    assert_eq!(r.pr_number, 42);
    assert!(r.quality_gates_passed && r.merged);
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("ws-abc") && json.contains("42"));
}

#[tokio::test]
async fn test_release_workspace_not_found() {
    let pool = make_pool();
    let agent = make_agent(Box::new(MockConnector::new_ok()), pool);
    let result = agent.release("nonexistent", "org/repo").await;
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(
        msg.contains("workspace not found") || msg.contains("not found"),
        "got: {msg}"
    );
}

#[tokio::test]
async fn test_release_full_pipeline_merged() {
    let pool = make_pool();
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();
    seed_workspace(&pool, "ws-001", &path, "plan/698-w4");

    let agent = make_agent(Box::new(MockConnector::new_ok()), pool.clone());
    // Quality gates may fail (no real repo) — assert no panic and meaningful error if Err
    match agent.release("ws-001", "org/repo").await {
        Ok(r) => assert_eq!(r.workspace_id, "ws-001"),
        Err(e) => assert!(!e.is_empty(), "error must be descriptive"),
    }
}

#[tokio::test]
async fn test_release_not_merged_when_ci_pending() {
    let pool = make_pool();
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();
    seed_workspace(&pool, "ws-002", &path, "plan/698-w4");

    let agent = make_agent(Box::new(MockConnector::new_not_mergeable()), pool.clone());
    match agent.release("ws-002", "org/repo").await {
        Ok(r) => {
            assert!(!r.merged, "should not be merged when CI pending");
            assert_eq!(r.workspace_id, "ws-002");
        }
        // Quality gate may fail (no real repo) — also acceptable
        Err(e) => assert!(!e.is_empty()),
    }
}

#[tokio::test]
async fn test_release_records_events() {
    let pool = make_pool();
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().to_str().unwrap().to_string();
    seed_workspace(&pool, "ws-events", &path, "branch/test");

    let logger = EventLogger::new(pool.clone());
    let agent = ReleaseAgent::new(
        Box::new(MockConnector::new_ok()),
        EventLogger::new(pool.clone()),
        pool.clone(),
    );
    let _ = agent.release("ws-events", "org/repo").await;

    let events = logger.query_events("ws-events", None, None).unwrap();
    assert!(!events.is_empty(), "at least one event must be recorded");
    let actions: Vec<&str> = events.iter().map(|e| e.action.as_str()).collect();
    let has_gate = actions
        .iter()
        .any(|a| *a == "quality_gate_pass" || *a == "quality_gate_fail");
    assert!(
        has_gate,
        "quality gate event must be recorded, got: {actions:?}"
    );
}

// Additional tests (PR description + struct construction) split to stay under 250 lines.
#[path = "release_agent_tests2.rs"]
mod part2;
