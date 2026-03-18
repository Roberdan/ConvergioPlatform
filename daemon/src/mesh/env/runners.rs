// CI runner registration and configuration

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunnersError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, RunnersError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunnerConfig {
    pub name: String,
    pub path: PathBuf,
    pub labels: Vec<String>,
    pub repository: Option<String>,
    pub service_name: Option<String>,
}

/// Scans given paths for `.runner` JSON files and deserializes them.
pub fn scan_runners(paths: &[String]) -> Vec<RunnerConfig> {
    let mut runners = Vec::new();

    for path_str in paths {
        let dir = Path::new(path_str);
        let runner_file = dir.join(".runner");

        if runner_file.exists() {
            match parse_runner_file(&runner_file) {
                Ok(mut cfg) => {
                    cfg.path = dir.to_path_buf();
                    runners.push(cfg);
                }
                Err(e) => eprintln!("Warning: failed to parse {}: {}", runner_file.display(), e),
            }
        }
    }

    runners
}

fn parse_runner_file(path: &Path) -> Result<RunnerConfig> {
    let content = std::fs::read_to_string(path)?;
    let raw: serde_json::Value = serde_json::from_str(&content)?;

    let name = raw["name"].as_str().unwrap_or("unknown").to_string();
    let labels = raw["labels"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let repository = raw["repository"].as_str().map(|s| s.to_string());
    let service_name = raw["serviceName"].as_str().map(|s| s.to_string());

    Ok(RunnerConfig {
        name,
        path: path.parent().unwrap_or(Path::new(".")).to_path_buf(),
        labels,
        repository,
        service_name,
    })
}

/// Serializes runner configs to JSON.
pub fn export_runner_configs(runners: &[RunnerConfig]) -> Result<String> {
    Ok(serde_json::to_string_pretty(runners)?)
}

/// Deserializes runner configs from JSON.
pub fn import_runner_configs(json: &str) -> Result<Vec<RunnerConfig>> {
    Ok(serde_json::from_str(json)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_runners_empty() {
        let runners = scan_runners(&[]);
        assert!(runners.is_empty());
    }

    #[test]
    fn test_scan_runners_with_file() {
        let tmp = tempfile::tempdir().unwrap();
        let runner_json = serde_json::json!({
            "name": "my-runner",
            "labels": ["self-hosted", "macOS"],
            "repository": "org/repo",
            "serviceName": "github-actions-runner"
        });
        std::fs::write(tmp.path().join(".runner"), runner_json.to_string()).unwrap();

        let runners = scan_runners(&[tmp.path().to_string_lossy().to_string()]);
        assert_eq!(runners.len(), 1);
        assert_eq!(runners[0].name, "my-runner");
        assert_eq!(runners[0].labels, vec!["self-hosted", "macOS"]);
        assert_eq!(runners[0].repository.as_deref(), Some("org/repo"));
    }

    #[test]
    fn test_export_import_roundtrip() {
        let configs = vec![RunnerConfig {
            name: "runner-1".to_string(),
            path: PathBuf::from("/tmp/runner-1"),
            labels: vec!["self-hosted".to_string()],
            repository: Some("owner/repo".to_string()),
            service_name: None,
        }];

        let json = export_runner_configs(&configs).unwrap();
        let imported = import_runner_configs(&json).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].name, "runner-1");
    }
}
