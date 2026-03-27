use super::markdown_export::import_from_markdown;
use super::sqlite_store::SqliteMemoryStore;
use super::types::MemoryError;
use super::vector_store::VectorStore;
use super::MemoryStore;
use std::path::Path;

/// Recovery chain: Markdown → SQLite → LanceDB/VectorStore.
/// Each layer rebuilds from the previous one.
pub struct Reindexer {
    db_path: String,
}

impl Reindexer {
    pub fn new(db_path: &str) -> Self {
        Self {
            db_path: db_path.to_string(),
        }
    }

    /// Rebuild vector store from SQLite memories.
    /// Use when LanceDB/vector index is corrupted but SQLite is intact.
    pub fn reindex_vectors(&self) -> Result<ReindexReport, MemoryError> {
        let store = SqliteMemoryStore::new(&self.db_path)?;
        let vs = VectorStore::new(&self.db_path)?;
        let all = store.recall(super::types::RecallQuery {
            limit: 100_000,
            ..Default::default()
        })?;
        let mut indexed = 0;
        for mem in &all {
            vs.store(&mem.id, &mem.content)?;
            indexed += 1;
        }
        Ok(ReindexReport {
            source: "sqlite".to_string(),
            target: "vector_store".to_string(),
            items_processed: indexed,
        })
    }

    /// Rebuild SQLite from Markdown exports.
    /// Use when SQLite is corrupted but Markdown files are intact.
    pub fn rebuild_from_markdown(&self, memory_dir: &Path) -> Result<ReindexReport, MemoryError> {
        let memories = import_from_markdown(memory_dir)?;
        let store = SqliteMemoryStore::new(&self.db_path)?;
        let mut imported = 0;
        for mem in &memories {
            // Skip if already exists (idempotent rebuild).
            if store
                .recall(super::types::RecallQuery {
                    text_search: Some(mem.id.clone()),
                    limit: 1,
                    ..Default::default()
                })
                .map(|r| r.is_empty())
                .unwrap_or(true)
            {
                store.remember(mem.clone())?;
                imported += 1;
            }
        }
        Ok(ReindexReport {
            source: "markdown".to_string(),
            target: "sqlite".to_string(),
            items_processed: imported,
        })
    }

    /// Full recovery: Markdown → SQLite → VectorStore.
    pub fn full_rebuild(&self, memory_dir: &Path) -> Result<Vec<ReindexReport>, MemoryError> {
        let mut reports = Vec::new();
        reports.push(self.rebuild_from_markdown(memory_dir)?);
        reports.push(self.reindex_vectors()?);
        Ok(reports)
    }
}

/// Report from a reindex operation.
#[derive(Debug, Clone)]
pub struct ReindexReport {
    pub source: String,
    pub target: String,
    pub items_processed: usize,
}

impl std::fmt::Display for ReindexReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} → {}: {} items",
            self.source, self.target, self.items_processed
        )
    }
}

#[cfg(test)]
#[path = "reindex_tests.rs"]
mod tests;
