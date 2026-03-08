use std::collections::HashMap;
use std::path::PathBuf;
use sha2::{Digest, Sha256};
use anyhow::Result;
use thiserror::Error;

const MAX_BLOB_SIZE: u64 = 50 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum BlobError {
    #[error("blob not found")]
    NotFound,
    #[error("blob corrupted")]
    Corruption,
    #[error("blob too large ({0} bytes, limit {1})")]
    TooLarge(u64, u64),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

struct BlobMeta {
    mime: String,
    size: usize,
    filename: Option<String>,
}

pub struct BlobStore {
    base_dir: PathBuf,
    metadata: HashMap<String, BlobMeta>,
    refcounts: HashMap<String, usize>,
}

impl Default for BlobStore {
    fn default() -> Self {
        Self::new()
    }
}

impl BlobStore {
    pub fn new() -> Self {
        let base_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".onyx")
            .join("blobs");
        let _ = std::fs::create_dir_all(&base_dir);
        Self {
            base_dir,
            metadata: HashMap::new(),
            refcounts: HashMap::new(),
        }
    }

    /// Path-based constructor for testing isolation.
    pub fn new_with_path(path: std::path::PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&path);
        Self {
            base_dir: path,
            metadata: HashMap::new(),
            refcounts: HashMap::new(),
        }
    }

    pub fn store_blob_raw(&mut self, data: &[u8]) -> anyhow::Result<String> {
        Ok(self.store_blob(data, "application/octet-stream"))
    }

    pub fn clone_ref(&mut self, hash: &str) -> anyhow::Result<()> {
        self.get_blob(hash).map_err(|e| anyhow::anyhow!("{e}"))?;
        *self.refcounts.entry(hash.to_string()).or_insert(1) += 1;
        Ok(())
    }

    fn blob_path(&self, hash: &str) -> PathBuf {
        self.base_dir.join(hash)
    }

    pub fn store_blob(&mut self, data: &[u8], mime: &str) -> String {
        let hash = Self::sha256_hex(data);
        if self.blob_path(&hash).exists() {
            *self.refcounts.entry(hash.clone()).or_insert(0) += 1;
            return hash;
        }
        let key = match crate::persistence::dev_encryption_key() {
            Ok(k) => k,
            Err(_) => return String::new(),
        };
        let encrypted = match crate::crypto::encrypt_data(data, &key) {
            Ok(e) => e,
            Err(_) => return String::new(),
        };
        if std::fs::write(self.blob_path(&hash), &encrypted).is_err() {
            return String::new();
        }
        *self.refcounts.entry(hash.clone()).or_insert(0) += 1;
        self.metadata.entry(hash.clone()).or_insert(BlobMeta {
            mime: mime.to_string(),
            size: data.len(),
            filename: None,
        });
        hash
    }

    pub fn store_blob_named(&mut self, data: &[u8], mime: &str, filename: &str) -> String {
        let hash = self.store_blob(data, mime);
        if let Some(meta) = self.metadata.get_mut(&hash) {
            meta.filename = Some(filename.to_string());
        }
        hash
    }

    pub fn get_blob(&self, hash: &str) -> Result<Vec<u8>, BlobError> {
        let path = self.blob_path(hash);
        if !path.exists() {
            return Err(BlobError::NotFound);
        }
        let file_size = std::fs::metadata(&path)?.len();
        if file_size > MAX_BLOB_SIZE {
            return Err(BlobError::TooLarge(file_size, MAX_BLOB_SIZE));
        }
        let encrypted = std::fs::read(&path)?;
        let key = crate::persistence::dev_encryption_key().map_err(|_| BlobError::Corruption)?;
        let decrypted = zeroize::Zeroizing::new(
            crate::crypto::decrypt_data(&encrypted, &key).map_err(|_| BlobError::Corruption)?,
        );
        let computed = Self::sha256_hex(&decrypted);
        if computed != hash {
            return Err(BlobError::Corruption);
        }
        Ok(decrypted.to_vec())
    }

    pub fn delete_blob(&mut self, hash: &str) -> anyhow::Result<()> {
        let count = self
            .refcounts
            .get_mut(hash)
            .ok_or_else(|| anyhow::anyhow!("blob not found: {}", hash))?;

        if *count > 1 {
            *count -= 1;
            return Ok(());
        }
        let _ = std::fs::remove_file(self.blob_path(hash));
        self.metadata.remove(hash);
        self.refcounts.remove(hash);
        Ok(())
    }

    pub fn has_blob(&self, hash: &str) -> bool {
        self.blob_path(hash).exists()
    }

    pub fn get_mime(&self, hash: &str) -> Option<&str> {
        self.metadata.get(hash).map(|m| m.mime.as_str())
    }

    pub fn get_size(&self, hash: &str) -> Option<usize> {
        self.metadata.get(hash).map(|m| m.size)
    }

    fn sha256_hex(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}
