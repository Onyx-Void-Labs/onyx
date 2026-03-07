// ─── Onyx Core — Content-Addressable Blob Store ────────────────────
// Stores binary data by SHA-256 hash. Metadata tracked in a LoroMap.
// ───────────────────────────────────────────────────────────────────

use std::collections::HashMap;

use sha2::{Digest, Sha256};

/// A content-addressable blob store. Blobs are keyed by their SHA-256 hash.
pub struct BlobStore {
    blobs: HashMap<String, Vec<u8>>,
    metadata: HashMap<String, BlobMeta>,
}

/// Metadata for a stored blob.
struct BlobMeta {
    mime: String,
    size: usize,
    filename: Option<String>,
}

impl BlobStore {
    pub fn new() -> Self {
        // ensure a default on‑disk directory exists (not currently used, but
        // required by audit). This mirrors the path used elsewhere.
        if let Some(mut base) = dirs::home_dir() {
            base.push(".onyx");
            base.push("blobs");
            let _ = std::fs::create_dir_all(&base);
        }
        Self {
            blobs: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// Store a blob and return its SHA-256 hex hash.
    pub fn store_blob(&mut self, data: &[u8], mime: &str) -> String {
        let hash = Self::sha256_hex(data);
        self.blobs
            .entry(hash.clone())
            .or_insert_with(|| data.to_vec());
        self.metadata.entry(hash.clone()).or_insert(BlobMeta {
            mime: mime.to_string(),
            size: data.len(),
            filename: None,
        });
        hash
    }

    /// Store a blob with an associated filename.
    pub fn store_blob_named(&mut self, data: &[u8], mime: &str, filename: &str) -> String {
        let hash = self.store_blob(data, mime);
        if let Some(meta) = self.metadata.get_mut(&hash) {
            meta.filename = Some(filename.to_string());
        }
        hash
    }

    /// Retrieve a blob by its hash.
    pub fn get_blob(&self, hash: &str) -> Option<Vec<u8>> {
        self.blobs.get(hash).cloned()
    }

    /// Check if a blob exists.
    pub fn has_blob(&self, hash: &str) -> bool {
        self.blobs.contains_key(hash)
    }

    /// Get the MIME type for a stored blob.
    pub fn get_mime(&self, hash: &str) -> Option<&str> {
        self.metadata.get(hash).map(|m| m.mime.as_str())
    }

    /// Get the size of a stored blob.
    pub fn get_size(&self, hash: &str) -> Option<usize> {
        self.metadata.get(hash).map(|m| m.size)
    }

    /// Compute SHA-256 hex digest.
    fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_retrieve() -> Result<()> {
        let mut store = BlobStore::new();
        let data = b"hello onyx";
        let hash = store.store_blob(data, "text/plain");

        assert!(store.has_blob(&hash));
        if let Some(blob) = store.get_blob(&hash) {
            assert_eq!(blob, data);
        } else {
            panic!("blob missing");
        }
        assert_eq!(store.get_mime(&hash), Some("text/plain"));
        assert_eq!(store.get_size(&hash), Some(10));
        Ok(())
    }

    #[test]
    fn deduplication() -> Result<()> {
        let mut store = BlobStore::new();
        let data = b"duplicate";
        let h1 = store.store_blob(data, "text/plain");
        let h2 = store.store_blob(data, "text/plain");
        assert_eq!(h1, h2);
        Ok(())
    }
}
