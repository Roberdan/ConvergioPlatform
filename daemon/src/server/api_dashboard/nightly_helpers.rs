// Nightly job helpers — extracted from nightly.rs (Plan F, T5-02).

use super::super::state::ApiError;
use serde_json::Value;
use std::collections::HashMap;

/// Validate project_id: only `[a-zA-Z0-9_-]` allowed.
/// Prevents path traversal when project_id is embedded in script filenames.
pub(super) fn validate_project_id(project_id: &str) -> Result<(), ApiError> {
    if project_id.is_empty() {
        return Err(ApiError::bad_request("project_id is required"));
    }
    if !project_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ApiError::bad_request(
            "project_id must match ^[a-zA-Z0-9_-]+$ (no path separators or special characters)",
        ));
    }
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::validate_project_id;

    #[test]
    fn test_validate_project_id_accepts_safe_values() {
        assert!(validate_project_id("my-project").is_ok());
        assert!(validate_project_id("project_123").is_ok());
        assert!(validate_project_id("convergio").is_ok());
        assert!(validate_project_id("abc-DEF-01").is_ok());
    }

    #[test]
    fn test_validate_project_id_rejects_path_traversal() {
        assert!(validate_project_id("../evil").is_err());
        assert!(validate_project_id("../../root").is_err());
        assert!(validate_project_id("/etc/passwd").is_err());
        assert!(validate_project_id("foo/bar").is_err());
        assert!(validate_project_id("foo bar").is_err());
        assert!(validate_project_id("$(rm -rf /)").is_err());
        assert!(validate_project_id("").is_err());
    }
}

pub(super) fn spawn_nightly_guardian(
    project_id: &str,
    trigger_source: &str,
    parent_run_id: Option<&str>,
) {
    // BUG-2: validate before using project_id as a script filename component
    if let Err(e) = validate_project_id(project_id) {
        eprintln!("[api_dashboard] rejected invalid project_id for nightly guardian: {e}");
        return;
    }

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

        let scripts_dir = claude_home.join("scripts");
        let script_name = format!("{project_id}-nightly-guardian.sh");
        let script_path = scripts_dir.join(&script_name);

        // BUG-2: verify the resolved script path is inside the expected scripts/ directory
        let canonical_scripts = match std::fs::canonicalize(&scripts_dir) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[api_dashboard] cannot canonicalize scripts dir: {e}");
                return;
            }
        };
        // script_path may not exist yet — canonicalize scripts_dir and check prefix
        // We already validated project_id contains only safe chars, but defense-in-depth
        let canonical_script = match std::fs::canonicalize(&script_path) {
            Ok(p) => p,
            Err(_) => {
                // Script does not exist — build expected canonical path to verify
                eprintln!(
                    "[api_dashboard] nightly guardian script not found: {} (project: {})",
                    script_path.display(),
                    project_id
                );
                return;
            }
        };
        if !canonical_script.starts_with(&canonical_scripts) {
            eprintln!(
                "[api_dashboard] script path escapes scripts dir — rejected: {}",
                canonical_script.display()
            );
            return;
        }

        let mut command = Command::new(canonical_script);
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
