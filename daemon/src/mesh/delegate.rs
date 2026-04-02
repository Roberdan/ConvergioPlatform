// Delegation engine: SSH-based task spawning and monitoring on mesh peers.

pub use super::delegate_types::{DelegateError, DelegateResult, DelegateStatus};

// Re-export prompt helpers so existing callers (including tests) can access them.
pub(crate) use super::delegate_prompt::{
    delegate_timeout, parse_tokens_from_output, remote_worktree_dir, worktree_branch,
};
// Re-export for tests that use `crate::mesh::delegate::ssh_destination_legacy`.
#[cfg(test)]
pub(crate) use super::delegate_prompt::ssh_destination_legacy;
use super::handoff::SshClient;
use super::peer_resolver;
use super::peers::PeersRegistry;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

const SSH_CONNECT_TIMEOUT_SECS: u64 = 15;
const HEALTH_CHECK_RETRIES: u32 = 3;

pub struct DelegateEngine {
    peers_conf_path: PathBuf,
    db_path: Option<PathBuf>,
}

impl DelegateEngine {
    pub fn new(peers_conf_path: PathBuf) -> Self {
        Self { peers_conf_path, db_path: None }
    }

    pub fn with_db(mut self, db_path: PathBuf) -> Self {
        self.db_path = Some(db_path);
        self
    }

    fn resolve_peer(&self, peer_name: &str) -> Result<(peer_resolver::ResolvedPeer, String), DelegateError> {
        let registry = PeersRegistry::load(&self.peers_conf_path)?;
        let resolved = peer_resolver::resolve_from_registry(peer_name, &registry)
            .map_err(|_| DelegateError::PeerNotFound(peer_name.to_owned()))?;
        let config = registry
            .peers
            .get(&resolved.canonical_name)
            .ok_or_else(|| DelegateError::PeerNotFound(peer_name.to_owned()))?;
        if config.status != "active" {
            return Err(DelegateError::PeerInactive(
                resolved.canonical_name.clone(),
                config.status.clone(),
            ));
        }
        let dest = peer_resolver::ssh_destination(&resolved);
        Ok((resolved, dest))
    }

    fn check_remote_health(ssh: &SshClient, peer_name: &str) -> Result<(), DelegateError> {
        let cmd = "curl -sf --max-time 5 http://localhost:8420/api/health";
        for attempt in 1..=HEALTH_CHECK_RETRIES {
            match ssh.exec(cmd) {
                Ok((0, out, _)) if !out.is_empty() => {
                    debug!(peer = peer_name, "remote daemon healthy");
                    return Ok(());
                }
                Ok((code, _, stderr)) => {
                    warn!(
                        peer = peer_name,
                        attempt, code, "health check failed: {stderr}"
                    );
                }
                Err(e) => warn!(peer = peer_name, attempt, "health check error: {e}"),
            }
            if attempt < HEALTH_CHECK_RETRIES {
                std::thread::sleep(Duration::from_secs(2));
            }
        }
        Err(DelegateError::DaemonUnhealthy(
            peer_name.to_owned(),
            format!("failed after {HEALTH_CHECK_RETRIES} attempts"),
        ))
    }

    fn create_remote_worktree(
        ssh: &SshClient,
        plan_id: i64,
        task_id: &str,
    ) -> Result<String, DelegateError> {
        let branch = worktree_branch(plan_id, task_id);
        let dir = remote_worktree_dir(plan_id, task_id);
        let cmd = format!(
            "cd ~/GitHub/ConvergioPlatform && \
             git fetch origin main 2>/dev/null; \
             git branch {branch} origin/main 2>/dev/null || true; \
             git worktree add {dir} {branch} 2>&1"
        );
        let (code, out, err) = ssh.exec(&cmd).map_err(|e: crate::mesh::error::MeshError| {
            DelegateError::WorktreeCreate(format!("ssh exec: {e}"))
        })?;
        if code != 0 {
            return Err(DelegateError::WorktreeCreate(format!(
                "exit {code}: {out} {err}"
            )));
        }
        debug!(branch, dir, "remote worktree created");
        Ok(dir)
    }

