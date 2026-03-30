// Approval UX — reason codes, batch approval, and cache for skip-if-seen-before.
// migrate() must be called once at daemon start before any other function.

use rusqlite::{params, Connection};

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasonCode {
    RiskLevel,
    FileScope,
    SecurityImpact,
    BreakingChange,
}

impl ReasonCode {
    fn as_str(&self) -> &'static str {
        match self {
            Self::RiskLevel => "risk_level",
            Self::FileScope => "file_scope",
            Self::SecurityImpact => "security_impact",
            Self::BreakingChange => "breaking_change",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub reason_code: ReasonCode,
    pub task_id: String,
    pub description: String,
    pub previously_approved: bool,
}

/// Minimal task context needed to classify the approval reason.
#[derive(Debug, Clone)]
pub struct TaskInfo {
    /// Logical type: "security", "refactor", "breaking", "feature", etc.
    pub task_type: String,
    /// Effort estimate: "XS", "S", "M", "L", "XL".
    pub effort: String,
    /// Paths of files expected to be touched.
    pub files: Vec<String>,
}

// ── Migration ─────────────────────────────────────────────────────────────────

/// Create approval_cache table — idempotent (safe to call on every startup).
pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS approval_cache (
            id          INTEGER PRIMARY KEY,
            reason_code TEXT NOT NULL,
            task_type   TEXT NOT NULL,
            approved_by TEXT NOT NULL,
            created_at  TEXT DEFAULT (datetime('now')),
            UNIQUE (reason_code, task_type)
        );
        CREATE TABLE IF NOT EXISTS batch_approvals (
            id          INTEGER PRIMARY KEY,
            task_id     TEXT NOT NULL,
            approved_by TEXT NOT NULL,
            created_at  TEXT DEFAULT (datetime('now')),
            UNIQUE (task_id)
        );",
    )
}

// ── Classification ────────────────────────────────────────────────────────────

/// Infer the most relevant approval reason from task metadata.
///
/// Priority: SecurityImpact > BreakingChange > FileScope > RiskLevel.
pub fn classify_reason(task: &TaskInfo) -> ReasonCode {
    let lower = task.task_type.to_lowercase();

    if lower.contains("security") || lower.contains("auth") || lower.contains("crypto") {
        return ReasonCode::SecurityImpact;
    }

    if lower.contains("breaking") || lower.contains("migration") || lower.contains("schema") {
        return ReasonCode::BreakingChange;
    }

    // Many files touched or large effort → FileScope
    let large_effort = matches!(task.effort.as_str(), "L" | "XL");
    let many_files = task.files.len() >= 5;
    if large_effort || many_files {
        return ReasonCode::FileScope;
    }

    ReasonCode::RiskLevel
}

// ── Cache ─────────────────────────────────────────────────────────────────────

/// Return `true` if this (reason_code, task_type) pair was approved before.
///
/// Callers should skip the interactive prompt when this returns `true`.
pub fn check_approval_cache(
    conn: &Connection,
    reason_code: &ReasonCode,
    task_type: &str,
) -> bool {
    conn.query_row(
        "SELECT 1 FROM approval_cache WHERE reason_code = ?1 AND task_type = ?2 LIMIT 1",
        params![reason_code.as_str(), task_type],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

/// Persist an approval so future identical patterns are auto-approved.
pub fn record_approval(
    conn: &Connection,
    reason_code: &ReasonCode,
    task_type: &str,
    approved_by: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO approval_cache (reason_code, task_type, approved_by)
         VALUES (?1, ?2, ?3)",
        params![reason_code.as_str(), task_type, approved_by],
    )?;
    Ok(())
}

// ── Batch approval ────────────────────────────────────────────────────────────

/// Approve multiple tasks at once, recording each in `batch_approvals`.
///
/// Uses a transaction so all-or-nothing semantics apply.
pub fn approve_batch(
    conn: &Connection,
    task_ids: &[&str],
    approved_by: &str,
) -> rusqlite::Result<()> {
    conn.execute_batch("BEGIN")?;
    for task_id in task_ids {
        conn.execute(
            "INSERT OR REPLACE INTO batch_approvals (task_id, approved_by) VALUES (?1, ?2)",
            params![task_id, approved_by],
        )?;
    }
    conn.execute_batch("COMMIT")?;
    Ok(())
}

/// Build an `ApprovalRequest` for a task, consulting the cache.
pub fn build_request(
    conn: &Connection,
    task: &TaskInfo,
    task_id: impl Into<String>,
    description: impl Into<String>,
) -> ApprovalRequest {
    let reason_code = classify_reason(task);
    let previously_approved =
        check_approval_cache(conn, &reason_code, &task.task_type);
    ApprovalRequest {
        reason_code,
        task_id: task_id.into(),
        description: description.into(),
        previously_approved,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn task(task_type: &str, effort: &str, files: usize) -> TaskInfo {
        TaskInfo {
            task_type: task_type.to_string(),
            effort: effort.to_string(),
            files: (0..files).map(|i| format!("file{i}.rs")).collect(),
        }
    }

    #[test]
    fn test_classify_security() {
        assert_eq!(classify_reason(&task("security_audit", "S", 1)), ReasonCode::SecurityImpact);
        assert_eq!(classify_reason(&task("auth_refactor", "M", 2)), ReasonCode::SecurityImpact);
    }

    #[test]
    fn test_classify_breaking() {
        assert_eq!(classify_reason(&task("breaking_api_change", "M", 2)), ReasonCode::BreakingChange);
        assert_eq!(classify_reason(&task("schema_migration", "S", 1)), ReasonCode::BreakingChange);
    }

    #[test]
    fn test_classify_file_scope_effort() {
        assert_eq!(classify_reason(&task("feature", "L", 1)), ReasonCode::FileScope);
        assert_eq!(classify_reason(&task("feature", "XL", 0)), ReasonCode::FileScope);
    }

    #[test]
    fn test_classify_file_scope_count() {
        assert_eq!(classify_reason(&task("feature", "S", 5)), ReasonCode::FileScope);
    }

    #[test]
    fn test_classify_risk_level_default() {
        assert_eq!(classify_reason(&task("feature", "S", 1)), ReasonCode::RiskLevel);
    }

    #[test]
    fn test_cache_miss_then_hit() {
        let conn = setup();
        assert!(!check_approval_cache(&conn, &ReasonCode::RiskLevel, "feature"));
        record_approval(&conn, &ReasonCode::RiskLevel, "feature", "alice").unwrap();
        assert!(check_approval_cache(&conn, &ReasonCode::RiskLevel, "feature"));
    }

    #[test]
    fn test_cache_idempotent_record() {
        let conn = setup();
        record_approval(&conn, &ReasonCode::FileScope, "refactor", "bob").unwrap();
        record_approval(&conn, &ReasonCode::FileScope, "refactor", "charlie").unwrap();
        assert!(check_approval_cache(&conn, &ReasonCode::FileScope, "refactor"));
    }

    #[test]
    fn test_approve_batch() {
        let conn = setup();
        approve_batch(&conn, &["T1", "T2", "T3"], "alice").unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM batch_approvals", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_build_request_previously_approved() {
        let conn = setup();
        let t = task("feature", "S", 1);
        record_approval(&conn, &ReasonCode::RiskLevel, "feature", "alice").unwrap();
        let req = build_request(&conn, &t, "T-42", "some desc");
        assert!(req.previously_approved);
        assert_eq!(req.task_id, "T-42");
    }
}
