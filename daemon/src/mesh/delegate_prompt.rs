// Why: Extracted from delegate.rs to keep files under 250 lines.
// Helper functions for delegation: timeouts, branch naming, token parsing.

use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 30 * 60;

pub(crate) fn ssh_destination_legacy(peer: &super::peers::PeerConfig) -> String {
    if !peer.ssh_alias.is_empty() {
        peer.ssh_alias.clone()
    } else {
        format!("{}@{}", peer.user, peer.tailscale_ip)
    }
}

pub(crate) fn delegate_timeout() -> Duration {
    let secs = std::env::var("DELEGATE_TIMEOUT")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

pub(crate) fn worktree_branch(plan_id: i64, task_id: &str) -> String {
    format!("delegate/plan-{plan_id}/{task_id}")
}

pub(crate) fn remote_worktree_dir(plan_id: i64, task_id: &str) -> String {
    format!("$HOME/.claude/worktrees/delegate-plan-{plan_id}-{task_id}")
}

/// Extract token count from progress markers (`[tokens: N]` or `tokens_used=N`).
pub(crate) fn parse_tokens_from_output(output: &str) -> u64 {
    for line in output.lines().rev() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("[tokens:") {
            if let Some(val) = rest.trim().strip_suffix(']') {
                if let Ok(n) = val.trim().parse::<u64>() {
                    return n;
                }
            }
        }
        if let Some(rest) = t.strip_prefix("tokens_used=") {
            if let Ok(n) = rest.trim().parse::<u64>() {
                return n;
            }
        }
    }
    0
}
