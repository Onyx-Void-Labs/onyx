// ─── Onyx Core — Content-Addressable Blob Store ────────────────────
// Stores binary data by SHA-256 hash. Metadata tracked in a LoroMap.
// ───────────────────────────────────────────────────────────────────

use std::collections::HashMap;

use sha2::{Digest, Sha256};

use anyhow::Result;
use thiserror::Error;

/// Errors returned by BlobStore operations.
#[derive(Debug, Error)]
pub enum BlobError {
    #[error("blob not found")]
    NotFound,
    #[error("blob corrupted")]
    Corruption,
}

/// A content-addressable blob store. Blobs are keyed by their SHA-256 hash.
pub struct BlobStore {
    blobs: HashMap<String, Vec<u8>>,
    metadata: HashMap<String, BlobMeta>,
    refcounts: HashMap<String, usize>,
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
            refcounts: HashMap::new(),
        }
    }

    /// Store a blob and return its SHA-256 hex hash.
    /// The hash is computed over the plaintext for content-addressing
    /// (deduplication). The stored bytes are encrypted.
    pub fn store_blob(&mut self, data: &[u8], mime: &str) -> String {
        // Content-address by plaintext hash (deterministic for dedup)
        let hash = Self::sha256_hex(data);
        if self.blobs.contains_key(&hash) {
            // Already stored — just bump refcount
            *self.refcounts.entry(hash.clone()).or_insert(0) += 1;
            return hash;
        }
        // Encrypt blob with workspace key
        let key = match crate::persistence::dev_encryption_key() {
            Ok(k) => k,
            Err(_) => return String::new(),
        };
        let encrypted = match crate::crypto::encrypt_data(data, &*key) {
            Ok(e) => e,
            Err(_) => return String::new(),
        };
        self.blobs.insert(hash.clone(), encrypted);
        *self.refcounts.entry(hash.clone()).or_insert(0) += 1;
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
    /// Decrypts the stored ciphertext and re-verifies the plaintext hash
    /// to detect corruption.
    pub fn get_blob(&self, hash: &str) -> Result<Vec<u8>, BlobError> {
        match self.blobs.get(hash) {
            Some(encrypted) => {
                // decrypt — wrap in Zeroizing so plaintext is wiped on drop
                let key =
                    crate::persistence::dev_encryption_key().map_err(|_| BlobError::Corruption)?;
                let decrypted = zeroize::Zeroizing::new(
                    crate::crypto::decrypt_data(encrypted, &*key)
                        .map_err(|_| BlobError::Corruption)?,
                );
                // verify plaintext hash matches the content-address key
                let computed = Self::sha256_hex(&decrypted);
                if computed != hash {
                    return Err(BlobError::Corruption);
                }
                Ok(decrypted.to_vec())
            }
            None => Err(BlobError::NotFound),
        }
    }

    /// Delete a blob by its hash. Respects reference counts: if the blob
    /// is still referenced elsewhere, only decrements the count.
    pub fn delete_blob(&mut self, hash: &str) -> anyhow::Result<()> {
        let count = self
            .refcounts
            .get_mut(hash)
            .ok_or_else(|| anyhow::anyhow!("blob not found: {}", hash))?;

        if *count > 1 {
            *count -= 1;
            return Ok(()); // still referenced — decrement only
        }

        // count <= 1: safe to fully remove
        self.blobs.remove(hash);
        self.metadata.remove(hash);
        self.refcounts.remove(hash);
        Ok(())
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
    use anyhow::Result;

    #[test]
    fn store_and_retrieve() -> Result<()> {
        let mut store = BlobStore::new();
        let data = b"hello onyx";
        let hash = store.store_blob(data, "text/plain");

        assert!(store.has_blob(&hash));
        match store.get_blob(&hash) {
            Ok(blob) => assert_eq!(blob, data),
            Err(_) => panic!("blob missing or corrupted"),
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
