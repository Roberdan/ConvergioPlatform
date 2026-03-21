// SSH key generation and screen-sharing detection utilities.

use std::path::Path;

use super::NetworkError;

/// Generate an ed25519 keypair at `path` (private) and `path.pub` (public).
/// Returns Err if the key already exists to avoid silent overwrites.
pub fn generate_ssh_keypair(path: &Path) -> Result<(), NetworkError> {
    if path.exists() {
        return Err(NetworkError::Other(format!(
            "key already exists: {}",
            path.display()
        )));
    }

    let path_str = path
        .to_str()
        .ok_or_else(|| NetworkError::Other("key path is not valid UTF-8".to_owned()))?;

    let out = std::process::Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-f", path_str])
        .output()?;

    if !out.status.success() {
        return Err(NetworkError::CommandFailed {
            cmd: "ssh-keygen".to_owned(),
            code: out.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        });
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum ScreenSharingStatus {
    Enabled,
    Disabled,
    Unknown,
}

/// Detect remote-desktop/screen-sharing status without modifying system state.
/// macOS: checks ARD via `launchctl list`.
/// Linux: checks for xrdp or wayvnc via `pgrep`.
/// Windows: returns Unknown (detection not implemented).
pub fn enable_screen_sharing(os: &str) -> Result<ScreenSharingStatus, NetworkError> {
    match os {
        "macos" => {
            let out = std::process::Command::new("launchctl")
                .args(["list", "com.apple.screensharing"])
                .output()?;
            if out.status.success() {
                Ok(ScreenSharingStatus::Enabled)
            } else {
                Ok(ScreenSharingStatus::Disabled)
            }
        }
        "linux" => {
            for svc in ["xrdp", "wayvnc"] {
                let out = std::process::Command::new("pgrep")
                    .arg("-x")
                    .arg(svc)
                    .output()?;
                if out.status.success() {
                    return Ok(ScreenSharingStatus::Enabled);
                }
            }
            Ok(ScreenSharingStatus::Disabled)
        }
        "windows" => Ok(ScreenSharingStatus::Unknown),
        other => Err(NetworkError::UnsupportedOs(other.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn ssh_keygen_creates_keypair() {
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("id_ed25519");
        generate_ssh_keypair(&key_path).unwrap();
        assert!(key_path.exists(), "private key file must exist");
        let pub_path = dir.path().join("id_ed25519.pub");
        assert!(pub_path.exists(), "public key file must exist");
    }

    #[test]
    fn ssh_keygen_refuses_overwrite() {
        let dir = TempDir::new().unwrap();
        let key_path = dir.path().join("id_ed25519");
        generate_ssh_keypair(&key_path).unwrap();
        let result = generate_ssh_keypair(&key_path);
        assert!(result.is_err(), "second call must fail — key exists");
    }
}
