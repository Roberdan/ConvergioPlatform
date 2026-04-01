//! CLI handlers for creating orgs from mission statements or existing repos.

use crate::cli_error::CliError;
use convergio_core::org::{factory, orgchart, provisioner, repo_scanner};
use std::path::Path;

/// Create an org from a mission statement, show orgchart, confirm, provision.
pub async fn handle_create_org(
    name: &str,
    mission: &str,
    budget: f64,
    yes: bool,
    api_url: &str,
) -> Result<(), CliError> {
    let blueprint = factory::design_org_from_mission(name, mission, budget);
    println!("{}", orgchart::render_orgchart(&blueprint));

    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt("Create this org? [Y/n]")
            .default(true)
            .interact()
            .map_err(|e| CliError::InvalidInput(format!("prompt failed: {e}")))?;
        if !confirmed {
            println!("Aborted.");
            return Ok(());
        }
    }

    let result = provisioner::provision_org(&blueprint, api_url)
        .map_err(|e| CliError::ApiCallFailed(format!("provision failed: {e}")))?;

    println!(
        "Org '{}' created! ID: {}, Plan: #{}, {} agents, {} night agents",
        name,
        result.org_id,
        result.plan_id,
        result.agents_created,
        result.night_agents_scheduled,
    );
    Ok(())
}

/// Create an org from a scanned repo, show orgchart, confirm, provision.
pub async fn handle_create_org_from(
    path: &str,
    name: Option<&str>,
    budget: f64,
    yes: bool,
    api_url: &str,
) -> Result<(), CliError> {
    let profile = repo_scanner::scan_repo(Path::new(path))
        .map_err(|e| CliError::InvalidInput(format!("scan failed: {e}")))?;

    let lang_list: Vec<&str> = profile.languages.iter().map(|(l, _): &(String, usize)| l.as_str()).collect();
    println!(
        "Analyzed: {}, {}, {} files",
        lang_list.join(", "),
        profile.frameworks.join(", "),
        profile.total_files,
    );

    let blueprint = factory::design_org_from_repo(&profile, name, budget);
    println!("{}", orgchart::render_orgchart(&blueprint));

    if !yes {
        let confirmed = dialoguer::Confirm::new()
            .with_prompt("Create this org? [Y/n]")
            .default(true)
            .interact()
            .map_err(|e| CliError::InvalidInput(format!("prompt failed: {e}")))?;
        if !confirmed {
            println!("Aborted.");
            return Ok(());
        }
    }

    let result = provisioner::provision_org(&blueprint, api_url)
        .map_err(|e| CliError::ApiCallFailed(format!("provision failed: {e}")))?;

    let display_name = name.unwrap_or(&blueprint.name);
    println!(
        "Org '{}' created! ID: {}, Plan: #{}, {} agents, {} night agents",
        display_name,
        result.org_id,
        result.plan_id,
        result.agents_created,
        result.night_agents_scheduled,
    );
    Ok(())
}