    fn spawn_and_monitor(
        ssh: &SshClient,
        worktree_dir: &str,
        plan_id: i64,
        task_id: &str,
        agent_type: &str,
        timeout: Duration,
    ) -> Result<(String, u64, DelegateStatus), DelegateError> {
        let cmd = format!(
            "cd {worktree_dir} && \
             PLAN_ID={plan_id} TASK_ID={task_id} AGENT_TYPE={agent_type} \
             timeout {}s claude --agent {agent_type} --plan {plan_id} --task {task_id} 2>&1",
            timeout.as_secs()
        );
        let (code, stdout, stderr) =
            ssh.exec(&cmd).map_err(|e: crate::mesh::error::MeshError| {
                DelegateError::AgentSpawn(format!("ssh exec: {e}"))
            })?;
        let output = if stderr.is_empty() {
            stdout.clone()
        } else {
            format!("{stdout}\n--- stderr ---\n{stderr}")
        };
        let tokens = parse_tokens_from_output(&stdout);
        let status = match code {
            0 => DelegateStatus::Success,
            124 => DelegateStatus::TimedOut,
            _ => DelegateStatus::Failed,
        };
        Ok((output, tokens, status))
    }

    fn cleanup_remote_worktree(ssh: &SshClient, plan_id: i64, task_id: &str) {
        let dir = remote_worktree_dir(plan_id, task_id);
        let branch = worktree_branch(plan_id, task_id);
        let cmd = format!(
            "cd ~/GitHub/ConvergioPlatform && \
             git worktree remove {dir} --force 2>/dev/null; \
             git branch -D {branch} 2>/dev/null; true"
        );
        if let Err(e) = ssh.exec(&cmd) {
            warn!("cleanup failed for plan {plan_id} task {task_id}: {e}");
        }
    }

    /// Delegate a task to a remote mesh peer via SSH.
    pub async fn delegate_task(
        &self,
        peer_name: &str,
        plan_id: i64,
        task_id: &str,
        agent_type: &str,
    ) -> Result<DelegateResult, DelegateError> {
        let (resolved, dest) = self.resolve_peer(peer_name)?;
        let timeout = delegate_timeout();
        let (peer_owned, task_owned, agent_owned) = (
            resolved.canonical_name.clone(),
            task_id.to_owned(),
            agent_type.to_owned(),
        );
        let db_path = self.db_path.clone();
        let del_id = format!("del-{plan_id}-{}-{}", peer_name, task_id);
        info!(
            peer = peer_name,
            plan_id, task_id, agent_type,
            timeout_secs = timeout.as_secs(),
            "delegating task"
        );

        tokio::task::spawn_blocking(move || {
            use super::delegate_progress::record_step;
            let step = |s: &str, st: &str, msg: Option<&str>| {
                if let Some(ref p) = db_path {
                    record_step(p, &del_id, s, st, msg);
                }
            };

            step("resolving", "running", None);
            let started = Instant::now();

            step("connecting", "running", None);
            let ssh = SshClient::connect(&dest, Duration::from_secs(SSH_CONNECT_TIMEOUT_SECS))
                .map_err(|e: crate::mesh::error::MeshError| {
                    step("connecting", "blocked", Some(&e.to_string()));
                    DelegateError::SshConnect(e.to_string())
                })?;
            Self::check_remote_health(&ssh, &peer_owned)?;

            step("transferring", "running", None);
            let worktree_dir = Self::create_remote_worktree(&ssh, plan_id, &task_owned)?;

            step("executing", "running", None);
            let (output, tokens, status) = match Self::spawn_and_monitor(
                &ssh, &worktree_dir, plan_id, &task_owned, &agent_owned, timeout,
            ) {
                Ok(r) => r,
                Err(e) => {
                    step("failed", "blocked", Some(&e.to_string()));
                    Self::cleanup_remote_worktree(&ssh, plan_id, &task_owned);
                    return Err(e);
                }
            };
            if status != DelegateStatus::Success {
                step("failed", "blocked", Some(&output));
                Self::cleanup_remote_worktree(&ssh, plan_id, &task_owned);
            } else {
                step("completed", "done", None);
            }
            Ok(DelegateResult {
                status,
                output,
                tokens_used: tokens,
                duration: started.elapsed(),
                peer_name: peer_owned,
                worktree_path: if status == DelegateStatus::Success {
                    Some(worktree_dir)
                } else {
                    None
                },
            })
        })
        .await
        .map_err(|e| DelegateError::AgentSpawn(format!("task join: {e}")))?
    }
}

#[cfg(test)]
#[path = "delegate_tests.rs"]
mod delegate_tests;
