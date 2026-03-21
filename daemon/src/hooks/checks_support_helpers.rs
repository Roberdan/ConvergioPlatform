use serde::Deserialize;

pub fn contains_any(command: &str, values: &[&str]) -> bool {
    values.iter().any(|value| command.contains(value))
}

pub fn extract_base_cmd(command: &str) -> String {
    let mut base = command.split('|').next().unwrap_or_default().trim();
    base = base.rsplit("&&").next().unwrap_or(base).trim();
    base = base.rsplit(';').next().unwrap_or(base).trim();
    base.to_string()
}

pub fn extract_plan_id(command: &str) -> Option<u64> {
    let parts: Vec<_> = command.split_whitespace().collect();
    for window in parts.windows(3) {
        if window[0] == "plan-db.sh" && window[1] == "start" {
            return window[2].parse::<u64>().ok();
        }
    }
    for (idx, value) in parts.iter().enumerate() {
        if (*value == "execute-plan.sh" || *value == "copilot-worker.sh") && idx + 1 < parts.len() {
            return parts[idx + 1].parse::<u64>().ok();
        }
    }
    None
}

pub fn select_account(
    config: &GhAccountsConfig,
    cwd: &std::path::Path,
    home: &std::path::Path,
) -> Option<String> {
    let mut selected = config.default_account.clone().unwrap_or_default();
    let mut best_len = 0usize;
    for mapping in &config.mappings {
        let expanded = mapping.path.replace('~', &home.to_string_lossy());
        let path = std::path::PathBuf::from(expanded);
        if (cwd == path || cwd.starts_with(&path)) && path.as_os_str().len() > best_len {
            selected = mapping.account.clone();
            best_len = path.as_os_str().len();
        }
    }
    if selected.is_empty() {
        None
    } else {
        Some(selected)
    }
}

#[derive(Debug, Deserialize)]
pub struct PreflightSnapshot {
    #[serde(default)]
    pub generated_epoch: i64,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GhAccountsConfig {
    #[serde(default)]
    pub default_account: Option<String>,
    #[serde(default)]
    pub mappings: Vec<GhMapping>,
}

#[derive(Debug, Deserialize)]
pub struct GhMapping {
    pub path: String,
    pub account: String,
}

pub const INFRA_DRIFT_QUERY: &str = "SELECT COUNT(*) AS pending,
  COALESCE((SELECT group_concat(task_id || ': ' || title, char(10)) FROM (
    SELECT t.task_id, t.title FROM tasks t JOIN plans p ON t.plan_id=p.id
    WHERE p.status='doing' AND t.status IN ('pending','in_progress')
      AND (t.title LIKE '%Azure%' OR t.title LIKE '%Bicep%' OR t.title LIKE '%ACR%' OR t.title LIKE '%Container%'
        OR t.title LIKE '%Redis%' OR t.title LIKE '%PostgreSQL%' OR t.title LIKE '%Key Vault%' OR t.title LIKE '%Storage%'
        OR t.title LIKE '%MI %' OR t.title LIKE '%Managed Identity%' OR t.title LIKE '%deploy%' OR t.title LIKE '%provision%')
    LIMIT 5)), '') AS tasks
  FROM tasks t JOIN plans p ON t.plan_id=p.id
  WHERE p.status='doing' AND t.status IN ('pending','in_progress')
    AND (t.title LIKE '%Azure%' OR t.title LIKE '%Bicep%' OR t.title LIKE '%ACR%' OR t.title LIKE '%Container%'
      OR t.title LIKE '%Redis%' OR t.title LIKE '%PostgreSQL%' OR t.title LIKE '%Key Vault%' OR t.title LIKE '%Storage%'
      OR t.title LIKE '%MI %' OR t.title LIKE '%Managed Identity%' OR t.title LIKE '%deploy%' OR t.title LIKE '%provision%')";
