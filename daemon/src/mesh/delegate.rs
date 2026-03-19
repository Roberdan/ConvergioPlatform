// Delegation engine: SSH-based task spawning and monitoring on mesh peers.

use super::handoff::SshClient;
use super::peers::{PeerConfig, PeersError, PeersRegistry};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{debug, info, warn};

const DEFAULT_TIMEOUT_SECS: u64 = 30 * 60;
const SSH_CONNECT_TIMEOUT_SECS: u64 = 15;
const HEALTH_CHECK_RETRIES: u32 = 3;

#[derive(Debug, Error)]
pub enum DelegateError {
    #[error("peer not found: {0}")]
    PeerNotFound(String),
    #[error("peer '{0}' is not active (status: {1})")]
    PeerInactive(String, String),
    #[error("SSH connection failed: {0}")]
    SshConnect(String),
    #[error("remote daemon not healthy on {0}: {1}")]
    DaemonUnhealthy(String, String),
    #[error("worktree creation failed: {0}")]
    WorktreeCreate(String),
    #[error("agent spawn failed: {0}")]
    AgentSpawn(String),
    #[error("task timed out after {0:?}")]
    Timeout(Duration),
    #[error("peers config error: {0}")]
    PeersConfig(#[from] PeersError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateResult {
    pub status: DelegateStatus,
    pub output: String,
    pub tokens_used: u64,
    pub duration: Duration,
    pub peer_name: String,
    pub worktree_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegateStatus {
    Success,
    Failed,
    TimedOut,
    Cancelled,
}

pub(crate) fn ssh_destination(peer: &PeerConfig) -> String {
    if !peer.ssh_alias.is_empty() { peer.ssh_alias.clone() }
    else { format!("{}@{}", peer.user, peer.tailscale_ip) }
}

pub(crate) fn delegate_timeout() -> Duration {
    let secs = std::env::var("DELEGATE_TIMEOUT")
        .ok().and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

pub(crate) fn worktree_branch(plan_id: i64, task_id: &str) -> String {
    format!("delegate/plan-{plan_id}/{task_id}")
}

fn remote_worktree_dir(plan_id: i64, task_id: &str) -> String {
    format!("$HOME/.claude/worktrees/delegate-plan-{plan_id}-{task_id}")
}

pub struct DelegateEngine {
    peers_conf_path: PathBuf,
}

impl DelegateEngine {
    pub fn new(peers_conf_path: PathBuf) -> Self {
        Self { peers_conf_path }
    }

    fn resolve_peer(&self, peer_name: &str) -> Result<PeerConfig, DelegateError> {
        let registry = PeersRegistry::load(&self.peers_conf_path)?;
        let peer = registry.peers.get(peer_name)
            .ok_or_else(|| DelegateError::PeerNotFound(peer_name.to_owned()))?;
        if peer.status != "active" {
            return Err(DelegateError::PeerInactive(
                peer_name.to_owned(), peer.status.clone()));
        }
        Ok(peer.clone())
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
                    warn!(peer = peer_name, attempt, code, "health check failed: {stderr}");
                }
                Err(e) => warn!(peer = peer_name, attempt, "health check error: {e}"),
            }
            if attempt < HEALTH_CHECK_RETRIES {
                std::thread::sleep(Duration::from_secs(2));
            }
        }
        Err(DelegateError::DaemonUnhealthy(
            peer_name.to_owned(), format!("failed after {HEALTH_CHECK_RETRIES} attempts")))
    }

    fn create_remote_worktree(
        ssh: &SshClient, plan_id: i64, task_id: &str,
    ) -> Result<String, DelegateError> {
        let branch = worktree_branch(plan_id, task_id);
        let dir = remote_worktree_dir(plan_id, task_id);
        let cmd = format!(
            "cd ~/GitHub/ConvergioPlatform && \
             git fetch origin main 2>/dev/null; \
             git branch {branch} origin/main 2>/dev/null || true; \
             git worktree add {dir} {branch} 2>&1");
        let (code, out, err) = ssh.exec(&cmd)
            .map_err(|e| DelegateError::WorktreeCreate(format!("ssh exec: {e}")))?;
        if code != 0 {
            return Err(DelegateError::WorktreeCreate(format!("exit {code}: {out} {err}")));
        }
        debug!(branch, dir, "remote worktree created");
        Ok(dir)
    }

