// Autonomous execution policy — risk-based auto-progression control.
// Classifies tasks by risk level and determines whether human approval is required.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "LOW",
            RiskLevel::Medium => "MEDIUM",
            RiskLevel::High => "HIGH",
            RiskLevel::Critical => "CRITICAL",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "LOW" => Some(RiskLevel::Low),
            "MEDIUM" => Some(RiskLevel::Medium),
            "HIGH" => Some(RiskLevel::High),
            "CRITICAL" => Some(RiskLevel::Critical),
            _ => None,
        }
    }
}

/// Classify risk from task_type and effort_level (1–5 scale).
/// effort_level: 1=trivial, 2=small, 3=medium/sprint, 4=large, 5=epic
pub fn classify(task_type: &str, effort_level: u8) -> RiskLevel {
    let t = task_type.to_lowercase();

    // CRITICAL: data migrations or breaking changes regardless of effort
    if t.contains("migration") || t.contains("breaking") {
        return RiskLevel::Critical;
    }

    // HIGH: security, architecture, or high effort
    if t.contains("security") || t.contains("arch") || effort_level >= 3 {
        return RiskLevel::High;
    }

    // LOW: config, docs, tests, or effort 1
    if t.contains("config")
        || t.contains("doc")
        || t.contains("test")
        || effort_level <= 1
    {
        return RiskLevel::Low;
    }

    // MEDIUM: code + tests at effort 2
    RiskLevel::Medium
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    pub id: Option<i64>,
    pub project_id: String,
    pub risk_level: String,
    pub auto_progress: bool,
    pub require_human: bool,
    pub require_double_validation: bool,
}

impl ExecutionPolicy {
    /// Default policy for a risk level.
    pub fn default_for(project_id: &str, risk: RiskLevel) -> Self {
        let (auto_progress, require_human, require_double_validation) = match risk {
            RiskLevel::Low => (true, false, false),
            RiskLevel::Medium => (true, false, false),
            RiskLevel::High => (false, true, false),
            RiskLevel::Critical => (false, true, true),
        };
        ExecutionPolicy {
            id: None,
            project_id: project_id.to_string(),
            risk_level: risk.as_str().to_string(),
            auto_progress,
            require_human,
            require_double_validation,
        }
    }
}

pub fn ensure_table(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS execution_policy (
            id                       INTEGER PRIMARY KEY AUTOINCREMENT,
            project_id               TEXT    NOT NULL,
            risk_level               TEXT    NOT NULL,
            auto_progress            BOOLEAN NOT NULL DEFAULT 1,
            require_human            BOOLEAN NOT NULL DEFAULT 0,
            require_double_validation BOOLEAN NOT NULL DEFAULT 0,
            UNIQUE (project_id, risk_level)
        )",
    )
}

/// Load all policies for a project, inserting defaults where missing.
pub fn load_or_default(
    conn: &rusqlite::Connection,
    project_id: &str,
) -> rusqlite::Result<Vec<ExecutionPolicy>> {
    ensure_table(conn)?;

    // Seed defaults for any missing risk levels
    for risk in [RiskLevel::Low, RiskLevel::Medium, RiskLevel::High, RiskLevel::Critical] {
        let policy = ExecutionPolicy::default_for(project_id, risk);
        conn.execute(
            "INSERT OR IGNORE INTO execution_policy
             (project_id, risk_level, auto_progress, require_human, require_double_validation)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                policy.project_id,
                policy.risk_level,
                policy.auto_progress,
                policy.require_human,
                policy.require_double_validation,
            ],
        )?;
    }

    let mut stmt = conn.prepare(
        "SELECT id, project_id, risk_level, auto_progress, require_human, \
         require_double_validation FROM execution_policy WHERE project_id = ?1 \
         ORDER BY CASE risk_level WHEN 'LOW' THEN 0 WHEN 'MEDIUM' THEN 1 \
         WHEN 'HIGH' THEN 2 WHEN 'CRITICAL' THEN 3 ELSE 4 END",
    )?;

    let rows = stmt.query_map(rusqlite::params![project_id], |row| {
        Ok(ExecutionPolicy {
            id: row.get(0)?,
            project_id: row.get(1)?,
            risk_level: row.get(2)?,
            auto_progress: row.get(3)?,
            require_human: row.get(4)?,
            require_double_validation: row.get(5)?,
        })
    })?;

    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_critical() {
        assert_eq!(classify("data_migration", 1), RiskLevel::Critical);
        assert_eq!(classify("breaking_change", 2), RiskLevel::Critical);
    }

    #[test]
    fn test_classify_high() {
        assert_eq!(classify("security_audit", 2), RiskLevel::High);
        assert_eq!(classify("architecture_refactor", 3), RiskLevel::High);
        assert_eq!(classify("feature", 3), RiskLevel::High); // effort>=3
    }

    #[test]
    fn test_classify_low() {
        assert_eq!(classify("config_update", 1), RiskLevel::Low);
        assert_eq!(classify("docs", 2), RiskLevel::Low);
        assert_eq!(classify("test_fix", 2), RiskLevel::Low);
        assert_eq!(classify("anything", 1), RiskLevel::Low); // effort=1
    }

    #[test]
    fn test_classify_medium() {
        assert_eq!(classify("feature_code", 2), RiskLevel::Medium);
    }

    #[test]
    fn test_default_policy() {
        let p = ExecutionPolicy::default_for("proj1", RiskLevel::Critical);
        assert!(!p.auto_progress);
        assert!(p.require_human);
        assert!(p.require_double_validation);

        let p = ExecutionPolicy::default_for("proj1", RiskLevel::Low);
        assert!(p.auto_progress);
        assert!(!p.require_human);
        assert!(!p.require_double_validation);
    }

    #[test]
    fn test_risk_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
        assert!(RiskLevel::High < RiskLevel::Critical);
    }

    #[test]
    fn test_ensure_table_and_load() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let policies = load_or_default(&conn, "test-project").unwrap();
        assert_eq!(policies.len(), 4);
        let levels: Vec<&str> = policies.iter().map(|p| p.risk_level.as_str()).collect();
        assert!(levels.contains(&"LOW"));
        assert!(levels.contains(&"CRITICAL"));
    }
}
