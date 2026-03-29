// Private file-reading helpers for shell config export.

use std::path::{Path, PathBuf};

pub(super) fn read_file_opt(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(content) => Some(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            eprintln!("WARN: failed to read {}: {e}", path.display());
            None
        }
    }
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
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("WARN: failed to read dir {}: {e}", dir.display());
            return None;
        }
    };
    for entry in entries.flatten() {
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
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("WARN: failed to read claude config dir {}: {e}", dir.display());
            return None;
        }
    };
    for entry in entries.flatten() {
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
    match std::process::Command::new("tar")
        .args(["czf", "-", "-C", &dir.to_string_lossy(), "."])
        .output()
    {
        Ok(o) if o.status.success() && !o.stdout.is_empty() => Some(o.stdout),
        Ok(o) if !o.status.success() => {
            eprintln!("WARN: tar failed for {}: exit {}", dir.display(), o.status);
            None
        }
        Ok(_) => None, // empty output
        Err(e) => {
            eprintln!("WARN: tar command failed for {}: {e}", dir.display());
            None
        }
    }
}

pub(super) fn export_plist(domain: &str) -> Option<Vec<u8>> {
    let tmp = format!("/tmp/_convergiomesh_{}.plist", domain.replace('.', "_"));
    let ok = match std::process::Command::new("defaults")
        .args(["export", domain, &tmp])
        .status()
    {
        Ok(s) => s.success(),
        Err(e) => {
            eprintln!("WARN: defaults export {domain} failed: {e}");
            false
        }
    };
    if ok {
        let data = match std::fs::read(&tmp) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                eprintln!("WARN: failed to read plist {tmp}: {e}");
                None
            }
        };
        if let Err(e) = std::fs::remove_file(&tmp) {
            eprintln!("WARN: failed to remove temp plist {tmp}: {e}");
        }
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
