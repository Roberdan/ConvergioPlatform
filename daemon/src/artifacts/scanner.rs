use super::registry::ArtifactRegistry;
use super::types::{Artifact, ArtifactError, ArtifactType, Maturity};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Scan directories for artifact definitions and populate the registry.
pub fn scan_artifacts(
    base_path: &Path,
    registry: &ArtifactRegistry,
) -> Result<usize, ArtifactError> {
    let mut count = 0;

    // Scan agents
    let agents_dir = base_path.join("claude-config/agents");
    if agents_dir.exists() {
        count += scan_dir(&agents_dir, "agent", ArtifactType::Agent, registry)?;
    }
    let gh_agents = base_path.join(".github/agents");
    if gh_agents.exists() {
        count += scan_dir(&gh_agents, "agent", ArtifactType::Agent, registry)?;
    }

    // Scan skills
    let skills_dir = base_path.join("claude-config/skills");
    if skills_dir.exists() {
        count += scan_dir(&skills_dir, "skill", ArtifactType::Skill, registry)?;
    }

    // Scan rules
    let rules_dir = base_path.join("claude-config/rules");
    if rules_dir.exists() {
        count += scan_dir(&rules_dir, "rule", ArtifactType::Rule, registry)?;
    }

    Ok(count)
}

fn scan_dir(
    dir: &Path,
    domain: &str,
    artifact_type: ArtifactType,
    registry: &ArtifactRegistry,
) -> Result<usize, ArtifactError> {
    let entries = fs::read_dir(dir)
        .map_err(|e| ArtifactError::ScanError(format!("read {}: {e}", dir.display())))?;
    let mut count = 0;

    for entry in entries.filter_map(|e| match e {
        Ok(v) => Some(v),
        Err(e) => { tracing::warn!("artifact scan readdir: {e}"); None }
    }) {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" && ext != "yaml" && ext != "yml" {
            continue;
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| ArtifactError::ScanError(format!("read {}: {e}", path.display())))?;

        let artifact = parse_artifact(&path, &content, domain, artifact_type.clone())?;
        registry.register(artifact)?;
        count += 1;
    }
    Ok(count)
}

fn parse_artifact(
    path: &Path,
    content: &str,
    domain: &str,
    artifact_type: ArtifactType,
) -> Result<Artifact, ArtifactError> {
    let name = extract_frontmatter_field(content, "name")
        .or_else(|| path.file_stem().map(|s| s.to_string_lossy().to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let description = extract_frontmatter_field(content, "description")
        .unwrap_or_default();

    let model = extract_frontmatter_field(content, "model");

    let maturity = extract_frontmatter_field(content, "maturity")
        .map(|m| Maturity::from_str(&m))
        .unwrap_or(Maturity::Stable);

    let hash = sha256_hex(content.as_bytes());

    Ok(Artifact {
        id: 0,
        artifact_type,
        name,
        description,
        domain: domain.to_string(),
        maturity,
        source_path: path.to_string_lossy().to_string(),
        file_hash: hash,
        model,
        constraints: vec![],
        metadata: serde_json::json!({}),
    })
}

fn extract_frontmatter_field(content: &str, field: &str) -> Option<String> {
    let pattern = format!("{field}:");
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&pattern) {
            let val = rest.trim().trim_matches('"').trim_matches('\'');
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}
