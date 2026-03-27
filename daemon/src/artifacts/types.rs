use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactType {
    Agent,
    Skill,
    Rule,
    Standard,
    Template,
}

impl ArtifactType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "agent" => Self::Agent,
            "skill" => Self::Skill,
            "rule" => Self::Rule,
            "standard" => Self::Standard,
            "template" => Self::Template,
            _ => Self::Template,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Maturity {
    Draft,
    Experimental,
    Preview,
    Stable,
    Deprecated,
}

impl Maturity {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "draft" => Self::Draft,
            "experimental" => Self::Experimental,
            "preview" => Self::Preview,
            "stable" => Self::Stable,
            "deprecated" => Self::Deprecated,
            _ => Self::Draft,
        }
    }
}

/// A registered artifact in the ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: u64,
    pub artifact_type: ArtifactType,
    pub name: String,
    pub description: String,
    pub domain: String,
    pub maturity: Maturity,
    pub source_path: String,
    pub file_hash: String,
    pub model: Option<String>,
    pub constraints: Vec<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("artifact not found: {0}")]
    NotFound(String),
    #[error("scan error: {0}")]
    ScanError(String),
    #[error("storage error: {0}")]
    StorageError(String),
    #[error("render error: {0}")]
    RenderError(String),
}
