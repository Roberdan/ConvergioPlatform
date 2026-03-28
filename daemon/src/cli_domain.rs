// Copyright (c) 2026 Roberto D'Angelo. All rights reserved.
// Domain-skill mapping CLI subcommands: `cvg domain list` and `cvg domain map`.
// Delegates to daemon HTTP API for list; performs direct DB insert for map via API.

use crate::message_error::MessageResult;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum DomainCommands {
    /// List all domain→skill mappings
    List {
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
        /// Human-readable table output instead of JSON
        #[arg(long)]
        human: bool,
    },
    /// Add a domain→skill mapping
    Map {
        /// Domain name (e.g. healthcare)
        domain: String,
        /// Skill name — must match an existing claude-config/skills/<skill>/ directory
        skill: String,
        /// Optional description
        #[arg(long)]
        description: Option<String>,
        /// Daemon API base URL
        #[arg(long, default_value = "http://localhost:8420")]
        api_url: String,
        /// Human-readable output instead of JSON
        #[arg(long)]
        human: bool,
    },
}

pub async fn dispatch(cmd: DomainCommands) -> Result<(), crate::cli_error::CliError> {
    match cmd {
        DomainCommands::List { api_url, human } => handle_list(&api_url, human).await,
        DomainCommands::Map {
            domain,
            skill,
            description,
            api_url,
            human,
        } => handle_map(&domain, &skill, description.as_deref(), &api_url, human).await?,
    }
    Ok(())
}

async fn handle_list(api_url: &str, human: bool) {
    let _ = crate::cli_http::fetch_and_print(&format!("{api_url}/api/domain/list"), human).await;
}

async fn handle_map(
    domain: &str,
    skill: &str,
    description: Option<&str>,
    api_url: &str,
    human: bool,
) -> Result<(), crate::cli_error::CliError> {
    // Validate skill directory exists before calling daemon (fast-fail at CLI layer)
    let skill_path = format!("claude-config/skills/{skill}");
    validate_skill_dir(&skill_path)
        .map_err(|e| crate::cli_error::CliError::InvalidInput(format!("error: {e}")))?;
    let body = serde_json::json!({
        "domain": domain,
        "skill_name": skill,
        "description": description,
    });
    let _ = crate::cli_http::post_and_print(&format!("{api_url}/api/domain/map"), &body, human).await;
    Ok(())
}

/// Validate that the skill directory exists at `path`.
/// Returns Ok(()) if the directory exists, Err(message) otherwise.
pub(crate) fn validate_skill_dir(path: &str) -> MessageResult<()> {
    if std::path::Path::new(path).is_dir() {
        Ok(())
    } else {
        Err(format!("skill directory does not exist: {path}").into())
    }
}

/// A single row from domain_skill_map.
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct DomainSkillRow {
    pub(crate) domain: String,
    pub(crate) skill_name: String,
    pub(crate) description: Option<String>,
}

/// Query all rows from domain_skill_map ordered by domain, skill_name.
#[allow(dead_code)]
pub(crate) fn query_domain_list(
    conn: &rusqlite::Connection,
) -> rusqlite::Result<Vec<DomainSkillRow>> {
    let mut stmt = conn.prepare(
        "SELECT domain, skill_name, description \
         FROM domain_skill_map \
         ORDER BY domain, skill_name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(DomainSkillRow {
            domain: row.get(0)?,
            skill_name: row.get(1)?,
            description: row.get(2)?,
        })
    })?;
    rows.collect()
}

/// Insert a new domain→skill mapping. Returns Err on UNIQUE violation or DB error.
#[allow(dead_code)]
pub(crate) fn insert_domain_map(
    conn: &rusqlite::Connection,
    domain: &str,
    skill_name: &str,
    description: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO domain_skill_map (domain, skill_name, description) VALUES (?1, ?2, ?3)",
        rusqlite::params![domain, skill_name, description],
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "cli_domain_tests.rs"]
mod tests;
