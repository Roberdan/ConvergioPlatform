use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum MergeMethod {
    #[default]
    Squash,
    Merge,
    Rebase,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: i64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrReadiness {
    pub mergeable: bool,
    pub ci_passed: bool,
    pub pending_checks: i64,
    pub unresolved_threads: i64,
    pub review_status: String,
}

pub trait GitConnector: Send + Sync {
    fn commit(&self, path: &Path, message: &str) -> Result<String, String>;
    fn push(&self, path: &Path, branch: &str, force_with_lease: bool) -> Result<(), String>;
    fn create_pr(
        &self,
        repo: &str,
        branch: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<PrInfo, String>;
    fn merge_pr(&self, repo: &str, pr_number: i64, method: MergeMethod) -> Result<(), String>;
    fn pr_readiness(&self, repo: &str, pr_number: i64) -> Result<PrReadiness, String>;
    fn rebase(&self, path: &Path, onto: &str) -> Result<(), String>;
}

pub struct GitHubConnector {
    pub github_token: String,
}

fn run_git(path: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .map_err(|e| format!("git spawn failed: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

impl GitConnector for GitHubConnector {
    fn commit(&self, path: &Path, message: &str) -> Result<String, String> {
        run_git(path, &["add", "-A"])?;
        // Commit may fail if nothing to commit — propagate error
        run_git(path, &["commit", "-m", message])?;
        run_git(path, &["rev-parse", "HEAD"])
    }

    fn push(&self, path: &Path, branch: &str, force_with_lease: bool) -> Result<(), String> {
        let mut args = vec!["push", "-u", "origin", branch];
        if force_with_lease {
            // Insert before positional args
            args.insert(1, "--force-with-lease");
        }
        run_git(path, &args)?;
        Ok(())
    }

    fn rebase(&self, path: &Path, onto: &str) -> Result<(), String> {
        run_git(path, &["rebase", onto])?;
        Ok(())
    }

    fn create_pr(
        &self,
        repo: &str,
        branch: &str,
        base: &str,
        title: &str,
        body: &str,
    ) -> Result<PrInfo, String> {
        let client = reqwest::blocking::Client::new();
        let url = format!("https://api.github.com/repos/{repo}/pulls");
        let resp = client
            .post(&url)
            .header("Authorization", format!("token {}", self.github_token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "convergio-daemon")
            .json(&serde_json::json!({
                "title": title,
                "body": body,
                "head": branch,
                "base": base,
            }))
            .send()
            .map_err(|e| format!("PR create request failed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("PR create failed ({status}): {body}"));
        }
        let json: serde_json::Value = resp.json().map_err(|e| format!("PR parse failed: {e}"))?;
        Ok(PrInfo {
            number: json["number"].as_i64().unwrap_or(0),
            url: json["html_url"].as_str().unwrap_or("").to_string(),
        })
    }

    fn merge_pr(&self, repo: &str, pr_number: i64, method: MergeMethod) -> Result<(), String> {
        let client = reqwest::blocking::Client::new();
        let url = format!("https://api.github.com/repos/{repo}/pulls/{pr_number}/merge");
        let method_str = match method {
            MergeMethod::Squash => "squash",
            MergeMethod::Merge => "merge",
            MergeMethod::Rebase => "rebase",
        };
        let resp = client
            .put(&url)
            .header("Authorization", format!("token {}", self.github_token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "convergio-daemon")
            .json(&serde_json::json!({"merge_method": method_str}))
            .send()
            .map_err(|e| format!("PR merge request failed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("PR merge failed ({status}): {body}"));
        }
        Ok(())
    }

    fn pr_readiness(&self, repo: &str, pr_number: i64) -> Result<PrReadiness, String> {
        let client = reqwest::blocking::Client::new();
        let url = format!("https://api.github.com/repos/{repo}/pulls/{pr_number}");
        let resp = client
            .get(&url)
            .header("Authorization", format!("token {}", self.github_token))
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "convergio-daemon")
            .send()
            .map_err(|e| format!("PR readiness request failed: {e}"))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(format!("PR readiness failed ({status}): {body}"));
        }
        let json: serde_json::Value = resp.json().map_err(|e| format!("PR parse failed: {e}"))?;
        Ok(PrReadiness {
            mergeable: json["mergeable"].as_bool().unwrap_or(false),
            ci_passed: json["mergeable_state"].as_str() == Some("clean"),
            pending_checks: 0,
            unresolved_threads: 0,
            review_status: json["mergeable_state"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn merge_method_default_is_squash() {
        assert!(matches!(MergeMethod::default(), MergeMethod::Squash));
    }

    #[test]
    fn pr_readiness_construction() {
        let r = PrReadiness {
            mergeable: true,
            ci_passed: true,
            pending_checks: 0,
            unresolved_threads: 0,
            review_status: "clean".into(),
        };
        assert!(r.mergeable);
        assert!(r.ci_passed);
    }

    #[test]
    fn run_git_in_temp_repo() {
        let dir = tempdir().unwrap();
        run_git(dir.path(), &["init"]).unwrap();
        run_git(dir.path(), &["config", "user.email", "test@example.com"]).unwrap();
        run_git(dir.path(), &["config", "user.name", "Test"]).unwrap();
        fs::write(dir.path().join("test.txt"), "hello").unwrap();
        run_git(dir.path(), &["add", "."]).unwrap();
        run_git(dir.path(), &["commit", "-m", "init"]).unwrap();
        let sha = run_git(dir.path(), &["rev-parse", "HEAD"]).unwrap();
        assert_eq!(sha.len(), 40);
    }

    #[test]
    fn commit_returns_sha() {
        let dir = tempdir().unwrap();
        run_git(dir.path(), &["init"]).unwrap();
        run_git(dir.path(), &["config", "user.email", "test@example.com"]).unwrap();
        run_git(dir.path(), &["config", "user.name", "Test"]).unwrap();
        fs::write(dir.path().join("file.txt"), "content").unwrap();
        let connector = GitHubConnector {
            github_token: "fake".into(),
        };
        let sha = connector.commit(dir.path(), "test commit").unwrap();
        assert_eq!(sha.len(), 40);
    }

    #[test]
    fn pr_info_fields_accessible() {
        let info = PrInfo {
            number: 42,
            url: "https://github.com/example/repo/pull/42".into(),
        };
        assert_eq!(info.number, 42);
        assert!(!info.url.is_empty());
    }

    #[test]
    fn merge_method_variants_serialize() {
        let squash = serde_json::to_string(&MergeMethod::Squash).unwrap();
        let merge = serde_json::to_string(&MergeMethod::Merge).unwrap();
        let rebase = serde_json::to_string(&MergeMethod::Rebase).unwrap();
        assert!(squash.contains("Squash"));
        assert!(merge.contains("Merge"));
        assert!(rebase.contains("Rebase"));
    }

    #[test]
    fn run_git_invalid_dir_returns_error() {
        let result = run_git(Path::new("/nonexistent_path_xyz"), &["status"]);
        assert!(result.is_err());
    }
}
