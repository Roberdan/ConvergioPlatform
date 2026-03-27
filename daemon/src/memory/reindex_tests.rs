use super::*;
use crate::memory::markdown_export::export_memory;
use crate::memory::sqlite_store::SqliteMemoryStore;
use crate::memory::types::{AccessLevel, Memory, MemoryType, RecallQuery};
use crate::memory::vector_store::VectorStore;
use crate::memory::MemoryStore;
use chrono::Utc;
use tempfile::{NamedTempFile, TempDir};

fn make_mem(id: &str, content: &str) -> Memory {
    Memory {
        id: id.to_string(),
        agent_id: "agent-reindex".to_string(),
        memory_type: MemoryType::Fact,
        content: content.to_string(),
        tags: vec!["reindex".to_string()],
        created_at: Utc::now(),
        expires_at: None,
        access_level: AccessLevel::Private,
        attestations: vec![],
    }
}

#[test]
fn reindex_vectors_from_sqlite() {
    let db = NamedTempFile::new().unwrap();
    let db_path = db.path().to_str().unwrap();
    let store = SqliteMemoryStore::new(db_path).unwrap();
    store.remember(make_mem("r-001", "Vector reindex test one")).unwrap();
    store.remember(make_mem("r-002", "Vector reindex test two")).unwrap();

    let reindexer = Reindexer::new(db_path);
    let report = reindexer.reindex_vectors().unwrap();
    assert_eq!(report.items_processed, 2);
    assert_eq!(report.target, "vector_store");

    let vs = VectorStore::new(db_path).unwrap();
    assert_eq!(vs.count().unwrap(), 2);
}

#[test]
fn rebuild_from_markdown() {
    let md_dir = TempDir::new().unwrap();
    let db = NamedTempFile::new().unwrap();
    let db_path = db.path().to_str().unwrap();

    // Export memories to markdown
    export_memory(&make_mem("md-001", "Markdown rebuild test"), md_dir.path()).unwrap();

    // Rebuild SQLite from markdown
    let reindexer = Reindexer::new(db_path);
    let report = reindexer.rebuild_from_markdown(md_dir.path()).unwrap();
    assert!(report.items_processed > 0);

    let store = SqliteMemoryStore::new(db_path).unwrap();
    let all = store.recall(RecallQuery { limit: 100, ..Default::default() }).unwrap();
    assert!(!all.is_empty());
}

#[test]
fn full_rebuild_chain() {
    let md_dir = TempDir::new().unwrap();
    let db = NamedTempFile::new().unwrap();
    let db_path = db.path().to_str().unwrap();

    export_memory(&make_mem("chain-001", "Full chain test"), md_dir.path()).unwrap();

    let reindexer = Reindexer::new(db_path);
    let reports = reindexer.full_rebuild(md_dir.path()).unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].source, "markdown");
    assert_eq!(reports[1].source, "sqlite");
}
