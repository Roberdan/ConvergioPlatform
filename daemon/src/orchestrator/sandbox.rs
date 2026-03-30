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

#[cfg(test)]
#[path = "sandbox_tests.rs"]
mod tests;
