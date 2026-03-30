use super::api_memory_mgmt::{parse_frontmatter, read_memory_entries};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct GarbageCollectResult {
    pub deleted: Vec<String>,
    pub archived: Vec<String>,
    pub kept: Vec<String>,
}

impl GarbageCollectResult {
    pub fn total_changed(&self) -> usize {
        self.deleted.len() + self.archived.len()
    }
}

pub fn garbage_collect_dir(dir: &Path) -> GarbageCollectResult {
    if !dir.exists() {
        return GarbageCollectResult::default();
    }
    let git_cwd = dir.parent().unwrap_or(dir);
    let git_log = std::process::Command::new("git")
        .args(["log", "--oneline", "-100", "--all"])
        .current_dir(git_cwd)
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
        .unwrap_or_default();
    let now = std::time::SystemTime::now();
    let thirty_days = std::time::Duration::from_secs(30 * 24 * 3600);
    let mut result = GarbageCollectResult::default();

    for (fname, content, path) in read_memory_entries(dir).unwrap_or_default() {
        let (name, mem_type, _) = parse_frontmatter(&content);
        // intentional: fs metadata is auxiliary for age-based tiering; unreadable files are kept.
        let is_old = fs::metadata(&path)
            .ok()
            .and_then(|meta| meta.modified().ok())
            .map(|modified| now.duration_since(modified).map(|age| age > thirty_days).unwrap_or(false))
            .unwrap_or(false);
        let ref_name = name.as_deref().unwrap_or(&fname);
        let in_git = git_log.contains(ref_name);

        if mem_type.as_deref() == Some("feedback") && in_git {
            if fs::remove_file(&path).is_ok() {
                result.deleted.push(fname);
            } else {
                result.kept.push(fname);
            }
        } else if is_old && !in_git {
            let archive_dir = dir.join(".archived");
            let target = archive_dir.join(&fname);
            if fs::create_dir_all(&archive_dir).is_ok() && fs::rename(&path, target).is_ok() {
                result.archived.push(fname);
            } else {
                result.kept.push(fname);
            }
        } else {
            result.kept.push(fname);
        }
    }

    result
}

pub fn garbage_collect_all_projects() -> Vec<(String, GarbageCollectResult)> {
    let base = std::env::var("CLAUDE_PROJECTS_DIR").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        format!("{home}/.claude/projects")
    });
    let mut results = Vec::new();
    let base_path = PathBuf::from(base);
    let Ok(entries) = fs::read_dir(base_path) else {
        return results;
    };
    for entry in entries.flatten() {
        let slug = entry.file_name().to_string_lossy().to_string();
        let dir = entry.path().join("memory");
        if !dir.is_dir() {
            continue;
        }
        results.push((slug, garbage_collect_dir(&dir)));
    }
    results
}

#[cfg(test)]
mod tests {
    use super::{garbage_collect_dir, GarbageCollectResult};
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    fn git(args: &[&str], dir: &std::path::Path) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .status()
            .expect("run git");
        assert!(status.success(), "git {:?} failed", args);
    }

    #[test]
    fn missing_directory_returns_empty_result() {
        let result = garbage_collect_dir(std::path::Path::new("/tmp/definitely-missing-memory-dir"));
        assert_eq!(result.total_changed(), 0);
        assert!(matches!(result, GarbageCollectResult { .. }));
    }

    #[test]
    fn garbage_collect_uses_project_git_history() {
        let root = tempdir().expect("tempdir");
        git(&["init"], root.path());
        git(&["config", "user.email", "tests@example.com"], root.path());
        git(&["config", "user.name", "Tests"], root.path());
        fs::write(root.path().join("tracked.txt"), "tracked\n").expect("write tracked");
        git(&["add", "tracked.txt"], root.path());
        git(&["commit", "-m", "remember tracked.txt"], root.path());

        let memory_dir = root.path().join("memory");
        fs::create_dir_all(&memory_dir).expect("create memory dir");
        fs::write(
            memory_dir.join("feedback.md"),
            "---\ntype: feedback\nname: tracked.txt\n---\nkeep or delete\n",
        )
        .expect("write memory file");

        let result = garbage_collect_dir(&memory_dir);
        assert_eq!(result.deleted, vec!["feedback.md".to_string()]);
    }
}
