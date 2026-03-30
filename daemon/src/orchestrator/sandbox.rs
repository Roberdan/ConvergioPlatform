// Agent sandboxing: per-agent execution profiles with filesystem/network/command allowlists.
// Violations are logged to audit_log; settings.json generation targets .claude/settings.json.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Per-agent execution profile defining allowed resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub name: String,
    pub filesystem_allowlist: Vec<PathBuf>,
    pub network_allowlist: Vec<String>,
    pub allowed_commands: Vec<String>,
}

pub const MIGRATION_AGENT_PROFILES: &str =
    "CREATE TABLE IF NOT EXISTS agent_profiles (\
     id INTEGER PRIMARY KEY, \
     name TEXT UNIQUE NOT NULL, \
     filesystem_allowlist TEXT, \
     network_allowlist TEXT, \
     allowed_commands TEXT, \
     created_at TEXT DEFAULT (datetime('now'))\
     )";

pub const MIGRATION_AUDIT_LOG: &str =
    "CREATE TABLE IF NOT EXISTS audit_log (\
     id INTEGER PRIMARY KEY, \
     timestamp TEXT NOT NULL DEFAULT (datetime('now')), \
     agent TEXT, \
     action TEXT NOT NULL, \
     resource TEXT, \
     detail TEXT, \
     ip_addr TEXT\
     )";

/// Run idempotent table migrations for sandbox tables.
pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(MIGRATION_AGENT_PROFILES)?;
    conn.execute_batch(MIGRATION_AUDIT_LOG)?;
    Ok(())
}

/// Check that `command` is permitted by the profile (exact match).
/// Returns `Err` with a human-readable violation description if blocked.
pub fn validate_command(profile: &AgentProfile, command: &str) -> Result<(), String> {
    let allowed = profile.allowed_commands.iter().any(|a| a == command);
    if allowed {
        return Ok(());
    }
    Err(format!(
        "agent '{}' is not permitted to run '{}'; allowed: [{}]",
        profile.name,
        command,
        profile.allowed_commands.join(", ")
    ))
}

/// Load an agent profile from the database.
pub fn get_profile(conn: &Connection, agent_name: &str) -> Option<AgentProfile> {
    let result = conn.query_row(
        "SELECT name, filesystem_allowlist, network_allowlist, allowed_commands \
         FROM agent_profiles WHERE name = ?1",
        params![agent_name],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        },
    );

    result.ok().map(|(name, fs, net, cmds)| AgentProfile {
        name,
        filesystem_allowlist: parse_path_list(fs.as_deref()),
        network_allowlist: parse_str_list(net.as_deref()),
        allowed_commands: parse_str_list(cmds.as_deref()),
    })
}

/// Insert a violation record into audit_log.
pub fn log_violation(conn: &Connection, agent: &str, command: &str, reason: &str) {
    if let Err(e) = conn.execute(
        "INSERT INTO audit_log (agent, action, resource, detail) VALUES (?1, 'BLOCKED', ?2, ?3)",
        params![agent, command, reason],
    ) {
        tracing::warn!("sandbox: failed to log violation for agent '{agent}': {e}");
    }
}

/// Generate a `.claude/settings.json` fragment that restricts commands to the profile allowlist.
pub fn generate_worktree_settings(profile: &AgentProfile) -> String {
    let allow_rules: Vec<serde_json::Value> = profile
        .allowed_commands
        .iter()
        .map(|cmd| {
            serde_json::json!({
                "type": "bash",
                "command": cmd
            })
        })
        .collect();

    let settings = serde_json::json!({
        "permissions": {
            "allow": allow_rules,
            "deny": []
        }
    });

    serde_json::to_string_pretty(&settings).unwrap_or_else(|_| "{}".to_string())
}

fn parse_str_list(s: Option<&str>) -> Vec<String> {
    match s {
        None | Some("") => vec![],
        Some(raw) => serde_json::from_str::<Vec<String>>(raw)
            .unwrap_or_else(|_| raw.split(',').map(|s| s.trim().to_string()).collect()),
    }
}

fn parse_path_list(s: Option<&str>) -> Vec<PathBuf> {
    parse_str_list(s).into_iter().map(PathBuf::from).collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_profile(cmds: &[&str]) -> AgentProfile {
        AgentProfile {
            name: "test-agent".to_string(),
            filesystem_allowlist: vec![],
            network_allowlist: vec![],
            allowed_commands: cmds.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn in_memory_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn test_validate_command_allowed() {
        let profile = test_profile(&["cargo test", "git status"]);
        assert!(validate_command(&profile, "cargo test").is_ok());
        assert!(validate_command(&profile, "git status").is_ok());
    }

    #[test]
    fn test_validate_command_blocked() {
        let profile = test_profile(&["cargo test"]);
        let err = validate_command(&profile, "rm -rf /").unwrap_err();
        assert!(err.contains("not permitted"));
        assert!(err.contains("rm -rf /"));
        assert!(err.contains("test-agent"));
    }

    #[test]
    fn test_validate_command_exact_match_required() {
        // Only exact strings are matched; "cargo" does NOT grant "cargo build".
        let profile = test_profile(&["cargo"]);
        assert!(validate_command(&profile, "cargo").is_ok());
        let err = validate_command(&profile, "cargo build").unwrap_err();
        assert!(err.contains("not permitted"));
        assert!(err.contains("cargo build"));
    }

    #[test]
    fn test_get_profile_missing() {
        let conn = in_memory_conn();
        assert!(get_profile(&conn, "no-such-agent").is_none());
    }

    #[test]
    fn test_get_profile_roundtrip() {
        let conn = in_memory_conn();
        conn.execute(
            "INSERT INTO agent_profiles (name, filesystem_allowlist, network_allowlist, allowed_commands) \
             VALUES (?1, ?2, ?3, ?4)",
            params![
                "builder",
                r#"["/workspace"]"#,
                r#"["github.com"]"#,
                r#"["cargo build","cargo test"]"#
            ],
        )
        .unwrap();

        let profile = get_profile(&conn, "builder").unwrap();
        assert_eq!(profile.name, "builder");
        assert_eq!(profile.filesystem_allowlist, vec![PathBuf::from("/workspace")]);
        assert_eq!(profile.network_allowlist, vec!["github.com"]);
        assert_eq!(
            profile.allowed_commands,
            vec!["cargo build".to_string(), "cargo test".to_string()]
        );
    }

    #[test]
    fn test_log_violation_inserts_row() {
        let conn = in_memory_conn();
        log_violation(&conn, "agent-x", "rm -rf /", "command not in allowlist");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_generate_worktree_settings() {
        let profile = test_profile(&["cargo build", "git commit"]);
        let settings = generate_worktree_settings(&profile);
        let v: serde_json::Value = serde_json::from_str(&settings).unwrap();
        let allow = v["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 2);
        assert_eq!(allow[0]["command"], "cargo build");
        assert_eq!(allow[1]["command"], "git commit");
    }

    #[test]
    fn test_migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        // Running twice must not error (CREATE TABLE IF NOT EXISTS)
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
    }
}
