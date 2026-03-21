// Private file-reading helpers for shell config export.

use std::path::{Path, PathBuf};

pub(super) fn read_file_opt(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

pub(super) fn extract_aliases(zshrc: &str) -> Vec<String> {
    zshrc
        .lines()
        .filter(|l| l.trim_start().starts_with("alias "))
        .map(|l| l.trim().to_string())
        .collect()
}

pub(super) fn read_dir_binary(dir: &Path) -> Option<Vec<(String, Vec<u8>)>> {
    if !dir.is_dir() {
        return None;
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        if p.is_file() {
            if let (Some(name), Ok(bytes)) = (
                p.file_name().map(|n| n.to_string_lossy().to_string()),
                std::fs::read(&p),
            ) {
                files.push((name, bytes));
            }
        }
    }
    if files.is_empty() {
        None
    } else {
        Some(files)
    }
}

pub(super) fn read_claude_config(dir: &Path) -> Option<Vec<(String, String)>> {
    if !dir.is_dir() {
        return None;
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        if p.is_file() {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            // Skip credential/secret files
            if name.contains("credential") || name.contains("secret") || name.contains(".db") {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&p) {
                files.push((name, content));
            }
        }
    }
    if files.is_empty() {
        None
    } else {
        Some(files)
    }
}

pub(super) fn create_tar_gz(dir: &Path) -> Option<Vec<u8>> {
    if !dir.is_dir() {
        return None;
    }
    std::process::Command::new("tar")
        .args(["czf", "-", "-C", &dir.to_string_lossy(), "."])
        .output()
        .ok()
        .filter(|o| o.status.success() && !o.stdout.is_empty())
        .map(|o| o.stdout)
}

pub(super) fn export_plist(domain: &str) -> Option<Vec<u8>> {
    let tmp = format!("/tmp/_convergiomesh_{}.plist", domain.replace('.', "_"));
    let ok = std::process::Command::new("defaults")
        .args(["export", domain, &tmp])
        .status()
        .ok()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        let data = std::fs::read(&tmp).ok();
        let _ = std::fs::remove_file(&tmp);
        data
    } else {
        None
    }
}

pub(super) fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
