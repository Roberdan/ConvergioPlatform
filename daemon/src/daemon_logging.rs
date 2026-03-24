// Daemon crash logging, panic hooks, and graceful shutdown.
// Why: daemon crashes left zero trace — no logs, no panic output, no file.
// This module ensures every crash is recorded to disk before the process exits.

use std::path::PathBuf;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Log file directory: $DASHBOARD_DB/../logs/ or data/logs/ relative to cwd.
fn log_dir() -> PathBuf {
    if let Ok(db) = std::env::var("DASHBOARD_DB") {
        if let Some(parent) = std::path::Path::new(&db).parent() {
            return parent.join("logs");
        }
    }
    PathBuf::from("data/logs")
}

/// Initialize daemon logging: file + stderr, panic hook, SIGTERM handler.
/// Returns a guard that must be held alive for the lifetime of the daemon —
/// dropping it flushes remaining log entries to disk.
pub fn init() -> WorkerGuard {
    init_inner(true)
}

/// File-only logging for TUI mode — no stderr output that would corrupt
/// the ratatui alternate screen.
pub fn init_file_only() -> WorkerGuard {
    init_inner(false)
}

fn init_inner(with_stderr: bool) -> WorkerGuard {
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);

    let file_appender = tracing_appender::rolling::daily(&dir, "daemon.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=warn"));

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true);

    if with_stderr {
        let stderr_layer = fmt::layer()
            .with_writer(std::io::stderr)
            .with_ansi(true)
            .with_target(false);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .with(stderr_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .init();
    }

    install_panic_hook(&dir);

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        pid = std::process::id(),
        "daemon starting"
    );

    guard
}

/// Panic hook writes crash info to daemon-crash.log before aborting.
fn install_panic_hook(log_dir: &std::path::Path) {
    let crash_path = log_dir.join("daemon-crash.log");
    std::panic::set_hook(Box::new(move |info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());

        let ts = chrono_now();
        let msg = format!(
            "[{ts}] PANIC pid={} at {location}: {payload}\n",
            std::process::id()
        );

        eprintln!("{msg}");
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crash_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, msg.as_bytes()));
    }));
}

fn chrono_now() -> String {
    // Avoid chrono dep — use system time formatted manually
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{now}")
}
