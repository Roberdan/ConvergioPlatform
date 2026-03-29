// Path-security helpers for the agent catalog API.
// Extracted from api_agent_catalog.rs to stay ≤250 lines.
// WHY: these validations must run before any filesystem operation on agent files.

use super::state::ApiError;
use std::path::Path;

/// Validate agent name: only `[a-zA-Z0-9_-]` allowed.
/// Rejects path traversal sequences like `../`, absolute paths, and shell metacharacters.
pub(super) fn validate_agent_name(name: &str) -> Result<(), ApiError> {
    if name.is_empty() {
        return Err(ApiError::bad_request("name is required"));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(ApiError::bad_request(
            "name must match ^[a-zA-Z0-9_-]+$ (no path separators or special characters)",
        ));
    }
    Ok(())
}

/// Verify that `child` is strictly inside `parent` after canonicalization.
/// Prevents path traversal even when `name` passes the regex check.
pub(super) fn assert_path_under(parent: &Path, child: &Path) -> Result<(), ApiError> {
    let canonical_parent = std::fs::canonicalize(parent)
        .map_err(|e| ApiError::bad_request(format!("invalid target_dir: {e}")))?;
    // child may not exist yet — canonicalize its parent directory instead
    let child_dir = child.parent().unwrap_or(child);
    let canonical_child_dir = std::fs::canonicalize(child_dir)
        .map_err(|e| ApiError::bad_request(format!("invalid target path: {e}")))?;
    if !canonical_child_dir.starts_with(&canonical_parent) {
        return Err(ApiError::bad_request(
            "resolved path escapes target_dir — request rejected",
        ));
    }
    Ok(())
}
