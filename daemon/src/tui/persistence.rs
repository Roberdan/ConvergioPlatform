// Session persistence for TUI — saves/loads last selected project.
// Uses ~/.claude/data/last_project.txt (plain text, one project id per line).

use std::path::{Path, PathBuf};

/// Returns the canonical path for the last-project persistence file.
pub fn last_project_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".claude").join("data").join("last_project.txt")
}

/// Write project id to the given path (creates parent dirs if needed).
pub fn save_last_project_to(path: &Path, project_id: &str) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, project_id);
}

/// Write project id to the canonical persistence path.
pub fn save_last_project(project_id: &str) {
    save_last_project_to(&last_project_path(), project_id);
}

/// Read project id from the given path; returns None if file missing or unreadable.
pub fn load_last_project_from(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Read project id from the canonical persistence path.
pub fn load_last_project() -> Option<String> {
    load_last_project_from(&last_project_path())
}
