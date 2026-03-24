// Release Agent — event-driven pipeline: quality gates -> commit -> push -> PR -> merge.
// Why: Plan 698 T4-01; centralise release lifecycle so agents trigger it programmatically.
use super::events::{EventLogger, WorkspaceAction};
use super::git_connector::{GitConnector, MergeMethod};
use super::quality_gate::QualityGate;
use crate::server::state_init::ConnPool;
use crate::workspace::core::WorkspaceError;
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, WorkspaceError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseResult {
    pub workspace_id: String,
    pub pr_number: i64,
    pub pr_url: String,
    pub quality_gates_passed: bool,
    pub merged: bool,
}

pub struct ReleaseAgent {
    pub connector: Box<dyn GitConnector>,
    pub event_logger: EventLogger,
    pub pool: ConnPool,
}

impl ReleaseAgent {
    pub fn new(
        connector: Box<dyn GitConnector>,
        event_logger: EventLogger,
        pool: ConnPool,
    ) -> Self {
        Self {
            connector,
            event_logger,
            pool,
        }
    }

    /// Full release pipeline: quality gate -> commit -> push -> PR -> merge.
    pub async fn release(&self, workspace_id: &str, repo_slug: &str) -> Result<ReleaseResult> {
        let conn = self.pool.get()?;
        let (path, branch): (String, String) = conn
            .query_row(
                "SELECT path, branch FROM workspaces \
                 WHERE workspace_id = ?1 AND status = 'active'",
                rusqlite::params![workspace_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| WorkspaceError::NotFound(format!("workspace {workspace_id}: {e}")))?;
        let path = std::path::Path::new(&path);

        let gates = QualityGate::run_all(path);
        let all_passed = gates.iter().all(|g| g.passed);
        let gate_summary = format!(
            "{} gates, {} passed",
            gates.len(),
            gates.iter().filter(|g| g.passed).count()
        );
        self.event_logger
            .record_event(
                workspace_id,
                "release-agent",
                if all_passed {
                    WorkspaceAction::QualityGatePass
                } else {
                    WorkspaceAction::QualityGateFail
                },
                None,
                Some(&gate_summary),
                None,
            )
            .ok();

        if !all_passed {
            let failures: Vec<String> = gates
                .iter()
                .filter(|g| !g.passed)
                .map(|g| format!("{}: {}", g.gate_name, g.message))
                .collect();
            return Err(WorkspaceError::Validation(format!(
                "Quality gates failed:\n{}",
                failures.join("\n")
            )));
        }

        let _sha = self
            .connector
            .commit(
                path,
                &format!("feat: release from workspace {workspace_id}"),
            )
            .map_err(|e| WorkspaceError::Git(e.to_string()))?;
        self.event_logger
            .record_event(
                workspace_id,
                "release-agent",
                WorkspaceAction::GitCommit,
                None,
                None,
                None,
            )
            .ok();

        let _ = self.connector.rebase(path, "origin/main");

        self.connector
            .push(path, &branch, true)
            .map_err(|e| WorkspaceError::Git(e.to_string()))?;
        self.event_logger
            .record_event(
                workspace_id,
                "release-agent",
                WorkspaceAction::GitPush,
                None,
                None,
                None,
            )
            .ok();

        let pr_body = self.generate_pr_description(workspace_id);
        let pr = self
            .connector
            .create_pr(
                repo_slug,
                &branch,
                "main",
                &format!("feat: workspace {workspace_id}"),
                &pr_body,
            )
            .await
            .map_err(|e| WorkspaceError::Merge(e.to_string()))?;
        self.event_logger
            .record_event(
                workspace_id,
                "release-agent",
                WorkspaceAction::PrCreated,
                None,
                Some(&format!("PR #{}", pr.number)),
                None,
            )
            .ok();

        let readiness = self
            .connector
            .pr_readiness(repo_slug, pr.number)
            .await
            .map_err(|e| WorkspaceError::Merge(e.to_string()))?;
        if !readiness.mergeable || !readiness.ci_passed {
            return Ok(ReleaseResult {
                workspace_id: workspace_id.to_string(),
                pr_number: pr.number,
                pr_url: pr.url,
                quality_gates_passed: true,
                merged: false,
            });
        }

        self.connector
            .merge_pr(repo_slug, pr.number, MergeMethod::Squash)
            .await
            .map_err(|e| WorkspaceError::Merge(e.to_string()))?;
        self.event_logger
            .record_event(
                workspace_id,
                "release-agent",
                WorkspaceAction::PrMerged,
                None,
                Some(&format!("PR #{} merged", pr.number)),
                None,
            )
            .ok();

        conn.execute(
            "UPDATE workspaces SET status = 'merged' WHERE workspace_id = ?1",
            rusqlite::params![workspace_id],
        )
        .ok();

        Ok(ReleaseResult {
            workspace_id: workspace_id.to_string(),
            pr_number: pr.number,
            pr_url: pr.url,
            quality_gates_passed: true,
            merged: true,
        })
    }

    /// Build a PR description from workspace_events.
    pub fn generate_pr_description(&self, workspace_id: &str) -> String {
        let events = self
            .event_logger
            .query_events(workspace_id, Some(50), None)
            .unwrap_or_default();
        let file_writes: Vec<String> = events
            .iter()
            .filter(|e| e.action == "file_write")
            .filter_map(|e| e.file_path.clone())
            .collect();
        let agents: std::collections::BTreeSet<String> =
            events.iter().map(|e| e.agent.clone()).collect();
        format!(
            "## Workspace Release\n\n\
             **Workspace**: {workspace_id}\n\
             **Agents**: {}\n\
             **Files changed**: {}\n",
            agents.into_iter().collect::<Vec<_>>().join(", "),
            file_writes.len(),
        )
    }
}

#[cfg(test)]
#[path = "release_agent_tests.rs"]
mod tests;
