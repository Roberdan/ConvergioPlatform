// Shell configuration (zsh, starship, ghostty, claude) migration

mod helpers;

use std::path::Path;
use thiserror::Error;

use helpers::{
    create_tar_gz, export_plist, extract_aliases, home_dir, read_claude_config, read_dir_binary,
    read_file_opt,
};

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
    pub warp_themes: Option<Vec<(String, Vec<u8>)>>, // (filename, bytes) — yaml + png
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
    let vscode_settings =
        read_file_opt(&home.join("Library/Application Support/Code/User/settings.json"));
    let vscode_keybindings =
        read_file_opt(&home.join("Library/Application Support/Code/User/keybindings.json"));

    // VSCode extensions list
    let vscode_extensions = std::process::Command::new("code")
        .args(["--list-extensions"])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.to_string())
                .collect()
        })
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
        zshrc,
        starship_toml,
        aliases,
        shell_aliases_sh,
        ghostty_config,
        claude_config_dir,
        warp_themes,
        warp_keybindings,
        zed_settings,
        zed_keymap,
        vscode_settings,
        vscode_keybindings,
        vscode_extensions,
        fonts_tar_gz,
        screenshot_plist,
        symbolic_hotkeys_plist,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_aliases() {
        let zshrc =
            "alias gs='git status'\nalias ll='ls -la'\nexport PATH=$PATH:/usr/local/bin\n";
        let aliases = helpers::extract_aliases(zshrc);
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
