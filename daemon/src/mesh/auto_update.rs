// Mesh auto-update background task.
// Polls /api/mesh/update-status every 5 minutes and applies updates when safe.

use serde::Deserialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

pub(crate) static LAST_UPDATE: AtomicU64 = AtomicU64::new(0);

const INTERVAL_SECS: u64 = 300; // 5 minutes
const RATE_LIMIT_SECS: u64 = 1800; // 30 minutes

#[derive(Deserialize)]
struct UpdateStatus {
    update_available: bool,
    #[allow(dead_code)]
    latest_version: String,
    #[allow(dead_code)]
    current_version: String,
    #[allow(dead_code)]
    peer_with_latest: String,
    rustc_mismatch: bool,
}

/// Returns true during quiet hours (23:00–06:59 Europe/Rome).
/// Assumes the daemon process runs with Europe/Rome system timezone.
pub(crate) fn is_quiet_hours() -> bool {
    let hour = chrono::Local::now().hour();
    hour >= 23 || hour < 7
}

use chrono::Timelike;

/// Returns true if the last successful update was less than 30 minutes ago.
pub(crate) fn is_rate_limited() -> bool {
    let last = LAST_UPDATE.load(Ordering::Relaxed);
    if last == 0 {
        return false;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now.saturating_sub(last) < RATE_LIMIT_SECS
}

fn now_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Run a shell command and return stdout or an error message.
pub(crate) fn run_cmd(cmd: &str, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("spawn {cmd}: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("{cmd} failed ({}): {stderr}", output.status))
    }
}

/// Background loop: check for updates every 5 minutes and apply when safe.
pub async fn run_auto_update_loop(repo_root: String) {
    let mut ticker = tokio::time::interval(Duration::from_secs(INTERVAL_SECS));
    let client = reqwest::Client::new();

    loop {
        // UNBOUNDED: event loop
        ticker.tick().await;
        if let Err(e) = try_update(&client, &repo_root).await {
            tracing::error!("auto-update tick failed: {e}");
            notify_error(&client, &e).await;
        }
    }
}

async fn try_update(client: &reqwest::Client, repo_root: &str) -> Result<(), String> {
    // 1. Fetch update status
    let resp = client
        .get("http://localhost:8420/api/mesh/update-status")
        .send()
        .await
        .map_err(|e| format!("fetch update-status: {e}"))?;
    let status: UpdateStatus = resp
        .json()
        .await
        .map_err(|e| format!("parse update-status: {e}"))?;

    // 2. No update available — skip
    if !status.update_available {
        tracing::debug!("auto-update: no update available");
        return Ok(());
    }

    // 3. Quiet hours — skip
    if is_quiet_hours() {
        tracing::info!("auto-update: skipping during quiet hours (23:00-07:00)");
        return Ok(());
    }

    // 4. Rate limit — skip
    if is_rate_limited() {
        tracing::info!("auto-update: rate-limited, last update < 30 min ago");
        return Ok(());
    }

    tracing::info!("auto-update: applying update");

    // 5a. git pull
    run_cmd("git", &["-C", repo_root, "pull", "origin", "main"])?;

    // 5b. Rustc mismatch → update toolchain
    if status.rustc_mismatch {
        tracing::info!("auto-update: rustc mismatch, updating stable toolchain");
        run_cmd("rustup", &["update", "stable"])?;
    }

    // 5c. Build or fetch binary based on node role
    let role = std::env::var("CONVERGIO_ROLE").unwrap_or_else(|_| "coordinator".into());
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let bin_path = format!("{home}/.convergio/bin/convergio-platform-daemon");

    if role == "coordinator" {
        run_cmd(
            "cargo",
            &[
                "build",
                "--release",
                "--features",
                "kernel",
                "--manifest-path",
                &format!("{repo_root}/daemon/Cargo.toml"),
            ],
        )
        .map(|_| {
            std::env::set_var("CARGO_TARGET_DIR", "/tmp/convergio-update-build");
        })
        .map_err(|e| format!("cargo build: {e}"))?;
        let install_script = format!("{repo_root}/scripts/platform/daemon-install.sh");
        run_cmd("bash", &[&install_script])?;
    } else {
        let coordinator =
            std::env::var("CONVERGIO_COORDINATOR").unwrap_or_else(|_| "macM5Max".into());
        let src = format!(
            "{coordinator}:~/.convergio/bin/convergio-platform-daemon"
        );
        run_cmd("rsync", &["-az", &src, &bin_path])?;
    }

    // 5d. Backup current binary
    let bak = format!("{bin_path}.bak");
    if std::path::Path::new(&bin_path).exists() {
        std::fs::copy(&bin_path, &bak)
            .map_err(|e| format!("backup binary: {e}"))?;
    }

    // 5e. Write restart-requested marker
    let restart_marker = format!("{home}/.convergio/restart-requested");
    std::fs::write(&restart_marker, now_epoch().to_string())
        .map_err(|e| format!("write restart marker: {e}"))?;

    // 5f. Record success
    LAST_UPDATE.store(now_epoch(), Ordering::Relaxed);
    let _ = client
        .post("http://localhost:8420/api/tracking/agent-activity")
        .json(&serde_json::json!({
            "agent_id": "mesh-auto-update",
            "status": "completed"
        }))
        .send()
        .await;

    tracing::info!("auto-update: completed successfully");
    Ok(())
}

async fn notify_error(client: &reqwest::Client, message: &str) {
    let _ = client
        .post("http://localhost:8420/api/notify")
        .json(&serde_json::json!({
            "message": format!("auto-update error: {message}")
        }))
        .send()
        .await;
}