    fn spawn_and_monitor(
        ssh: &SshClient, worktree_dir: &str, plan_id: i64,
        task_id: &str, agent_type: &str, timeout: Duration,
    ) -> Result<(String, u64, DelegateStatus), DelegateError> {
        let cmd = format!(
            "cd {worktree_dir} && \
             PLAN_ID={plan_id} TASK_ID={task_id} AGENT_TYPE={agent_type} \
             timeout {}s claude --agent {agent_type} --plan {plan_id} --task {task_id} 2>&1",
            timeout.as_secs());
        let (code, stdout, stderr) = ssh.exec(&cmd)
            .map_err(|e| DelegateError::AgentSpawn(format!("ssh exec: {e}")))?;
        let output = if stderr.is_empty() { stdout.clone() }
            else { format!("{stdout}\n--- stderr ---\n{stderr}") };
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
             git branch -D {branch} 2>/dev/null; true");
        if let Err(e) = ssh.exec(&cmd) {
            warn!("cleanup failed for plan {plan_id} task {task_id}: {e}");
        }
    }

    /// Delegate a task to a remote mesh peer via SSH.
    pub async fn delegate_task(
        &self, peer_name: &str, plan_id: i64, task_id: &str, agent_type: &str,
    ) -> Result<DelegateResult, DelegateError> {
        let peer = self.resolve_peer(peer_name)?;
        let dest = ssh_destination(&peer);
        let timeout = delegate_timeout();
        let (peer_owned, task_owned, agent_owned) =
            (peer_name.to_owned(), task_id.to_owned(), agent_type.to_owned());
        info!(peer = peer_name, plan_id, task_id, agent_type,
            timeout_secs = timeout.as_secs(), "delegating task");

        tokio::task::spawn_blocking(move || {
            let started = Instant::now();
            let ssh = SshClient::connect(&dest, Duration::from_secs(SSH_CONNECT_TIMEOUT_SECS))
                .map_err(DelegateError::SshConnect)?;
            Self::check_remote_health(&ssh, &peer_owned)?;
            let worktree_dir = Self::create_remote_worktree(&ssh, plan_id, &task_owned)?;
            let (output, tokens, status) = match Self::spawn_and_monitor(
                &ssh, &worktree_dir, plan_id, &task_owned, &agent_owned, timeout,
            ) {
                Ok(r) => r,
                Err(e) => {
                    Self::cleanup_remote_worktree(&ssh, plan_id, &task_owned);
                    return Err(e);
                }
            };
            if status != DelegateStatus::Success {
                Self::cleanup_remote_worktree(&ssh, plan_id, &task_owned);
            }
            Ok(DelegateResult {
                status, output, tokens_used: tokens,
                duration: started.elapsed(), peer_name: peer_owned,
                worktree_path: if status == DelegateStatus::Success {
                    Some(worktree_dir) } else { None },
            })
        })
        .await
        .map_err(|e| DelegateError::AgentSpawn(format!("task join: {e}")))?
    }
}

/// Extract token count from progress markers (`[tokens: N]` or `tokens_used=N`).
pub(crate) fn parse_tokens_from_output(output: &str) -> u64 {
    for line in output.lines().rev() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("[tokens:") {
            if let Some(val) = rest.trim().strip_suffix(']') {
                if let Ok(n) = val.trim().parse::<u64>() { return n; }
            }
        }
        if let Some(rest) = t.strip_prefix("tokens_used=") {
            if let Ok(n) = rest.trim().parse::<u64>() { return n; }
        }
    }
    0
}

#[cfg(test)]
#[path = "delegate_tests.rs"]
mod delegate_tests;
