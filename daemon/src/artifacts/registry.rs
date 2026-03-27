use super::types::{Artifact, ArtifactError, ArtifactType, Maturity};
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory artifact registry. Populated by scanner, queried by renderers.
pub struct ArtifactRegistry {
    artifacts: RwLock<HashMap<u64, Artifact>>,
    next_id: RwLock<u64>,
}

impl ArtifactRegistry {
    pub fn new() -> Self {
        Self {
            artifacts: RwLock::new(HashMap::new()),
            next_id: RwLock::new(1),
        }
    }

    /// Register or update an artifact. Returns the assigned ID.
    pub fn register(&self, mut artifact: Artifact) -> Result<u64, ArtifactError> {
        let mut artifacts = self.artifacts.write()
            .map_err(|e| ArtifactError::StorageError(format!("lock: {e}")))?;

        // Check for existing by source_path (idempotent upsert).
        let existing_id = artifacts.values()
            .find(|a| a.source_path == artifact.source_path)
            .map(|a| a.id);

        if let Some(id) = existing_id {
            artifact.id = id;
            artifacts.insert(id, artifact);
            return Ok(id);
        }

        let mut id_counter = self.next_id.write()
            .map_err(|e| ArtifactError::StorageError(format!("lock: {e}")))?;
        let id = *id_counter;
        *id_counter += 1;
        artifact.id = id;
        artifacts.insert(id, artifact);
        Ok(id)
    }

    /// Get an artifact by ID.
    pub fn get(&self, id: u64) -> Result<Artifact, ArtifactError> {
        self.artifacts.read()
            .map_err(|e| ArtifactError::StorageError(format!("lock: {e}")))?
            .get(&id)
            .cloned()
            .ok_or_else(|| ArtifactError::NotFound(format!("id {id}")))
    }

    /// List artifacts with optional filters.
    pub fn list(
        &self,
        type_filter: Option<ArtifactType>,
        domain_filter: Option<&str>,
        maturity_filter: Option<Maturity>,
    ) -> Vec<Artifact> {
        let artifacts = self.artifacts.read().unwrap_or_else(|e| e.into_inner());
        let mut result: Vec<Artifact> = artifacts.values()
            .filter(|a| type_filter.as_ref().map(|t| a.artifact_type == *t).unwrap_or(true))
            .filter(|a| domain_filter.map(|d| a.domain == d).unwrap_or(true))
            .filter(|a| maturity_filter.as_ref().map(|m| a.maturity == *m).unwrap_or(true))
            .cloned()
            .collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    pub fn count(&self) -> usize {
        self.artifacts.read().map(|a| a.len()).unwrap_or(0)
    }
}

impl Default for ArtifactRegistry {
    fn default() -> Self {
        Self::new()
    }
}
