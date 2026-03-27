use super::registry::ArtifactRegistry;
use super::types::{ArtifactError, ArtifactType};
use serde::{Deserialize, Serialize};

/// Output format for artifact rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OutputFormat {
    Report,
    VsCodeExtension,
    OpenClaw,
    Api,
}

/// A single rendered output file.
#[derive(Debug, Clone)]
pub struct OutputFile {
    pub relative_path: String,
    pub content: Vec<u8>,
    pub content_type: String,
}

/// Render options passed to renderers.
#[derive(Debug, Clone, Default)]
pub struct RenderOptions {
    pub output_dir: Option<String>,
    pub include_deprecated: bool,
}

/// Render a registry to the specified format.
pub fn render(
    registry: &ArtifactRegistry,
    format: OutputFormat,
    _options: &RenderOptions,
) -> Result<Vec<OutputFile>, ArtifactError> {
    match format {
        OutputFormat::Report => render_report(registry),
        OutputFormat::VsCodeExtension => render_vscode(registry),
        OutputFormat::OpenClaw => render_openclaw(registry),
        OutputFormat::Api => render_api_json(registry),
    }
}

fn render_report(registry: &ArtifactRegistry) -> Result<Vec<OutputFile>, ArtifactError> {
    let all = registry.list(None, None, None);
    let agents = all.iter().filter(|a| a.artifact_type == ArtifactType::Agent).count();
    let skills = all.iter().filter(|a| a.artifact_type == ArtifactType::Skill).count();
    let rules = all.iter().filter(|a| a.artifact_type == ArtifactType::Rule).count();

    let mut md = String::from("# Convergio Platform Report\n\n");
    md.push_str(&format!("Total artifacts: {} ({} agents, {} skills, {} rules)\n\n", all.len(), agents, skills, rules));
    md.push_str("| Name | Type | Domain | Maturity | Model |\n");
    md.push_str("|------|------|--------|----------|-------|\n");
    for a in &all {
        md.push_str(&format!(
            "| {} | {:?} | {} | {:?} | {} |\n",
            a.name, a.artifact_type, a.domain, a.maturity,
            a.model.as_deref().unwrap_or("-")
        ));
    }

    Ok(vec![OutputFile {
        relative_path: "REPORT.md".to_string(),
        content: md.into_bytes(),
        content_type: "text/markdown".to_string(),
    }])
}

fn render_vscode(registry: &ArtifactRegistry) -> Result<Vec<OutputFile>, ArtifactError> {
    let agents = registry.list(Some(ArtifactType::Agent), None, None);
    let mut files = Vec::new();

    for agent in &agents {
        let content = format!(
            "---\nagent: {}\ndescription: {}\nmodel: {}\n---\n",
            agent.name, agent.description, agent.model.as_deref().unwrap_or("sonnet-4.6")
        );
        files.push(OutputFile {
            relative_path: format!("agents/{}.agent.md", agent.name),
            content: content.into_bytes(),
            content_type: "text/markdown".to_string(),
        });
    }

    Ok(files)
}

fn render_openclaw(registry: &ArtifactRegistry) -> Result<Vec<OutputFile>, ArtifactError> {
    let skills = registry.list(Some(ArtifactType::Skill), None, None);
    let mut files = Vec::new();

    for skill in &skills {
        let content = format!("# {}\n\n{}\n", skill.name, skill.description);
        files.push(OutputFile {
            relative_path: format!("skills/{}.md", skill.name),
            content: content.into_bytes(),
            content_type: "text/markdown".to_string(),
        });
    }

    // Index
    let index = serde_json::json!({
        "skills": skills.iter().map(|s| &s.name).collect::<Vec<_>>(),
        "count": skills.len()
    });
    files.push(OutputFile {
        relative_path: "index.json".to_string(),
        content: serde_json::to_string_pretty(&index).unwrap_or_default().into_bytes(),
        content_type: "application/json".to_string(),
    });

    Ok(files)
}

fn render_api_json(registry: &ArtifactRegistry) -> Result<Vec<OutputFile>, ArtifactError> {
    let all = registry.list(None, None, None);
    let json = serde_json::to_string_pretty(&all)
        .map_err(|e| ArtifactError::RenderError(format!("serialize: {e}")))?;
    Ok(vec![OutputFile {
        relative_path: "artifacts.json".to_string(),
        content: json.into_bytes(),
        content_type: "application/json".to_string(),
    }])
}
