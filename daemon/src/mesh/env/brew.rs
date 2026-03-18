// Homebrew package migration

use std::collections::HashMap;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BrewError {
    #[error("brew command failed: {0}")]
    CommandFailed(String),
    #[error("UTF-8 decode error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, BrewError>;

const ESSENTIAL: &[&str] = &["git", "curl", "jq", "sqlite3", "node", "rust", "wget", "openssl"];
const DEV: &[&str] = &["bat", "fd", "fzf", "gh", "lazygit", "delta", "ripgrep", "eza", "zoxide", "starship"];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BrewEntry {
    pub name: String,
    pub kind: EntryKind,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EntryKind {
    Tap,
    Formula,
    Cask,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Brewfile {
    pub taps: Vec<String>,
    pub formulae: HashMap<String, Category>,
    pub casks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Category {
    Essential,
    Dev,
    Optional,
}

impl Brewfile {
    pub fn categorize(&mut self) {
        for (name, cat) in self.formulae.iter_mut() {
            *cat = categorize_formula(name);
        }
    }
}

fn categorize_formula(name: &str) -> Category {
    if ESSENTIAL.contains(&name) {
        Category::Essential
    } else if DEV.contains(&name) {
        Category::Dev
    } else {
        Category::Optional
    }
}

/// Runs `brew bundle dump --file=/dev/stdout` and parses the output.
pub fn export_brewfile() -> Result<Brewfile> {
    let output = Command::new("brew")
        .args(["bundle", "dump", "--file=/dev/stdout"])
        .output()?;

    if !output.status.success() {
        return Err(BrewError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let stdout = String::from_utf8(output.stdout)?;
    parse_brewfile_output(&stdout)
}

fn parse_brewfile_output(content: &str) -> Result<Brewfile> {
    let mut taps = Vec::new();
    let mut formulae = HashMap::new();
    let mut casks = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = line.strip_prefix("tap ") {
            let name = rest.trim_matches('"').trim_matches('\'').to_string();
            taps.push(name);
        } else if let Some(rest) = line.strip_prefix("brew ") {
            // brew "name" or brew "name", args: [...]
            let name = rest
                .split(',')
                .next()
                .unwrap_or(rest)
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            let cat = categorize_formula(&name);
            formulae.insert(name, cat);
        } else if let Some(rest) = line.strip_prefix("cask ") {
            let name = rest.trim_matches('"').trim_matches('\'').to_string();
            casks.push(name);
        }
    }

    Ok(Brewfile { taps, formulae, casks })
}

/// Installs selected brew entries.
pub fn install_brewfile(brewfile: &Brewfile, selected: &[String]) -> Result<()> {
    for name in selected {
        if brewfile.formulae.contains_key(name.as_str()) {
            let status = Command::new("brew").args(["install", name]).status()?;
            if !status.success() {
                return Err(BrewError::CommandFailed(format!("brew install {} failed", name)));
            }
        } else if brewfile.casks.contains(name) {
            let status = Command::new("brew").args(["install", "--cask", name]).status()?;
            if !status.success() {
                return Err(BrewError::CommandFailed(format!(
                    "brew install --cask {} failed",
                    name
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_essential() {
        assert_eq!(categorize_formula("git"), Category::Essential);
        assert_eq!(categorize_formula("curl"), Category::Essential);
        assert_eq!(categorize_formula("node"), Category::Essential);
    }

    #[test]
    fn test_categorize_dev() {
        assert_eq!(categorize_formula("bat"), Category::Dev);
        assert_eq!(categorize_formula("gh"), Category::Dev);
        assert_eq!(categorize_formula("lazygit"), Category::Dev);
    }

    #[test]
    fn test_categorize_optional() {
        assert_eq!(categorize_formula("htop"), Category::Optional);
        assert_eq!(categorize_formula("tmux"), Category::Optional);
    }

    #[test]
    fn test_parse_brewfile_output() {
        let content = r#"
tap "homebrew/bundle"
brew "git"
brew "bat"
brew "htop"
cask "iterm2"
cask "visual-studio-code"
"#;
        let bf = parse_brewfile_output(content).unwrap();
        assert_eq!(bf.taps, vec!["homebrew/bundle"]);
        assert_eq!(bf.formulae[&"git".to_string()], Category::Essential);
        assert_eq!(bf.formulae[&"bat".to_string()], Category::Dev);
        assert_eq!(bf.formulae[&"htop".to_string()], Category::Optional);
        assert!(bf.casks.contains(&"iterm2".to_string()));
    }
}
