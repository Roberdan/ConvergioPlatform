use std::path::PathBuf;

use super::types::*;
use super::utils::{default_db_path, default_peers_conf};

// ── IpcCommands enum construction ──────────────────────────────────

#[test]
fn ipc_commands_auth_variant() {
    let cmd = IpcCommands::Auth {
        command: AuthCommands::List { db_path: None },
    };
    assert!(matches!(cmd, IpcCommands::Auth { .. }));
}

#[test]
fn ipc_commands_models_variant() {
    let cmd = IpcCommands::Models { db_path: None };
    assert!(matches!(cmd, IpcCommands::Models { db_path: None }));
}

#[test]
fn ipc_commands_models_with_path() {
    let p = PathBuf::from("/tmp/test.db");
    let cmd = IpcCommands::Models {
        db_path: Some(p.clone()),
    };
    match cmd {
        IpcCommands::Models { db_path } => assert_eq!(db_path, Some(p)),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn ipc_commands_sub_variant() {
    let cmd = IpcCommands::Sub {
        command: SubCommands::List { db_path: None },
    };
    assert!(matches!(cmd, IpcCommands::Sub { .. }));
}

#[test]
fn ipc_commands_budget_variant() {
    let cmd = IpcCommands::Budget { db_path: None };
    assert!(matches!(cmd, IpcCommands::Budget { .. }));
}

#[test]
fn ipc_commands_route_variant() {
    let cmd = IpcCommands::Route {
        task_description: "test task".to_string(),
        dry_run: true,
        parallel: false,
        db_path: None,
    };
    match cmd {
        IpcCommands::Route {
            task_description,
            dry_run,
            parallel,
            ..
        } => {
            assert_eq!(task_description, "test task");
            assert!(dry_run);
            assert!(!parallel);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn ipc_commands_skills_variant() {
    let cmd = IpcCommands::Skills {
        agent: Some("convergio".to_string()),
        db_path: None,
    };
    match cmd {
        IpcCommands::Skills { agent, .. } => {
            assert_eq!(agent.unwrap(), "convergio");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn ipc_commands_request_skill_variant() {
    let cmd = IpcCommands::RequestSkill {
        skill: "code-review".to_string(),
        payload: r#"{"file":"main.rs"}"#.to_string(),
        db_path: None,
    };
    match cmd {
        IpcCommands::RequestSkill {
            skill, payload, ..
        } => {
            assert_eq!(skill, "code-review");
            assert!(payload.contains("main.rs"));
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn ipc_commands_respond_skill_variant() {
    let cmd = IpcCommands::RespondSkill {
        request_id: "req-001".to_string(),
        result: "done".to_string(),
        db_path: None,
    };
    match cmd {
        IpcCommands::RespondSkill {
            request_id, result, ..
        } => {
            assert_eq!(request_id, "req-001");
            assert_eq!(result, "done");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn ipc_commands_rate_skill_variant() {
    let cmd = IpcCommands::RateSkill {
        request_id: "req-002".to_string(),
        rating: 4.5,
        db_path: None,
    };
    match cmd {
        IpcCommands::RateSkill {
            request_id, rating, ..
        } => {
            assert_eq!(request_id, "req-002");
            assert!((rating - 4.5).abs() < f64::EPSILON);
        }
        _ => panic!("wrong variant"),
    }
}

// ── AuthCommands enum construction ─────────────────────────────────

#[test]
fn auth_commands_store() {
    let cmd = AuthCommands::Store {
        service: "github".to_string(),
        token: "ghp_xxx".to_string(),
        secret: "s3cret".to_string(),
        db_path: None,
    };
    assert!(matches!(cmd, AuthCommands::Store { .. }));
}

#[test]
fn auth_commands_get() {
    let cmd = AuthCommands::Get {
        service: "github".to_string(),
        secret: "s3cret".to_string(),
        db_path: Some(PathBuf::from("/tmp/t.db")),
    };
    match cmd {
        AuthCommands::Get { service, .. } => assert_eq!(service, "github"),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn auth_commands_revoke() {
    let cmd = AuthCommands::Revoke {
        service: "slack".to_string(),
        host: Some("node-1".to_string()),
        db_path: None,
    };
    match cmd {
        AuthCommands::Revoke { service, host, .. } => {
            assert_eq!(service, "slack");
            assert_eq!(host.unwrap(), "node-1");
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn auth_commands_revoke_no_host() {
    let cmd = AuthCommands::Revoke {
        service: "slack".to_string(),
        host: None,
        db_path: None,
    };
    assert!(matches!(cmd, AuthCommands::Revoke { host: None, .. }));
}

#[test]
fn auth_commands_rotate() {
    let cmd = AuthCommands::Rotate {
        old_secret: "old".to_string(),
        new_secret: "new".to_string(),
        db_path: None,
    };
    assert!(matches!(cmd, AuthCommands::Rotate { .. }));
}

// ── SubCommands enum construction ──────────────────────────────────

#[test]
fn sub_commands_add() {
    let cmd = SubCommands::Add {
        name: "anthropic-pro".to_string(),
        provider: "anthropic".to_string(),
        plan: "pro".to_string(),
        budget: 100.0,
        reset_day: 15,
        models: vec!["claude-3".to_string(), "haiku".to_string()],
        db_path: None,
    };
    match cmd {
        SubCommands::Add {
            name, budget, reset_day, models, ..
        } => {
            assert_eq!(name, "anthropic-pro");
            assert!((budget - 100.0).abs() < f64::EPSILON);
            assert_eq!(reset_day, 15);
            assert_eq!(models.len(), 2);
        }
        _ => panic!("wrong variant"),
    }
}

#[test]
fn sub_commands_list() {
    let cmd = SubCommands::List { db_path: None };
    assert!(matches!(cmd, SubCommands::List { .. }));
}

#[test]
fn sub_commands_remove() {
    let cmd = SubCommands::Remove {
        name: "old-sub".to_string(),
        db_path: None,
    };
    match cmd {
        SubCommands::Remove { name, .. } => assert_eq!(name, "old-sub"),
        _ => panic!("wrong variant"),
    }
}
