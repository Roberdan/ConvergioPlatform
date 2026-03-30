// Node readiness check functions — extracted from api_node_readiness.rs.
// WHY: individual checks are independently testable; isolating them keeps
//      api_node_readiness.rs ≤250 lines.

use super::Check;

pub(super) fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
}

pub(super) fn gethostname() -> String {
    match std::process::Command::new("hostname").arg("-s").output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(e) => { tracing::warn!("hostname command failed: {e}"); String::new() }
    }
}

/// Parse peers.conf → (role, capabilities) for this node.
/// Matches by: section name, dns_name, ssh_alias, or hostname substring.
pub(super) fn parse_peers_conf() -> (String, Vec<String>) {
    let content = std::fs::read_to_string(
        format!("{}/.claude/config/peers.conf", home())
    ).unwrap_or_default();
    let hostname = gethostname().to_lowercase();
    let (mut role, mut caps) = (String::new(), Vec::new());
    let mut current_role = String::new();
    let mut current_caps: Vec<String> = Vec::new();
    let mut current_match = false;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            if current_match && !current_role.is_empty() {
                role = current_role.clone();
                caps = current_caps.clone();
            }
            let section = &t[1..t.len()-1];
            current_match = section.to_lowercase() == hostname
                || hostname.contains(&section.to_lowercase())
                || section.to_lowercase().contains(&hostname);
            current_role.clear();
            current_caps.clear();
        } else if !t.is_empty() && !t.starts_with('#') {
            if let Some(v) = t.strip_prefix("role=") { current_role = v.into(); }
            if let Some(v) = t.strip_prefix("capabilities=") {
                current_caps = v.split(',').map(|s| s.trim().to_string()).collect();
            }
            if let Some(v) = t.strip_prefix("dns_name=") {
                if v.to_lowercase().contains(&hostname) || hostname.contains(&v.to_lowercase().replace(".tail01f12c.ts.net","").replace("-","")) {
                    current_match = true;
                }
            }
            if let Some(v) = t.strip_prefix("ssh_alias=") {
                if v.to_lowercase().contains(&hostname) || hostname.contains(&v.to_lowercase()) {
                    current_match = true;
                }
            }
        }
    }
    if current_match && !current_role.is_empty() {
        role = current_role;
        caps = current_caps;
    }
    (role, caps)
}

pub(super) fn check_mlx_lm() -> Check {
    let ok = crate::ipc::models::apple_fm::AppleFmBridge::new().is_available();
    if ok { Check::pass("mlx_lm", format!("available via {}/convergio-env", home())) }
    else { Check::fail("mlx_lm", "mlx_lm not found or not on Apple Silicon") }
}

pub(super) fn check_python_venv() -> Check {
    let p = format!("{}/convergio-env/bin/python", home());
    if std::path::Path::new(&p).exists() { Check::pass("python_venv", format!("found at {p}")) }
    else { Check::fail("python_venv", format!("not found: {p}")) }
}

pub(super) fn check_db_path(db: &std::path::Path) -> Check {
    if !db.exists() { return Check::fail("db_exists", format!("not found: {}", db.display())); }
    let result = match rusqlite::Connection::open(db) {
        Ok(c) => match c.query_row("PRAGMA integrity_check", [], |r| r.get::<_,String>(0)) {
            Ok(v) => v,
            Err(e) => { tracing::warn!("db integrity check failed: {e}"); "error".into() }
        },
        Err(e) => { tracing::warn!("db open for integrity check failed: {e}"); "error".into() }
    };
    if result == "ok" { Check::pass("db_exists", format!("integrity_check=ok ({})", db.display())) }
    else { Check::fail("db_exists", format!("PRAGMA integrity_check failed: {result}")) }
}

pub(super) fn check_db_symlink(db: &std::path::Path) -> Check {
    match std::fs::read_link(db) {
        Err(_) if db.exists() => Check::pass("db_symlink", format!("direct file at {}", db.display())),
        Err(_) => Check::fail("db_symlink", "db path does not exist"),
        Ok(t) => {
            let ts = t.to_string_lossy();
            if ts.contains(&home()) || ts.starts_with('/') { Check::pass("db_symlink", format!("symlink → {ts}")) }
            else { Check::fail("db_symlink", format!("symlink target looks external: {ts}")) }
        }
    }
}

