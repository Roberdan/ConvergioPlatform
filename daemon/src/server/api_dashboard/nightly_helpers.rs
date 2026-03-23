// Nightly job helpers — extracted from nightly.rs (Plan F, T5-02).

use super::super::state::ApiError;
use serde_json::Value;
use std::collections::HashMap;

pub(super) fn parse_positive_i64(
    qs: &HashMap<String, String>,
    key: &str,
    default_value: i64,
) -> Result<i64, ApiError> {
    let value = qs
        .get(key)
        .map(|raw| {
            raw.parse::<i64>()
                .map_err(|_| ApiError::bad_request(format!("invalid {key}")))
        })
        .transpose()?
        .unwrap_or(default_value);
    if value < 1 {
        return Err(ApiError::bad_request(format!("{key} must be >= 1")));
    }
    Ok(value)
}

pub(super) fn parse_json_text_field(row: &mut Value, field: &str) -> Result<(), ApiError> {
    let raw = row
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if let Some(raw) = raw {
        let parsed = serde_json::from_str::<Value>(&raw)
            .map_err(|err| ApiError::internal(format!("invalid {field}: {err}")))?;
        if let Some(object) = row.as_object_mut() {
            object.insert(field.to_string(), parsed);
        }
    }
    Ok(())
}

pub(super) fn spawn_nightly_guardian(
    project_id: &str,
    trigger_source: &str,
    parent_run_id: Option<&str>,
) {
    #[cfg(test)]
    {
        let _ = (project_id, trigger_source, parent_run_id);
    }

    #[cfg(not(test))]
    {
        use std::env;
        use std::process::Command;

        let claude_home = env::var("CLAUDE_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                env::var("HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .join(".claude")
            });
        let script_name = format!("{project_id}-nightly-guardian.sh");
        let script_path = claude_home.join(format!("scripts/{script_name}"));
        if !script_path.exists() {
            eprintln!(
                "[api_dashboard] nightly guardian script not found: {} (project: {})",
                script_path.display(),
                project_id
            );
            return;
        }

        let mut command = Command::new(script_path);
        command
            .arg(format!("--trigger={trigger_source}"))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Some(parent_run_id) = parent_run_id.filter(|value| !value.is_empty()) {
            command.arg(format!("--parent-run-id={parent_run_id}"));
        }
        if let Err(err) = command.spawn() {
            eprintln!("[api_dashboard] failed to spawn nightly guardian for {project_id}: {err}");
        }
    }
}
