// macOS defaults and system preferences migration

use std::path::Path;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MacosError {
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, MacosError>;

fn defaults_write(domain: &str, key: &str, kind: &str, value: &str) -> Result<()> {
    let status = Command::new("defaults")
        .args(["write", domain, key, kind, value])
        .status()?;

    if !status.success() {
        return Err(MacosError::CommandFailed(format!(
            "defaults write {} {} {} {} failed",
            domain, key, kind, value
        )));
    }
    Ok(())
}

/// Excludes a path from Spotlight indexing.
pub fn spotlight_exclude(path: &Path) -> Result<()> {
    let path_str = path.to_string_lossy();

    // Disable metadata indexing on the volume/path
    let status = Command::new("mdutil")
        .args(["-i", "off", path_str.as_ref()])
        .status()?;

    if !status.success() {
        return Err(MacosError::CommandFailed(format!(
            "mdutil -i off {} failed",
            path_str
        )));
    }

    // Add to privacy exclusion list via defaults
    defaults_write(
        "com.apple.spotlight",
        "orderedItems",
        "-array-add",
        &format!(
            "<dict><key>enabled</key><false/><key>name</key><string>PATH:{}</string></dict>",
            path_str
        ),
    )
}

/// Removes all non-persistent items from the Dock.
pub fn configure_dock() -> Result<()> {
    // Remove all static dock items (apps must be re-added manually)
    defaults_write("com.apple.dock", "static-only", "-bool", "false")?;
    defaults_write("com.apple.dock", "show-recents", "-bool", "false")?;
    defaults_write("com.apple.dock", "tilesize", "-int", "48")?;
    defaults_write("com.apple.dock", "autohide", "-bool", "true")?;

    // Restart dock to apply
    let _ = Command::new("killall").args(["Dock"]).status();
    Ok(())
}

/// Configures Finder: show extensions, path bar, status bar.
pub fn configure_finder() -> Result<()> {
    defaults_write("com.apple.finder", "AppleShowAllExtensions", "-bool", "true")?;
    defaults_write("com.apple.finder", "ShowPathbar", "-bool", "true")?;
    defaults_write("com.apple.finder", "ShowStatusBar", "-bool", "true")?;
    defaults_write("com.apple.finder", "FXPreferredViewStyle", "-string", "Nlsv")?;
    defaults_write(
        "com.apple.finder",
        "FXDefaultSearchScope",
        "-string",
        "SCcf",
    )?;

    let _ = Command::new("killall").args(["Finder"]).status();
    Ok(())
}

/// Sets fast key repeat with short initial delay.
pub fn configure_keyboard() -> Result<()> {
    // Key repeat rate: lower = faster (2 is very fast, 6 is default)
    defaults_write("NSGlobalDomain", "KeyRepeat", "-int", "2")?;
    // Initial key repeat delay: lower = shorter (15 = ~225ms, 25 is default)
    defaults_write("NSGlobalDomain", "InitialKeyRepeat", "-int", "15")?;
    Ok(())
}

/// Prevents .DS_Store creation on network shares.
pub fn disable_ds_store_network() -> Result<()> {
    defaults_write(
        "com.apple.desktopservices",
        "DSDontWriteNetworkStores",
        "-bool",
        "true",
    )
}

/// Apply all macOS defaults in one call.
pub fn apply_all() -> Result<()> {
    configure_dock()?;
    configure_finder()?;
    configure_keyboard()?;
    disable_ds_store_network()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    // macOS defaults commands require the actual system; we only test that
    // the error type is correct and the Result wrapping works.
    use super::*;

    #[test]
    fn test_error_display() {
        let e = MacosError::CommandFailed("test".to_string());
        assert!(e.to_string().contains("test"));
    }
}
