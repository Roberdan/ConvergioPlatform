// VS Code extensions and settings migration

use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VscodeError {
    #[error("code command failed: {0}")]
    CommandFailed(String),
    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, VscodeError>;

/// Runs `code --list-extensions` and returns the list.
pub fn export_extensions() -> Result<Vec<String>> {
    let output = Command::new("code").args(["--list-extensions"]).output()?;

    if !output.status.success() {
        return Err(VscodeError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let extensions = stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    Ok(extensions)
}

/// Installs each extension via `code --install-extension`.
pub fn import_extensions(extensions: &[String]) -> Result<()> {
    for ext in extensions {
        let status = Command::new("code")
            .args(["--install-extension", ext, "--force"])
            .status()?;

        if !status.success() {
            eprintln!("Warning: failed to install extension: {}", ext);
        }
    }
    Ok(())
}

fn vscode_settings_path() -> PathBuf {
    let home = dirs_home();
    home.join("Library/Application Support/Code/User/settings.json")
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Reads ~/Library/Application Support/Code/User/settings.json.
pub fn export_settings() -> Option<String> {
    let path = vscode_settings_path();
    std::fs::read_to_string(path).ok()
}

/// Writes settings.json to `target_home/Library/Application Support/Code/User/settings.json`.
pub fn import_settings(settings: &str, target_home: &std::path::Path) -> Result<()> {
    let dir = target_home.join("Library/Application Support/Code/User");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("settings.json"), settings)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_export_settings_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = r#"{"editor.fontSize": 14}"#;
        import_settings(settings, tmp.path()).unwrap();

        let written = std::fs::read_to_string(
            tmp.path()
                .join("Library/Application Support/Code/User/settings.json"),
        )
        .unwrap();
        assert_eq!(written, settings);
    }
}
