use super::types::MemoryError;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

/// Content-addressed blob store for large memory artifacts.
/// Files stored as `data/blobs/<sha256-hex>` with dedup via hash.
pub struct BlobStore {
    root: PathBuf,
    max_blob_bytes: u64,
}

impl BlobStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, MemoryError> {
        let root = root.into();
        fs::create_dir_all(&root)
            .map_err(|e| MemoryError::StorageError(format!("cannot create blobs dir: {e}")))?;
        Ok(Self {
            root,
            max_blob_bytes: 100 * 1024 * 1024, // 100 MiB default
        })
    }

    /// Override the per-blob size limit.
    pub fn with_max_size(mut self, bytes: u64) -> Self {
        self.max_blob_bytes = bytes;
        self
    }

    /// Store bytes, returning the SHA-256 hex hash.
    /// Deduplicates: if the hash already exists, returns immediately.
    pub fn store(&self, data: &[u8]) -> Result<String, MemoryError> {
        if data.len() as u64 > self.max_blob_bytes {
            return Err(MemoryError::StorageError(format!(
                "blob size {} exceeds limit {}",
                data.len(),
                self.max_blob_bytes
            )));
        }
        let hash = sha256_hex(data);
        let path = self.blob_path(&hash);
        if path.exists() {
            return Ok(hash);
        }
        // Write to temp then rename for atomicity.
        let tmp = self.root.join(format!(".tmp-{hash}"));
        let mut file = fs::File::create(&tmp)
            .map_err(|e| MemoryError::StorageError(format!("create blob: {e}")))?;
        file.write_all(data)
            .map_err(|e| MemoryError::StorageError(format!("write blob: {e}")))?;
        fs::rename(&tmp, &path)
            .map_err(|e| MemoryError::StorageError(format!("rename blob: {e}")))?;
        Ok(hash)
    }

    /// Retrieve blob bytes by hash.
    pub fn load(&self, hash: &str) -> Result<Vec<u8>, MemoryError> {
        let path = self.blob_path(hash);
        if !path.exists() {
            return Err(MemoryError::NotFound(format!("blob {hash}")));
        }
        let mut file = fs::File::open(&path)
            .map_err(|e| MemoryError::StorageError(format!("open blob: {e}")))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| MemoryError::StorageError(format!("read blob: {e}")))?;
        Ok(buf)
    }

    /// Delete a blob by hash.
    pub fn delete(&self, hash: &str) -> Result<(), MemoryError> {
        let path = self.blob_path(hash);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| MemoryError::StorageError(format!("delete blob: {e}")))?;
        }
        Ok(())
    }

    /// Check if a blob exists.
    pub fn exists(&self, hash: &str) -> bool {
        self.blob_path(hash).exists()
    }

    /// Count stored blobs.
    pub fn count(&self) -> Result<usize, MemoryError> {
        let entries = fs::read_dir(&self.root)
            .map_err(|e| MemoryError::StorageError(format!("read blobs dir: {e}")))?;
        Ok(entries
            .filter_map(|e| e.ok())
            .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .count())
    }

    /// Total bytes used by all blobs.
    pub fn total_bytes(&self) -> Result<u64, MemoryError> {
        let entries = fs::read_dir(&self.root)
            .map_err(|e| MemoryError::StorageError(format!("read blobs dir: {e}")))?;
        let total = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum();
        Ok(total)
    }

    /// Garbage collect: remove blobs not in the referenced set.
    pub fn gc(&self, referenced: &[String]) -> Result<usize, MemoryError> {
        let entries = fs::read_dir(&self.root)
            .map_err(|e| MemoryError::StorageError(format!("read blobs dir: {e}")))?;
        let mut removed = 0;
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if !referenced.contains(&name) {
                fs::remove_file(entry.path())
                    .map_err(|e| MemoryError::StorageError(format!("gc remove: {e}")))?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    fn blob_path(&self, hash: &str) -> PathBuf {
        self.root.join(hash)
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
#[path = "blob_store_tests.rs"]
mod tests;
