// Git repository inventory and cloning

use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReposError {
    #[error("git command failed: {0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

pub type Result<T> = std::result::Result<T, ReposError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RepoInfo {
    pub name: String,
    pub path: PathBuf,
    pub remote_url: Option<String>,
    pub branch: Option<String>,
    pub has_changes: bool,
}

/// Scans a directory for git repos. For each subdirectory, reads the origin remote URL.
pub fn scan_github_dir(path: &Path) -> Vec<RepoInfo> {
    let mut repos = Vec::new();

    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return repos,
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }

        let git_dir = entry_path.join(".git");
        if !git_dir.exists() {
            continue;
        }

        let name = entry_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let remote_url = get_remote_url(&entry_path);
        let branch = get_current_branch(&entry_path);
        let has_changes = check_has_changes(&entry_path);

        repos.push(RepoInfo { name, path: entry_path, remote_url, branch, has_changes });
    }

    repos
}

fn get_remote_url(repo_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn get_current_branch(repo_path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch == "HEAD" {
            None
        } else {
            Some(branch)
        }
    } else {
        None
    }
}

fn check_has_changes(repo_path: &Path) -> bool {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output();

    match output {
        Ok(o) => !o.stdout.is_empty(),
        Err(_) => false,
    }
}

/// Clones each repo to `target / repo.name`.
pub fn clone_repos(repos: &[RepoInfo], target: &Path) -> Result<()> {
    std::fs::create_dir_all(target)?;

    for repo in repos {
        let Some(ref url) = repo.remote_url else { continue };
        let dest = target.join(&repo.name);
        if dest.exists() {
            eprintln!("Skipping {}: already exists", repo.name);
            continue;
        }

        let status = Command::new("git")
            .args(["clone", url, dest.to_string_lossy().as_ref()])
            .status()?;

        if !status.success() {
            return Err(ReposError::CommandFailed(format!("git clone {} failed", url)));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let repos = scan_github_dir(tmp.path());
        assert!(repos.is_empty());
    }

    #[test]
    fn test_scan_non_git_dir() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir(tmp.path().join("not-a-repo")).unwrap();
        let repos = scan_github_dir(tmp.path());
        assert!(repos.is_empty());
    }

    #[test]
    fn test_scan_git_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let repo_dir = tmp.path().join("my-repo");
        std::fs::create_dir(&repo_dir).unwrap();
        std::fs::create_dir(repo_dir.join(".git")).unwrap();

        let repos = scan_github_dir(tmp.path());
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "my-repo");
        assert!(repos[0].remote_url.is_none()); // no remote configured
    }
}
