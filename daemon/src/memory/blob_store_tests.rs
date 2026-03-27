use super::*;
use tempfile::TempDir;

fn temp_store() -> (BlobStore, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let store = BlobStore::new(dir.path()).expect("blob store");
    (store, dir)
}

#[test]
fn store_and_load_roundtrip() {
    let (bs, _d) = temp_store();
    let data = b"the quick brown fox jumps over the lazy dog";
    let hash = bs.store(data).expect("store");
    assert_eq!(hash.len(), 64, "SHA-256 hex should be 64 chars");
    let loaded = bs.load(&hash).expect("load");
    assert_eq!(loaded, data);
}

#[test]
fn deterministic_hash() {
    let (bs, _d) = temp_store();
    let h1 = bs.store(b"deterministic").expect("store1");
    let h2 = bs.store(b"deterministic").expect("store2");
    assert_eq!(h1, h2, "same content must produce same hash");
}

#[test]
fn dedup_no_extra_files() {
    let (bs, _d) = temp_store();
    bs.store(b"dedup test").expect("store1");
    bs.store(b"dedup test").expect("store2");
    assert_eq!(bs.count().expect("count"), 1, "dedup should not create extra files");
}

#[test]
fn delete_removes_blob() {
    let (bs, _d) = temp_store();
    let hash = bs.store(b"to be deleted").expect("store");
    assert!(bs.exists(&hash));
    bs.delete(&hash).expect("delete");
    assert!(!bs.exists(&hash));
}

#[test]
fn delete_nonexistent_is_ok() {
    let (bs, _d) = temp_store();
    bs.delete("nonexistent-hash").expect("delete should not fail");
}

#[test]
fn load_nonexistent_returns_not_found() {
    let (bs, _d) = temp_store();
    let err = bs.load("nonexistent-hash").unwrap_err();
    assert!(matches!(err, MemoryError::NotFound(_)));
}

#[test]
fn count_tracks_blobs() {
    let (bs, _d) = temp_store();
    assert_eq!(bs.count().unwrap(), 0);
    bs.store(b"blob one").unwrap();
    bs.store(b"blob two").unwrap();
    assert_eq!(bs.count().unwrap(), 2);
}

#[test]
fn total_bytes_correct() {
    let (bs, _d) = temp_store();
    bs.store(b"hello").unwrap(); // 5 bytes
    bs.store(b"world!!").unwrap(); // 7 bytes
    assert_eq!(bs.total_bytes().unwrap(), 12);
}

#[test]
fn gc_removes_unreferenced() {
    let (bs, _d) = temp_store();
    let h1 = bs.store(b"keep this").unwrap();
    let _h2 = bs.store(b"remove this").unwrap();
    let removed = bs.gc(&[h1.clone()]).unwrap();
    assert_eq!(removed, 1);
    assert_eq!(bs.count().unwrap(), 1);
    assert!(bs.exists(&h1));
}

#[test]
fn max_size_enforced() {
    let (bs, _d) = temp_store();
    let bs = BlobStore::new(bs.root.clone()).unwrap().with_max_size(10);
    let err = bs.store(&vec![0u8; 11]).unwrap_err();
    assert!(matches!(err, MemoryError::StorageError(_)));
}

#[test]
fn exists_returns_correct_state() {
    let (bs, _d) = temp_store();
    assert!(!bs.exists("no-such-hash"));
    let hash = bs.store(b"exists test").unwrap();
    assert!(bs.exists(&hash));
}
