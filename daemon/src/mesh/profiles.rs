// Profile management: TOML-based node profiles

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, ProfileError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: String,
    pub modules: Vec<String>,
}

/// Parses a TOML file into a Profile.
pub fn load_profile(path: &Path) -> Result<Profile> {
    let content = std::fs::read_to_string(path)?;
    let profile: Profile = toml::from_str(&content)?;
    Ok(profile)
}

/// Scans a directory for `.toml` files and returns all valid profiles.
pub fn list_profiles(dir: &Path) -> Vec<Profile> {
    let mut profiles = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return profiles,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            match load_profile(&path) {
                Ok(p) => profiles.push(p),
                Err(e) => eprintln!("Warning: failed to load profile {}: {}", path.display(), e),
            }
        }
    }

    profiles
}

/// Returns the path to the bundled profiles directory relative to `base`.
pub fn profiles_dir(base: &Path) -> PathBuf {
    base.join("profiles")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEV_MAC_TOML: &str = r#"
name = "dev-mac"
description = "Full macOS developer setup"
modules = ["brew", "vscode", "repos", "shell", "macos"]
"#;

    const CLAUDE_MESH_TOML: &str = r#"
name = "claude-mesh"
description = "Claude mesh node configuration"
modules = ["brew", "shell", "runners"]
"#;

    #[test]
    fn test_load_profile_toml() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("dev-mac.toml");
        std::fs::write(&p, DEV_MAC_TOML).unwrap();

        let profile = load_profile(&p).unwrap();
        assert_eq!(profile.name, "dev-mac");
        assert!(profile.modules.contains(&"brew".to_string()));
        assert!(profile.modules.contains(&"macos".to_string()));
    }

    #[test]
    fn test_list_profiles() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("dev-mac.toml"), DEV_MAC_TOML).unwrap();
        std::fs::write(tmp.path().join("claude-mesh.toml"), CLAUDE_MESH_TOML).unwrap();
        std::fs::write(tmp.path().join("notes.txt"), "not a profile").unwrap();

        let profiles = list_profiles(tmp.path());
        assert_eq!(profiles.len(), 2);
        let names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"dev-mac"));
        assert!(names.contains(&"claude-mesh"));
    }

    #[test]
    fn test_list_profiles_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles = list_profiles(tmp.path());
        assert!(profiles.is_empty());
    }
}