pub(super) fn check_telegram_token() -> Check {
    if crate::telegram_config::telegram_token().is_some() {
        Check::pass("telegram_token", "token configured")
    } else {
        Check::fail(
            "telegram_token",
            "Telegram token not set (need CONVERGIO_TELEGRAM_TOKEN or TELEGRAM_BOT_TOKEN)",
        )
    }
}

pub(super) fn check_disk_space(path: &str) -> Check {
    use sysinfo::Disks;
    const MIN: u64 = 5 * 1024 * 1024 * 1024;
    let disks = Disks::new_with_refreshed_list();
    let best = disks.iter()
        .filter(|d| path.starts_with(d.mount_point().to_string_lossy().as_ref()))
        .max_by_key(|d| d.mount_point().to_string_lossy().len());
    match best {
        None => Check::fail("disk_space", "could not determine disk for path"),
        Some(d) => {
            let gb = d.available_space() as f64 / 1_073_741_824.0;
            let detail = format!("{gb:.1} GB free on {}", d.mount_point().display());
            if d.available_space() >= MIN { Check::pass("disk_space", detail) }
            else { Check::fail("disk_space", detail) }
        }
    }
}

pub(super) fn check_models_downloaded() -> Check {
    let hub = format!("{}/.cache/huggingface/hub", home());
    let found = match std::fs::read_dir(&hub) {
        Ok(entries) => entries.flatten()
            .filter(|f| {
                let name = f.file_name().to_string_lossy().to_lowercase();
                name.contains("mistral") || name.contains("whisper") || name.contains("voxtral") || name.contains("qwen")
            })
            .map(|f| f.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>(),
        Err(e) => { tracing::debug!("models dir read failed {hub}: {e}"); Vec::new() }
    };
    if found.is_empty() {
        Check::fail("models_downloaded", format!("no MLX models in {hub}"))
    } else {
        Check::pass("models_downloaded", format!("{} model(s): {}", found.len(), found.join(", ")))
    }
}

pub(super) fn check_daemon_version() -> Check {
    let cargo_ver = env!("CARGO_PKG_VERSION");
    let file_ver = std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| format!("{d}/../VERSION.md"))
        .and_then(|p| std::fs::read_to_string(&p).map_err(|_| std::env::VarError::NotPresent))
        .map(|s| s.trim().to_string()).unwrap_or_default();
    if file_ver.is_empty() {
        Check::pass("daemon_version", format!("version {cargo_ver} (VERSION.md not found)"))
    } else if file_ver == cargo_ver {
        Check::pass("daemon_version", format!("version {cargo_ver} matches VERSION.md"))
    } else {
        Check::fail("daemon_version", format!("mismatch: Cargo={cargo_ver} VERSION.md={file_ver}"))
    }
}

pub(super) fn check_node_role() -> Check {
    let (role, _) = parse_peers_conf();
    if role.is_empty() { Check::fail("node_role", "hostname not found in peers.conf or role not set") }
    else { Check::pass("node_role", format!("role={role}")) }
}

pub(super) fn check_role_capabilities() -> Check {
    let (role, caps) = parse_peers_conf();
    if role.is_empty() { return Check::fail("role_capabilities", "role unknown"); }
    if role != "kernel" {
        return Check::pass("role_capabilities", format!("role={role} — no mandatory infra requirements"));
    }
    let mut missing: Vec<&str> = Vec::new();
    if !caps.iter().any(|c| c == "mlx_lm" || c == "ollama") {
        if !crate::ipc::models::apple_fm::AppleFmBridge::new().is_available() {
            missing.push("mlx_lm");
        }
    }
    let cache = format!("{}/.cache/huggingface", home());
    let has_mistral = match std::fs::read_dir(&cache) {
        Ok(e) => e.flatten().any(|f| f.file_name().to_string_lossy().to_lowercase().contains("mistral")),
        Err(_) => false,
    };
    if !has_mistral { missing.push("mistral model"); }
    if crate::telegram_config::telegram_token().is_none() {
        missing.push("CONVERGIO_TELEGRAM_TOKEN/TELEGRAM_BOT_TOKEN");
    }
    if missing.is_empty() {
        Check::pass("role_capabilities", format!("role={role} — all required capabilities present"))
    } else {
        Check::fail("role_capabilities", format!("role={role} missing: {}", missing.join(", ")))
    }
}
