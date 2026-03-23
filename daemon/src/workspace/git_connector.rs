// Why: Plan 706 T3-01 — async HTTP methods + GitError for better error handling.
// Use Pin<Box<dyn Future>> in trait for dyn-compatibility without async_trait crate.
#[path = "git_connector_types.rs"]
mod git_connector_types;
pub use git_connector_types::{AsyncResult, GitError, MergeMethod, PrInfo, PrReadiness};

use std::path::Path;
use std::process::Command;
use std::time::Duration;

const GITHUB_TIMEOUT: Duration = Duration::from_secs(30);

/// Git + GitHub operations interface. Sync methods return GitError, async return GitError via future.
pub trait GitConnector: Send + Sync {
    fn commit(&self, path: &Path, message: &str) -> Result<String, GitError>;
    fn push(&self, path: &Path, branch: &str, force_with_lease: bool) -> Result<(), GitError>;
    fn create_pr<'a>(
        &'a self,
        repo: &'a str,
        branch: &'a str,
        base: &'a str,
        title: &'a str,
        body: &'a str,
    ) -> AsyncResult<'a, PrInfo>;
    fn merge_pr<'a>(
        &'a self,
        repo: &'a str,
        pr_number: i64,
        method: MergeMethod,
    ) -> AsyncResult<'a, ()>;
    fn pr_readiness<'a>(&'a self, repo: &'a str, pr_number: i64) -> AsyncResult<'a, PrReadiness>;
    fn rebase(&self, path: &Path, onto: &str) -> Result<(), GitError>;
}

pub struct GitHubConnector {
    pub github_token: String,
}

fn run_git(path: &Path, args: &[&str]) -> Result<String, GitError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .map_err(|e| GitError::Git(format!("git spawn failed: {e}")))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(GitError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

impl GitConnector for GitHubConnector {
    fn commit(&self, path: &Path, message: &str) -> Result<String, GitError> {
        run_git(path, &["add", "-A"])?;
        run_git(path, &["commit", "-m", message])?;
        run_git(path, &["rev-parse", "HEAD"])
    }

    fn push(&self, path: &Path, branch: &str, force_with_lease: bool) -> Result<(), GitError> {
        let mut args = vec!["push", "-u", "origin", branch];
        if force_with_lease {
            args.insert(1, "--force-with-lease");
        }
        run_git(path, &args)?;
        Ok(())
    }

    fn rebase(&self, path: &Path, onto: &str) -> Result<(), GitError> {
        run_git(path, &["rebase", onto])?;
        Ok(())
    }

    fn create_pr<'a>(
        &'a self,
        repo: &'a str,
        branch: &'a str,
        base: &'a str,
        title: &'a str,
        body: &'a str,
    ) -> AsyncResult<'a, PrInfo> {
        Box::pin(async move {
            let client = reqwest::Client::builder()
                .timeout(GITHUB_TIMEOUT)
                .build()?;
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
                .await?;
            if !resp.status().is_success() {
                let status = resp.status().as_u16();
                let body = resp.text().await.unwrap_or_default();
                return Err(GitError::Api { status, body });
            }
            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| GitError::Parse(e.to_string()))?;
            Ok(PrInfo {
                number: json["number"].as_i64().unwrap_or(0),
                url: json["html_url"].as_str().unwrap_or("").to_string(),
            })
        })
    }

    fn merge_pr<'a>(
        &'a self,
        repo: &'a str,
        pr_number: i64,
        method: MergeMethod,
    ) -> AsyncResult<'a, ()> {
        Box::pin(async move {
            let client = reqwest::Client::builder()
                .timeout(GITHUB_TIMEOUT)
                .build()?;
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
                .await?;
            if !resp.status().is_success() {
                let status = resp.status().as_u16();
                let body = resp.text().await.unwrap_or_default();
                return Err(GitError::Api { status, body });
            }
            Ok(())
        })
    }

    fn pr_readiness<'a>(&'a self, repo: &'a str, pr_number: i64) -> AsyncResult<'a, PrReadiness> {
        Box::pin(async move {
            let client = reqwest::Client::builder()
                .timeout(GITHUB_TIMEOUT)
                .build()?;
            let url = format!("https://api.github.com/repos/{repo}/pulls/{pr_number}");
            let resp = client
                .get(&url)
                .header("Authorization", format!("token {}", self.github_token))
                .header("Accept", "application/vnd.github.v3+json")
                .header("User-Agent", "convergio-daemon")
                .send()
                .await?;
            if !resp.status().is_success() {
                let status = resp.status().as_u16();
                let body = resp.text().await.unwrap_or_default();
                return Err(GitError::Api { status, body });
            }
            let json: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| GitError::Parse(e.to_string()))?;
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
        })
    }
}

#[cfg(test)]
#[path = "git_connector_tests.rs"]
mod tests;
