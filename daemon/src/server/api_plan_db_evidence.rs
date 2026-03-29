// Kernel evidence gate for task/update — Constitution Article VI.
// Extracted from api_plan_db.rs to stay ≤250 lines.
// WHY: "done" must be backed by evidence (build green, tests pass, files exist).

use super::state::ApiError;

/// Run the kernel evidence gate for a task status transition.
/// Only active when the `kernel` feature is enabled and not in test mode.
/// Returns Ok(()) if the gate passes, Err(ApiError::forbidden) if it blocks.
#[cfg(all(feature = "kernel", not(test)))]
pub(super) fn run_evidence_gate(
    conn: &rusqlite::Connection,
    task_id: i64,
    status: &str,
) -> Result<(), ApiError> {
    use crate::kernel::{engine::{KernelConfig, KernelEngine}, verify};
    use serde_json::{json, Value};

    let engine = KernelEngine::new(KernelConfig::default());

    // Resolve worktree path from the owning plan (best-effort; None is safe).
    let worktree: Option<String> = conn
        .query_row(
            "SELECT p.worktree_path \
             FROM tasks t JOIN plans p ON t.plan_id = p.id \
             WHERE t.id = ?1",
            rusqlite::params![task_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .unwrap_or(None);

    // Parse declared output files from output_data JSON (key "artifacts").
    let output_data_str: Option<String> = conn
        .query_row(
            "SELECT output_data FROM tasks WHERE id = ?1",
            rusqlite::params![task_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .unwrap_or(None);

    let artifact_strings: Vec<String> =
        output_data_str
            .as_deref()
            .and_then(|s| serde_json::from_str::<Value>(s).ok())
            .and_then(|v| v.get("artifacts").cloned())
            .and_then(|a| {
                a.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
            })
            .unwrap_or_default();
    let artifact_refs: Vec<&str> =
        artifact_strings.iter().map(String::as_str).collect();

    let report = verify::check_evidence(
        conn,
        &engine,
        task_id,
        status,
        worktree.as_deref(),
        &artifact_refs,
    );

    if !report.passed {
        let failed: Vec<serde_json::Value> = report
            .failed_checks()
            .iter()
            .map(|c| json!({"check": c.name, "detail": c.detail}))
            .collect();
        return Err(ApiError::forbidden(format!(
            "kernel evidence gate blocked task {} transition to '{}': {}",
            task_id,
            status,
            serde_json::to_string(&failed).unwrap_or_default(),
        )));
    }

    Ok(())
}
