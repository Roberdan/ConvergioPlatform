// SSE delegation helpers: event builders, task status updates, WS broadcast.

use crate::server::state::ServerState;
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
/// Returns `Err` if `cli` is not in `ALLOWED_CLI` or if any ID parameter
/// contains characters outside `[a-zA-Z0-9_-]` to prevent command injection.
pub(super) fn build_agent_command(
    cli: &str,
    plan_id: &str,
    qs: &HashMap<String, String>,
) -> Result<String, String> {
    // Validate cli against allowlist — reject unknown tools
    if !ALLOWED_CLI.contains(&cli) {
        return Err(format!("cli '{cli}' is not in the allowed list"));
    }

    // Validate plan_id
    if !is_safe_id(plan_id) {
        return Err(format!(
            "plan_id '{plan_id}' contains disallowed characters"
        ));
    }

    let task_id = qs.get("task_id").cloned().unwrap_or_default();
    let wave_id = qs.get("wave_id").cloned().unwrap_or_default();

    // Validate optional IDs only when present
    if !task_id.is_empty() && !is_safe_id(&task_id) {
        return Err(format!(
            "task_id '{task_id}' contains disallowed characters"
        ));
    }
    if !wave_id.is_empty() && !is_safe_id(&wave_id) {
        return Err(format!(
            "wave_id '{wave_id}' contains disallowed characters"
        ));
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

    // --- is_safe_id tests ---

    #[test]
    fn safe_id_accepts_alphanumeric() {
        assert!(is_safe_id("T1-02"));
        assert!(is_safe_id("671"));
        assert!(is_safe_id("W1"));
        assert!(is_safe_id("plan_706"));
        assert!(is_safe_id("abc-123_XYZ"));
    }

    #[test]
    fn safe_id_rejects_empty() {
        assert!(!is_safe_id(""));
    }

    #[test]
    fn safe_id_rejects_shell_metacharacters() {
        // These are the injection vectors we must block
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

    // --- build_agent_command: valid inputs ---

    #[test]
    fn valid_cli_claude_no_optional_params() {
        let cmd = build_agent_command("claude", "671", &HashMap::new());
        assert!(cmd.is_ok(), "expected Ok, got: {:?}", cmd);
        let s = cmd.unwrap();
        assert!(s.contains("claude --dangerously-skip-permissions"));
        assert!(s.contains("Execute plan 671"));
        // Must NOT contain task/wave suffixes when absent
        assert!(!s.contains("task "));
        assert!(!s.contains("wave "));
    }

    #[test]
    fn valid_cli_claude_with_task_and_wave() {
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
        assert!(cmd.contains("copilot"));
        assert!(cmd.contains("Execute plan 42 task 99"));
        // copilot must NOT use claude's --dangerously-skip-permissions flag
        assert!(!cmd.contains("--dangerously-skip-permissions"));
    }

    // --- build_agent_command: invalid cli ---

    #[test]
    fn invalid_cli_rejected() {
        let err = build_agent_command("my-agent", "42", &HashMap::new());
        assert!(err.is_err());
        let msg = err.unwrap_err();
        assert!(
            msg.contains("not in the allowed list"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn cli_injection_attempt_rejected() {
        // Attacker-controlled cli value with shell metacharacters
        let err = build_agent_command("claude; rm -rf /", "42", &HashMap::new());
        assert!(err.is_err());
    }

    // --- build_agent_command: injection in plan_id ---

    #[test]
    fn plan_id_injection_rejected() {
        let err = build_agent_command("claude", "42; curl attacker.com", &HashMap::new());
        assert!(err.is_err());
        let msg = err.unwrap_err();
        assert!(
            msg.contains("plan_id"),
            "expected plan_id error, got: {msg}"
        );
    }

    #[test]
    fn plan_id_backtick_injection_rejected() {
        let err = build_agent_command("claude", "`whoami`", &HashMap::new());
        assert!(err.is_err());
    }

    // --- build_agent_command: injection in task_id / wave_id ---

    #[test]
    fn task_id_injection_rejected() {
        let mut qs = HashMap::new();
        qs.insert("task_id".into(), "T1-02 && evil".into());
        let err = build_agent_command("claude", "671", &qs);
        assert!(err.is_err());
        let msg = err.unwrap_err();
        assert!(
            msg.contains("task_id"),
            "expected task_id error, got: {msg}"
        );
    }

    #[test]
    fn wave_id_injection_rejected() {
        let mut qs = HashMap::new();
        qs.insert("wave_id".into(), "W1$(id)".into());
        let err = build_agent_command("claude", "671", &qs);
        assert!(err.is_err());
        let msg = err.unwrap_err();
        assert!(
            msg.contains("wave_id"),
            "expected wave_id error, got: {msg}"
        );
    }

    // --- no format! shell string construction with raw cli ---

    #[test]
    fn command_does_not_interpolate_cli_param_directly() {
        // Verify the output uses a hardcoded binary path, not the caller's cli string
        let cmd = build_agent_command("claude", "1", &HashMap::new()).unwrap();
        // The literal string "claude" appears but only as a hardcoded tool name
        assert!(cmd.contains("claude --dangerously-skip-permissions"));
    }
}
