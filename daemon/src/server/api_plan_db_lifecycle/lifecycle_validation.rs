// Validation checks for plan lifecycle state transitions.
// Extracted from handlers.rs — pure query + guard logic.

use crate::server::state::{query_one, ApiError};
use rusqlite::Connection;
use serde_json::Value;

/// Verify all tasks in the plan are in a terminal state before completion.
/// Returns `Ok(())` or `Err` with a descriptive message.
pub(super) fn check_all_tasks_done(conn: &Connection, plan_id: i64) -> Result<(), ApiError> {
    let pending = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM tasks \
         WHERE plan_id = ?1 AND status NOT IN ('done', 'cancelled', 'skipped')",
        rusqlite::params![plan_id],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    if pending > 0 {
        return Err(ApiError::bad_request(format!(
            "plan {plan_id} has {pending} incomplete tasks"
        )));
    }
    Ok(())
}

/// Collect all worktree paths (plan + waves) for cleanup after completion.
pub(crate) fn worktree_cleanup_paths(conn: &Connection, plan_id: i64) -> Vec<String> {
    let mut paths = Vec::new();

    // Plan-level worktree
    if let Ok(Some(p)) = query_one(
        conn,
        "SELECT worktree_path AS p FROM plans WHERE id = ?1",
        rusqlite::params![plan_id],
    ) {
        if let Some(s) = p.get("p").and_then(Value::as_str) {
            if !s.is_empty() {
                paths.push(s.to_string());
            }
        }
    }

    // Wave-level worktrees
    let mut stmt = conn
        .prepare("SELECT worktree_path FROM waves WHERE plan_id = ?1 AND worktree_path IS NOT NULL AND worktree_path != ''")
        .unwrap_or_else(|_| conn.prepare("SELECT 1 WHERE 0").unwrap());
    if let Ok(rows) = stmt.query_map(rusqlite::params![plan_id], |row| {
        row.get::<_, String>(0)
    }) {
        for row in rows.flatten() {
            if !row.is_empty() {
                paths.push(row);
            }
        }
    }

    paths
}

/// Run git worktree prune, delete stale branches, and clean temp files.
/// Runs in a background task — failures are logged, never block the response.
pub(crate) fn run_post_complete_cleanup(plan_id: i64, wt_paths: &[String]) {
    // 1. git worktree prune
    if let Err(e) = std::process::Command::new("git")
        .args(["worktree", "prune"])
        .output()
    {
        tracing::warn!("plan {plan_id}: worktree prune failed: {e}");
    }

    // 2. Remove plan-specific temp files matching /tmp/convergio-plan-{id}*
    let prefix = format!("convergio-plan-{plan_id}");
    if let Ok(entries) = std::fs::read_dir("/tmp") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with(&prefix) {
                let path = entry.path();
                if path.is_dir() {
                    if let Err(e) = std::fs::remove_dir_all(&path) {
                        tracing::warn!("plan {plan_id}: temp dir cleanup failed {}: {e}", path.display());
                    }
                } else {
                    if let Err(e) = std::fs::remove_file(&path) {
                        tracing::warn!("plan {plan_id}: temp file cleanup failed {}: {e}", path.display());
                    }
                }
            }
        }
    }

    // 3. Delete stale worktree directories that still exist on disk
    for path in wt_paths {
        let p = std::path::Path::new(path);
        if p.exists() && p.is_dir() {
            let rm_result = std::process::Command::new("git")
                .args(["worktree", "remove", "--force", path])
                .output();
            if rm_result.is_err() || !rm_result.unwrap().status.success() {
                if let Err(e) = std::fs::remove_dir_all(p) {
                    tracing::warn!("plan {plan_id}: worktree dir cleanup failed {path}: {e}");
                }
            }
            tracing::info!("plan {plan_id}: cleaned worktree {path}");
        }
    }
}

/// Verify all done tasks were validated by Thor, not forced-admin.
/// forced-admin is an emergency bypass — plan completion requires real validation.
pub(super) fn check_thor_validated(conn: &Connection, plan_id: i64) -> Result<(), ApiError> {
    let forced = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM tasks \
         WHERE plan_id = ?1 AND status = 'done' \
         AND validated_by = 'forced-admin'",
        rusqlite::params![plan_id],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    if forced > 0 {
        return Err(ApiError::bad_request(format!(
            "plan {plan_id} has {forced} tasks validated by forced-admin, not Thor. \
             Re-validate with cvg plan validate {plan_id}."
        )));
    }
    Ok(())
}

/// Verify at least one wave has a merged PR (pr_url or pr_number set).
/// A plan is not complete until the code is in a PR and merged.
pub(super) fn check_pr_exists(conn: &Connection, plan_id: i64) -> Result<(), ApiError> {
    let has_pr = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM waves \
         WHERE plan_id = ?1 \
         AND (pr_url IS NOT NULL AND pr_url != '' \
              OR pr_number IS NOT NULL AND pr_number != '')",
        rusqlite::params![plan_id],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    if has_pr == 0 {
        return Err(ApiError::bad_request(format!(
            "plan {plan_id} has no PR linked to any wave. \
             Create a PR and record it before completing."
        )));
    }
    Ok(())
}

/// Verify all non-code deliverables linked to done tasks are approved.
/// Returns `Ok(())` or `Err` with a descriptive message.
pub(super) fn check_deliverables_approved(conn: &Connection, plan_id: i64) -> Result<(), ApiError> {
    let unapproved = query_one(
        conn,
        "SELECT COUNT(*) AS c FROM deliverables d \
         JOIN tasks t ON d.task_id = t.id \
         WHERE t.plan_id = ?1 AND t.status = 'done' \
         AND COALESCE(d.output_type, '') != 'pr' \
         AND d.status != 'approved'",
        rusqlite::params![plan_id],
    )?
    .and_then(|v| v.get("c").and_then(Value::as_i64))
    .unwrap_or(0);

    if unapproved > 0 {
        return Err(ApiError::bad_request(format!(
            "plan {plan_id} has {unapproved} unapproved non-code deliverables"
        )));
    }
    Ok(())
}
