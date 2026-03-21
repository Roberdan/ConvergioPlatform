use crate::mesh::peers::PeersRegistry;

pub fn ssh_read_peers_conf(ssh_alias: &str) -> Result<String, String> {
    let out = std::process::Command::new("ssh")
        .args([ssh_alias, "cat ~/.claude/config/peers.conf"])
        .output()
        .map_err(|e| e.to_string())?;

    if !out.status.success() {
        return Err(format!(
            "ssh cat peers.conf failed ({}): {}",
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn ssh_write_peers_conf(ssh_alias: &str, content: &str) -> Result<(), String> {
    use std::io::Write;

    let mut child = std::process::Command::new("ssh")
        .args([ssh_alias, "cat > ~/.claude/config/peers.conf"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(content.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!(
            "ssh write peers.conf failed ({})",
            status.code().unwrap_or(-1),
        ));
    }
    Ok(())
}

pub fn scp_db(from_alias: &str, to_alias: &str) -> Result<(), String> {
    let src = format!("{from_alias}:~/.claude/convergio.db");
    let dst = format!("{to_alias}:~/.claude/convergio.db");
    let out = std::process::Command::new("scp")
        .args(["-3", &src, &dst])
        .output()
        .map_err(|e| e.to_string())?;

    if !out.status.success() {
        return Err(format!(
            "scp db failed ({}): {}",
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr),
        ));
    }
    Ok(())
}

pub fn copy_crontab(from_alias: &str, to_alias: &str) -> Result<(), String> {
    // Dump crontab from source
    let out = std::process::Command::new("ssh")
        .args([from_alias, "crontab -l"])
        .output()
        .map_err(|e| e.to_string())?;

    if !out.status.success() {
        // No crontab is exit 1 on most systems — treat as empty
        if out.status.code() == Some(1) {
            return Ok(());
        }
        return Err(format!(
            "crontab -l failed ({}): {}",
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr),
        ));
    }

    let cron_content = out.stdout;
    if cron_content.is_empty() {
        return Ok(());
    }

    // Install on target
    use std::io::Write;
    let mut child = std::process::Command::new("ssh")
        .args([to_alias, "crontab -"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(&cron_content).map_err(|e| e.to_string())?;
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!(
            "crontab install failed ({})",
            status.code().unwrap_or(-1),
        ));
    }
    Ok(())
}

/// Serialise registry to INI string (delegates to PeersRegistry::save via a temp file).
pub fn registry_to_ini_string(registry: &PeersRegistry) -> String {
    let tmp = tempfile_path();
    if registry.save(&tmp).is_err() {
        return String::new();
    }
    std::fs::read_to_string(&tmp).unwrap_or_default()
}

pub fn tempfile_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "convergio-peers-{}.conf",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}
