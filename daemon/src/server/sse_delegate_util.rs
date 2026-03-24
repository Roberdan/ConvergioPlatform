// SSE delegation helpers: event builders, task status updates, WS broadcast.

use crate::server::state::{ApiError, ServerState};
use crate::server::ws_brain::broadcast_brain_task_update;
use axum::response::sse::Event;
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;

pub(super) type Events = Vec<Result<Event, Infallible>>;

/// Allowed CLI tools for remote agent execution.
/// Catch-all dispatch is intentionally absent to prevent RCE via arbitrary cli values.
const ALLOWED_CLI: &[&str] = &["claude", "copilot"];

/// Safe identifier pattern: alphanumeric, hyphen, underscore only.
/// Rejects any shell metacharacter that could escape the command context.
fn is_safe_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub(super) fn broadcast_ws(state: &ServerState, qs: &HashMap<String, String>, status: &str) {
    if let Some(tid) = qs.get("task_id").filter(|v| !v.is_empty()) {
        if let Ok(id) = tid.parse::<i64>() {
            broadcast_brain_task_update(state, id, status);
        }
    }
}

/// Build the shell command to run the agent on the remote machine via SSH.
///
/// Returns `Err(ApiError)` if `cli` is not in `ALLOWED_CLI` or if any ID
/// parameter contains characters outside `[a-zA-Z0-9_-]` to prevent injection.
pub(super) fn build_agent_command(
    cli: &str,
    plan_id: &str,
    qs: &HashMap<String, String>,
) -> Result<String, ApiError> {
    // Validate cli against allowlist — reject unknown tools
    if !ALLOWED_CLI.contains(&cli) {
        return Err(ApiError::bad_request(format!(
            "cli '{cli}' is not in the allowed list"
        )));
    }

    // Validate plan_id
    if !is_safe_id(plan_id) {
        return Err(ApiError::bad_request(format!(
            "plan_id '{plan_id}' contains disallowed characters"
        )));
    }

    let task_id = qs.get("task_id").cloned().unwrap_or_default();
    let wave_id = qs.get("wave_id").cloned().unwrap_or_default();

    // Validate optional IDs only when present
    if !task_id.is_empty() && !is_safe_id(&task_id) {
        return Err(ApiError::bad_request(format!(
            "task_id '{task_id}' contains disallowed characters"
        )));
    }
    if !wave_id.is_empty() && !is_safe_id(&wave_id) {
        return Err(ApiError::bad_request(format!(
            "wave_id '{wave_id}' contains disallowed characters"
        )));
    }

    let dir = "~/GitHub/ConvergioPlatform";
    // Build the prompt string incrementally — never interpolate raw user input into format!
    let mut prompt = format!("Execute plan {plan_id}");
    if !task_id.is_empty() {
        prompt.push_str(&format!(" task {task_id}"));
    }
    if !wave_id.is_empty() {
        prompt.push_str(&format!(" wave {wave_id}"));
    }

    // Use the validated cli name (from allowlist) as a literal — not interpolated from input
    let cli_bin = match cli {
        "claude" => "claude --dangerously-skip-permissions",
        "copilot" => "copilot",
        // Unreachable: allowlist check above ensures only known values reach here
        _ => unreachable!("ALLOWED_CLI check must have passed"),
    };

    Ok(format!("cd {dir} && {cli_bin} -p '{prompt}'"))
}

pub(super) fn stage(s: &str, peer: &str, detail: &str) -> serde_json::Value {
    json!({"type": "stage", "stage": s, "peer": peer, "detail": detail})
}

pub(super) fn push(events: &mut Events, event_type: &str, data: &serde_json::Value) {
    events.push(Ok(Event::default()
        .event(event_type)
        .data(data.to_string())));
}

pub(super) fn update_task_status(state: &ServerState, qs: &HashMap<String, String>, status: &str) {
    let task_id = match qs.get("task_id").filter(|v| !v.is_empty()) {
        Some(id) => id,
        None => return,
    };
    let plan_id = match qs.get("plan_id").filter(|v| !v.is_empty()) {
        Some(id) => id,
        None => return,
    };
    if let Ok(conn) = state.get_conn() {
        if let Err(e) = conn.execute(
            "UPDATE tasks SET status=?1 WHERE plan_id=?2 AND id=?3",
            [status, plan_id.as_str(), task_id.as_str()],
        ) {
            tracing::warn!("delegate task status update failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_id_valid_and_invalid() {
        assert!(is_safe_id("T1-02"));
        assert!(is_safe_id("671"));
        assert!(is_safe_id("plan_706"));
        assert!(!is_safe_id(""));
        for bad in &[
            "; rm -rf /",
            "$(whoami)",
            "`id`",
            "foo && bar",
            "foo | bar",
            "foo > /tmp/x",
            "foo\nbar",
            "foo'bar",
            "foo\"bar",
            "foo bar",
        ] {
            assert!(!is_safe_id(bad), "expected rejection of: {bad}");
        }
    }

    #[test]
    fn valid_cli_claude_builds_command() {
        let cmd = build_agent_command("claude", "671", &HashMap::new()).unwrap();
        assert!(cmd.contains("claude --dangerously-skip-permissions"));
        assert!(cmd.contains("Execute plan 671"));
        assert!(!cmd.contains("task "));
        assert!(!cmd.contains("wave "));

        let mut qs = HashMap::new();
        qs.insert("task_id".into(), "T1-02".into());
        qs.insert("wave_id".into(), "W1".into());
        let cmd = build_agent_command("claude", "671", &qs).unwrap();
        assert!(cmd.contains("Execute plan 671 task T1-02 wave W1"));
    }

    #[test]
    fn valid_cli_copilot_builds_command() {
        let mut qs = HashMap::new();
        qs.insert("task_id".into(), "99".into());
        let cmd = build_agent_command("copilot", "42", &qs).unwrap();
        assert!(cmd.contains("copilot") && !cmd.contains("--dangerously-skip-permissions"));
    }

    #[test]
    fn invalid_cli_and_injection_rejected() {
        let msg = build_agent_command("my-agent", "42", &HashMap::new())
            .unwrap_err()
            .to_string();
        assert!(msg.contains("not in the allowed list"), "got: {msg}");
        assert!(build_agent_command("claude; rm -rf /", "42", &HashMap::new()).is_err());
    }

    #[test]
    fn plan_id_injection_rejected() {
        let msg = build_agent_command("claude", "42; curl attacker.com", &HashMap::new())
            .unwrap_err()
            .to_string();
        assert!(msg.contains("plan_id"), "got: {msg}");
        assert!(build_agent_command("claude", "`whoami`", &HashMap::new()).is_err());
    }

    #[test]
    fn task_wave_id_injection_rejected() {
        let mut qs = HashMap::new();
        qs.insert("task_id".into(), "T1-02 && evil".into());
        let msg = build_agent_command("claude", "671", &qs)
            .unwrap_err()
            .to_string();
        assert!(msg.contains("task_id"), "got: {msg}");

        let mut qs2 = HashMap::new();
        qs2.insert("wave_id".into(), "W1$(id)".into());
        let msg2 = build_agent_command("claude", "671", &qs2)
            .unwrap_err()
            .to_string();
        assert!(msg2.contains("wave_id"), "got: {msg2}");
    }

    #[test]
    fn command_uses_hardcoded_binary() {
        let cmd = build_agent_command("claude", "1", &HashMap::new()).unwrap();
        assert!(cmd.contains("claude --dangerously-skip-permissions"));
    }
}
