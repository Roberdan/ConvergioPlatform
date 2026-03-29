// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
//! GET /api/node/readiness — node-level readiness checks for all swarm nodes.

use super::state::{ApiError, ServerState};
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use sysinfo::Disks;

pub fn router() -> Router<ServerState> {
    Router::new().route("/api/node/readiness", get(handle_node_readiness))
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

impl Check {
    fn pass(name: &str, detail: impl Into<String>) -> Self {
        Self { name: name.into(), passed: true, detail: detail.into() }
    }
    fn fail(name: &str, detail: impl Into<String>) -> Self {
        Self { name: name.into(), passed: false, detail: detail.into() }
    }
}

#[derive(Debug, Serialize)]
pub struct NodeReadinessResponse {
    pub ok: bool,
    pub node: String,
    pub role: String,
    pub checks: Vec<Check>,
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())
}

fn gethostname() -> String {
    std::process::Command::new("hostname").arg("-s").output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string()).unwrap_or_default()
}

/// Parse peers.conf → (role, capabilities) for this node.
/// Matches by: section name, dns_name, ssh_alias, or hostname substring.
fn parse_peers_conf() -> (String, Vec<String>) {
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
            // Save previous section if it matched
            if current_match && !current_role.is_empty() {
                role = current_role.clone();
                caps = current_caps.clone();
            }
            let section = &t[1..t.len()-1];
            // Match section name directly
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
            // Also match dns_name or ssh_alias fields
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
    // Check last section
    if current_match && !current_role.is_empty() {
        role = current_role;
        caps = current_caps;
    }
    (role, caps)
}

fn check_mlx_lm() -> Check {
    let ok = crate::ipc::models::apple_fm::AppleFmBridge::new().is_available();
    if ok { Check::pass("mlx_lm", format!("available via {}/convergio-env", home())) }
    else { Check::fail("mlx_lm", "mlx_lm not found or not on Apple Silicon") }
}

fn check_python_venv() -> Check {
    let p = format!("{}/convergio-env/bin/python", home());
    if std::path::Path::new(&p).exists() { Check::pass("python_venv", format!("found at {p}")) }
    else { Check::fail("python_venv", format!("not found: {p}")) }
}

fn check_db_path(db: &std::path::Path) -> Check {
    if !db.exists() { return Check::fail("db_exists", format!("not found: {}", db.display())); }
    let result = rusqlite::Connection::open(db).ok()
        .and_then(|c| c.query_row("PRAGMA integrity_check", [], |r| r.get::<_,String>(0)).ok())
        .unwrap_or_else(|| "error".into());
    if result == "ok" { Check::pass("db_exists", format!("integrity_check=ok ({})", db.display())) }
    else { Check::fail("db_exists", format!("PRAGMA integrity_check failed: {result}")) }
}

fn check_db_symlink(db: &std::path::Path) -> Check {
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

fn check_telegram_token() -> Check {
    if std::env::var("CONVERGIO_TELEGRAM_TOKEN").map(|v| !v.is_empty()).unwrap_or(false) {
        Check::pass("telegram_token", "token configured")
    } else { Check::fail("telegram_token", "CONVERGIO_TELEGRAM_TOKEN not set") }
}

fn check_disk_space(path: &str) -> Check {
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

fn check_models_downloaded() -> Check {
    let hub = format!("{}/.cache/huggingface/hub", home());
    let found = std::fs::read_dir(&hub).ok().map(|entries| {
        entries.flatten()
            .filter(|f| {
                let name = f.file_name().to_string_lossy().to_lowercase();
                name.contains("mistral") || name.contains("whisper") || name.contains("voxtral") || name.contains("qwen")
            })
            .map(|f| f.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>()
    }).unwrap_or_default();
    if found.is_empty() {
        Check::fail("models_downloaded", format!("no MLX models in {hub}"))
    } else {
        Check::pass("models_downloaded", format!("{} model(s): {}", found.len(), found.join(", ")))
    }
}

fn check_daemon_version() -> Check {
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

fn check_node_role() -> Check {
    let (role, _) = parse_peers_conf();
    if role.is_empty() { Check::fail("node_role", "hostname not found in peers.conf or role not set") }
    else { Check::pass("node_role", format!("role={role}")) }
}

fn check_role_capabilities() -> Check {
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
    if std::fs::read_dir(&cache).ok()
        .and_then(|e| e.flatten().find(|f| f.file_name().to_string_lossy().to_lowercase().contains("mistral")))
        .is_none() { missing.push("mistral model"); }
    if !std::env::var("CONVERGIO_TELEGRAM_TOKEN").map(|v| !v.is_empty()).unwrap_or(false) {
        missing.push("CONVERGIO_TELEGRAM_TOKEN");
    }
    if missing.is_empty() {
        Check::pass("role_capabilities", format!("role={role} — all required capabilities present"))
    } else {
        Check::fail("role_capabilities", format!("role={role} missing: {}", missing.join(", ")))
    }
}

fn default_db_path() -> std::path::PathBuf {
    std::env::var("DASHBOARD_DB").map(std::path::PathBuf::from).unwrap_or_else(|_| {
        std::path::PathBuf::from(format!("{}/.claude/data/dashboard.db", home()))
    })
}

/// Run all checks — exposed for unit tests (no live ServerState required).
pub fn run_checks() -> Vec<Check> {
    let db = default_db_path();
    build_checks(&db)
}

fn build_checks(db: &std::path::Path) -> Vec<Check> {
    vec![
        check_mlx_lm(),
        check_python_venv(),
        check_db_path(db),
        check_db_symlink(db),
        check_telegram_token(),
        check_disk_space(&home()),
        check_models_downloaded(),
        check_daemon_version(),
        check_node_role(),
        check_role_capabilities(),
    ]
}

/// GET /api/node/readiness
    #[tracing::instrument(skip_all)]
async fn handle_node_readiness(
    State(state): State<ServerState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let node = gethostname();
    let (role, _) = parse_peers_conf();
    let checks = build_checks(&state.db_path);
    let ok = checks.iter().all(|c| c.passed);
    Ok(Json(serde_json::json!({ "ok": ok, "node": node, "role": role, "checks": checks })))
}

#[cfg(test)]
#[path = "api_node_readiness_tests.rs"]
mod tests;
