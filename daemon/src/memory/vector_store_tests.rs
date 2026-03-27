use super::*;

fn temp_store() -> VectorStore {
    VectorStore::new(":memory:").expect("in-memory vector store")
}

#[test]
fn store_and_count() {
    let vs = temp_store();
    vs.store("mem-001", "kubernetes deployment pipeline").unwrap();
    vs.store("mem-002", "rust compiler optimisations").unwrap();
    assert_eq!(vs.count().unwrap(), 2);
}

#[test]
fn search_returns_similar() {
    let vs = temp_store();
    vs.store("mem-k8s", "kubernetes deployment pipeline cluster pods").unwrap();
    vs.store("mem-rust", "rust memory safety borrow checker lifetime").unwrap();
    vs.store("mem-cook", "italian pasta recipe tomato basil mozzarella").unwrap();

    let results = vs.search("kubernetes cluster deployment", 10, 0.0).unwrap();
    assert!(!results.is_empty(), "should return results");
    assert_eq!(
        results[0].memory_id, "mem-k8s",
        "kubernetes memory should rank first"
    );
}

#[test]
fn search_respects_threshold() {
    let vs = temp_store();
    vs.store("mem-a", "alpha beta gamma").unwrap();
    vs.store("mem-b", "completely different topic about marine biology").unwrap();

    let results = vs.search("alpha beta gamma", 10, 0.99).unwrap();
    // Only exact or near-exact matches should pass a high threshold.
    assert!(
        results.len() <= 1,
        "high threshold should filter dissimilar, got {}",
        results.len()
    );
}

#[test]
fn search_respects_limit() {
    let vs = temp_store();
    for i in 0..20 {
        vs.store(&format!("mem-{i}"), &format!("memory number {i} about topics"))
            .unwrap();
    }
    let results = vs.search("memory about topics", 5, 0.0).unwrap();
    assert!(results.len() <= 5, "limit should cap results");
}

#[test]
fn delete_removes_vector() {
    let vs = temp_store();
    vs.store("mem-del", "content to delete").unwrap();
    assert_eq!(vs.count().unwrap(), 1);
    vs.delete("mem-del").unwrap();
    assert_eq!(vs.count().unwrap(), 0);
}

#[test]
fn delete_nonexistent_is_ok() {
    let vs = temp_store();
    vs.delete("nonexistent").unwrap();
}

#[test]
fn store_replaces_on_duplicate() {
    let vs = temp_store();
    vs.store("mem-dup", "first version").unwrap();
    vs.store("mem-dup", "second version completely different").unwrap();
    assert_eq!(vs.count().unwrap(), 1);

    let results = vs.search("second version completely different", 1, 0.0).unwrap();
    assert_eq!(results[0].memory_id, "mem-dup");
}

#[test]
fn search_empty_store_returns_empty() {
    let vs = temp_store();
    let results = vs.search("anything", 10, 0.0).unwrap();
    assert!(results.is_empty());
}

#[test]
fn scores_are_sorted_descending() {
    let vs = temp_store();
    vs.store("m1", "the cat sat on the mat").unwrap();
    vs.store("m2", "the cat sat on the mat purring loudly").unwrap();
    vs.store("m3", "quantum physics entanglement experiment").unwrap();

    let results = vs.search("the cat sat on the mat", 10, 0.0).unwrap();
    for pair in results.windows(2) {
        assert!(
            pair[0].score >= pair[1].score,
            "results should be sorted by score descending"
        );
    }
}
