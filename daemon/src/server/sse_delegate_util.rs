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
    // Build the prompt with cvg workflow instructions (matches delegation_core)
    let mut prompt = format!(
        "Execute plan {plan_id}. Per task: \
         cvg task update <id> in_progress → work → cvg task update <id> submitted. \
         Run ALL verify[] commands before submitting. \
         After each wave: cvg plan validate {plan_id}. \
         After all waves: cvg plan complete {plan_id}"
    );
    if !task_id.is_empty() {
        prompt.push_str(&format!(" task {task_id}"));
    }
    if !wave_id.is_empty() {
        prompt.push_str(&format!(" wave {wave_id}"));
    }

    // Use file-based prompt delivery: write prompt to temp file, pass via --input-file.
    // This preserves all special characters (quotes, backticks, $vars, newlines) that
    // would be mangled by tmux send-keys or shell -p interpolation.
    let prompt_file = format!("/tmp/convergio-prompt-{plan_id}.txt");

    // Build per-cli command. For claude: write task-scoped settings.json to the
    // worktree instead of using --dangerously-skip-permissions.
    let cmd = match cli {
        "claude" => {
            let settings =
                crate::orchestrator::worktree_settings::generate_worktree_settings("rust");
            format!(
                "mkdir -p {dir}/.claude && printf '%s' {settings:?} > {dir}/.claude/settings.json \
                 && printf '%s' {prompt:?} > {prompt_file} \
                 && cd {dir} && claude --input-file {prompt_file}; rm -f {prompt_file}"
            )
        }
        "copilot" => format!(
            "printf '%s' {prompt:?} > {prompt_file} \
             && cd {dir} && copilot --input-file {prompt_file}; rm -f {prompt_file}"
        ),
        // Unreachable: allowlist check above ensures only known values reach here
        _ => unreachable!("ALLOWED_CLI check must have passed"),
    };
    Ok(cmd)
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
#[path = "sse_delegate_util_tests.rs"]
mod tests;
