use std::path::PathBuf;

use super::handle_ipc;
use super::types::*;
use super::utils::{default_db_path, default_peers_conf};

// ── DaemonCommands construction ────────────────────────────────────

#[test]
fn daemon_commands_start_defaults() {
    let cmd = DaemonCommands::Start {
        bind_ip: None,
        port: 9420,
        peers_conf: None,
        db_path: None,
        crsqlite_path: None,
        local_only: false,
    };
    match cmd {
        DaemonCommands::Start {
            port, local_only, ..
        } => {
            assert_eq!(port, 9420);
            assert!(!local_only);
        }
    }
}

#[test]
fn daemon_commands_start_local_only() {
    let cmd = DaemonCommands::Start {
        bind_ip: Some("127.0.0.1".to_string()),
        port: 8080,
        peers_conf: Some(PathBuf::from("/etc/peers.conf")),
        db_path: Some(PathBuf::from("/tmp/test.db")),
        crsqlite_path: Some("libcrsqlite.so".to_string()),
        local_only: true,
    };
    match cmd {
        DaemonCommands::Start {
            bind_ip,
            port,
            local_only,
            crsqlite_path,
            ..
        } => {
            assert_eq!(bind_ip.unwrap(), "127.0.0.1");
            assert_eq!(port, 8080);
            assert!(local_only);
            assert_eq!(crsqlite_path.unwrap(), "libcrsqlite.so");
        }
    }
}

// ── IpcHandlerError formatting ─────────────────────────────────────

#[test]
fn error_db_open_display() {
    let e = IpcHandlerError::DbOpen("no such file".to_string());
    assert_eq!(format!("{e}"), "database error: no such file");
}

#[test]
fn error_operation_failed_display() {
    let e = IpcHandlerError::OperationFailed("timeout".to_string());
    assert_eq!(format!("{e}"), "timeout");
}

#[test]
fn error_not_found_display() {
    let e = IpcHandlerError::NotFound("token xyz".to_string());
    assert_eq!(format!("{e}"), "not found: token xyz");
}

#[test]
fn error_server_failed_display() {
    let e = IpcHandlerError::ServerFailed("bind failed".to_string());
    assert_eq!(format!("{e}"), "server error: bind failed");
}

#[test]
fn error_debug_impl() {
    let e = IpcHandlerError::DbOpen("test".to_string());
    let dbg = format!("{e:?}");
    assert!(dbg.contains("DbOpen"));
}

#[test]
fn error_variants_are_distinct() {
    let a = format!("{}", IpcHandlerError::DbOpen("x".into()));
    let b = format!("{}", IpcHandlerError::NotFound("x".into()));
    let c = format!("{}", IpcHandlerError::ServerFailed("x".into()));
    assert_ne!(a, b);
    assert_ne!(b, c);
    assert_ne!(a, c);
}

// ── utils: default paths ───────────────────────────────────────────

#[test]
fn default_db_path_ends_with_dashboard_db() {
    // BUG-2: default_db_path now reads DASHBOARD_DB env first — clear for fallback test
    let saved = std::env::var("DASHBOARD_DB").ok();
    std::env::remove_var("DASHBOARD_DB");
    let p = default_db_path();
    if let Some(v) = saved { std::env::set_var("DASHBOARD_DB", v); }
    assert!(p.ends_with(".claude/data/dashboard.db"), "got: {p:?}");
}

#[test]
fn default_db_path_is_absolute_when_home_set() {
    if std::env::var("HOME").is_ok() {
        let p = default_db_path();
        assert!(p.is_absolute());
    }
}

#[test]
fn default_peers_conf_ends_with_peers_conf() {
    let p = default_peers_conf();
    assert!(p.ends_with(".claude/config/peers.conf"));
}

// ── handle_ipc: DB-error paths (non-existent DB) ──────────────────

#[tokio::test]
async fn handle_ipc_models_bad_db_returns_error() {
    let bad = PathBuf::from("/nonexistent/path/does_not_exist.db");
    let cmd = IpcCommands::Models {
        db_path: Some(bad),
    };
    let result = handle_ipc(cmd).await;
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("database error"));
}

#[tokio::test]
async fn handle_ipc_budget_bad_db_returns_error() {
    let bad = PathBuf::from("/nonexistent/bad.db");
    let cmd = IpcCommands::Budget {
        db_path: Some(bad),
    };
    let result = handle_ipc(cmd).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn handle_ipc_route_bad_db_returns_error() {
    let bad = PathBuf::from("/nonexistent/r.db");
    let cmd = IpcCommands::Route {
        task_description: "test".to_string(),
        dry_run: false,
        parallel: false,
        db_path: Some(bad),
    };
    let result = handle_ipc(cmd).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn handle_ipc_skills_bad_db_returns_error() {
    let bad = PathBuf::from("/nonexistent/s.db");
    let cmd = IpcCommands::Skills {
        agent: None,
        db_path: Some(bad),
    };
    let result = handle_ipc(cmd).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn handle_ipc_skills_with_agent_bad_db_returns_error() {
    let bad = PathBuf::from("/nonexistent/s2.db");
    let cmd = IpcCommands::Skills {
        agent: Some("test-agent".to_string()),
        db_path: Some(bad),
    };
    let result = handle_ipc(cmd).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn handle_ipc_auth_list_bad_db_returns_error() {
    let bad = PathBuf::from("/nonexistent/a.db");
    let cmd = IpcCommands::Auth {
        command: AuthCommands::List {
            db_path: Some(bad),
        },
    };
    let result = handle_ipc(cmd).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn handle_ipc_request_skill_bad_db_returns_error() {
    let bad = PathBuf::from("/nonexistent/rs.db");
    let cmd = IpcCommands::RequestSkill {
        skill: "test-skill".to_string(),
        payload: "{}".to_string(),
        db_path: Some(bad),
    };
    let result = handle_ipc(cmd).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn handle_ipc_respond_skill_bad_db_returns_error() {
    let bad = PathBuf::from("/nonexistent/resp.db");
    let cmd = IpcCommands::RespondSkill {
        request_id: "req-999".to_string(),
        result: "ok".to_string(),
        db_path: Some(bad),
    };
    let result = handle_ipc(cmd).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn handle_ipc_rate_skill_bad_db_returns_error() {
    let bad = PathBuf::from("/nonexistent/rate.db");
    let cmd = IpcCommands::RateSkill {
        request_id: "req-888".to_string(),
        rating: 5.0,
        db_path: Some(bad),
    };
    let result = handle_ipc(cmd).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn handle_ipc_sub_list_bad_db_returns_error() {
    let bad = PathBuf::from("/nonexistent/sub.db");
    let cmd = IpcCommands::Sub {
        command: SubCommands::List {
            db_path: Some(bad),
        },
    };
    let result = handle_ipc(cmd).await;
    assert!(result.is_err());
}
