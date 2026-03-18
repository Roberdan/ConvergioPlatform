// Shell configuration (zsh, starship, ghostty, claude) migration

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShellError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ShellError>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ShellConfig {
    pub zshrc: Option<String>,
    pub starship_toml: Option<String>,
    pub aliases: Vec<String>,
    pub shell_aliases_sh: Option<String>,
    pub ghostty_config: Option<String>,
    pub claude_config_dir: Option<Vec<(String, String)>>, // (filename, content)
    // App configs (themes + keybindings)
    pub warp_themes: Option<Vec<(String, Vec<u8>)>>,      // (filename, bytes) — yaml + png
    pub warp_keybindings: Option<String>,
    pub zed_settings: Option<String>,
    pub zed_keymap: Option<String>,
    pub vscode_settings: Option<String>,
    pub vscode_keybindings: Option<String>,
    pub vscode_extensions: Vec<String>,
    // Fonts (tarball)
    pub fonts_tar_gz: Option<Vec<u8>>,
    // macOS plists
    pub screenshot_plist: Option<Vec<u8>>,
    pub symbolic_hotkeys_plist: Option<Vec<u8>>,
}

fn read_file_opt(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn extract_aliases(zshrc: &str) -> Vec<String> {
    zshrc
        .lines()
        .filter(|l| l.trim_start().starts_with("alias "))
        .map(|l| l.trim().to_string())
        .collect()
}

/// Reads all shell config files from the current user's home.
pub fn export_shell_config() -> Result<ShellConfig> {
    let home = home_dir();
    export_shell_config_from(&home)
}

pub fn export_shell_config_from(home: &Path) -> Result<ShellConfig> {
    let zshrc = read_file_opt(&home.join(".zshrc"));
    let aliases = zshrc.as_deref().map(extract_aliases).unwrap_or_default();
    let shell_aliases_sh = read_file_opt(&home.join(".claude/shell-aliases.sh"));
    let starship_toml = read_file_opt(&home.join(".config/starship.toml"));
    let ghostty_config = read_file_opt(&home.join(".config/ghostty/config"));

    // App configs
    let warp_keybindings = read_file_opt(&home.join(".warp/keybindings.yaml"));
    let zed_settings = read_file_opt(&home.join(".config/zed/settings.json"));
    let zed_keymap = read_file_opt(&home.join(".config/zed/keymap.json"));
    let vscode_settings = read_file_opt(&home.join("Library/Application Support/Code/User/settings.json"));
    let vscode_keybindings = read_file_opt(&home.join("Library/Application Support/Code/User/keybindings.json"));

    // VSCode extensions list
    let vscode_extensions = std::process::Command::new("code")
        .args(["--list-extensions"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(|l| l.to_string()).collect())
        .unwrap_or_default();

    // Warp themes (yaml + png files)
    let warp_themes = read_dir_binary(&home.join(".warp/themes"));

    // Fonts tarball
    let fonts_tar_gz = create_tar_gz(&home.join("Library/Fonts"));

    // .claude config dir (non-credential files only)
    let claude_config_dir = read_claude_config(&home.join(".claude"));

    // macOS plists (via defaults export)
    let screenshot_plist = export_plist("com.apple.screencapture");
    let symbolic_hotkeys_plist = export_plist("com.apple.symbolichotkeys");

    Ok(ShellConfig {
        zshrc, starship_toml, aliases, shell_aliases_sh, ghostty_config, claude_config_dir,
        warp_themes, warp_keybindings, zed_settings, zed_keymap,
        vscode_settings, vscode_keybindings, vscode_extensions,
        fonts_tar_gz, screenshot_plist, symbolic_hotkeys_plist,
    })
}

fn read_dir_binary(dir: &Path) -> Option<Vec<(String, Vec<u8>)>> {
    if !dir.is_dir() { return None; }
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
    if files.is_empty() { None } else { Some(files) }
}

fn read_claude_config(dir: &Path) -> Option<Vec<(String, String)>> {
    if !dir.is_dir() { return None; }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        if p.is_file() {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            // Skip credential/secret files
            if name.contains("credential") || name.contains("secret") || name.contains(".db") {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&p) {
                files.push((name, content));
            }
        }
    }
    if files.is_empty() { None } else { Some(files) }
}

fn create_tar_gz(dir: &Path) -> Option<Vec<u8>> {
    if !dir.is_dir() { return None; }
    std::process::Command::new("tar")
        .args(["czf", "-", "-C", &dir.to_string_lossy(), "."])
        .output()
        .ok()
        .filter(|o| o.status.success() && !o.stdout.is_empty())
        .map(|o| o.stdout)
}

fn export_plist(domain: &str) -> Option<Vec<u8>> {
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

/// Writes shell config files to `home`.
pub fn import_shell_config(config: &ShellConfig, home: &Path) -> Result<()> {
    if let Some(ref content) = config.zshrc {
        std::fs::write(home.join(".zshrc"), content)?;
    }

    if let Some(ref content) = config.starship_toml {
        let dir = home.join(".config");
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("starship.toml"), content)?;
    }

    if let Some(ref content) = config.ghostty_config {
        let dir = home.join(".config/ghostty");
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("config"), content)?;
    }

    if let Some(ref files) = config.claude_config_dir {
        let dir = home.join(".claude");
        std::fs::create_dir_all(&dir)?;
        for (name, content) in files {
            std::fs::write(dir.join(name), content)?;
        }
    }

    Ok(())
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_aliases() {
        let zshrc = "alias gs='git status'\nalias ll='ls -la'\nexport PATH=$PATH:/usr/local/bin\n";
        let aliases = extract_aliases(zshrc);
        assert_eq!(aliases.len(), 2);
        assert!(aliases[0].contains("gs="));
        assert!(aliases[1].contains("ll="));
    }

    #[test]
    fn test_export_import_roundtrip() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();

        // Create source files
        std::fs::write(src.path().join(".zshrc"), "alias gs='git status'\n").unwrap();
        std::fs::create_dir_all(src.path().join(".config")).unwrap();
        std::fs::write(
            src.path().join(".config/starship.toml"),
            "[character]\nsymbol = \"➜\"\n",
        )
        .unwrap();

        let config = export_shell_config_from(src.path()).unwrap();
        assert!(config.zshrc.is_some());
        assert!(config.starship_toml.is_some());
        assert_eq!(config.aliases.len(), 1);

        import_shell_config(&config, dst.path()).unwrap();

        let written_zshrc = std::fs::read_to_string(dst.path().join(".zshrc")).unwrap();
        assert_eq!(written_zshrc, "alias gs='git status'\n");

        let written_starship =
            std::fs::read_to_string(dst.path().join(".config/starship.toml")).unwrap();
        assert!(written_starship.contains("character"));
    }
}
