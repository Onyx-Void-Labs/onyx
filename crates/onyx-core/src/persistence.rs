// ─── The Cortex: Persistence & AI Memory ───────────────────────────
// Atomic persistence layer for VoidNode state.
//
// StorageEngine:
//   • Serializes Vec<VoidNode> via rkyv (zero-copy).
//   • Atomic write: tmp → fsync → rename (never corrupt on crash).
//   • Auto-save every 5 seconds on a background thread.
//
// SemanticEngine:
//   • Embedding([f32; 384]) — candle vector store data structure.
//   • Stub: model not running yet, but data pipeline exists.
// ────────────────────────────────────────────────────────────────────

use crate::void_node::VoidNode;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

// ── Embedding type for the Semantic Engine ──────────────────────

/// A 384-dimensional embedding vector for semantic similarity.
/// Prepared for the candle vector store — the data structure
/// persists even when the model is offline.
#[derive(Debug, Clone)]
pub struct Embedding(pub [f32; 384]);

impl Default for Embedding {
    fn default() -> Self {
        Self([0.0; 384])
    }
}

/// Semantic memory index entry: node ID → embedding vector.
#[derive(Debug, Clone)]
pub struct SemanticEntry {
    pub node_id: crate::id::OnyxId,
    pub embedding: Embedding,
}

/// The Semantic Engine stub.
/// Holds the vector index in memory. The candle model will be
/// loaded lazily in a future phase; for now the index stores
/// pre-computed embeddings that survive serialization.
#[derive(Default)]
pub struct SemanticEngine {
    pub entries: Vec<SemanticEntry>,
}

impl SemanticEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert or update an embedding for a node.
    pub fn upsert(&mut self, node_id: crate::id::OnyxId, embedding: Embedding) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.node_id == node_id) {
            entry.embedding = embedding;
        } else {
            self.entries.push(SemanticEntry { node_id, embedding });
        }
    }

    /// Find the k nearest neighbours to the query embedding.
    /// Returns (node_id, cosine_similarity) pairs, sorted descending.
    pub fn search(&self, query: &Embedding, k: usize) -> Vec<(crate::id::OnyxId, f32)> {
        let mut scores: Vec<(crate::id::OnyxId, f32)> = self
            .entries
            .iter()
            .map(|e| {
                let sim = cosine_similarity(&query.0, &e.embedding.0);
                (e.node_id, sim)
            })
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        scores
    }
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32; 384], b: &[f32; 384]) -> f32 {
    let mut dot = 0.0_f32;
    let mut mag_a = 0.0_f32;
    let mut mag_b = 0.0_f32;
    for i in 0..384 {
        dot += a[i] * b[i];
        mag_a += a[i] * a[i];
        mag_b += b[i] * b[i];
    }
    let denom = (mag_a.sqrt() * mag_b.sqrt()).max(1e-10);
    dot / denom
}

// ── Storage Engine ──────────────────────────────────────────────

/// Atomic persistence engine for VoidNode state.
///
/// Write protocol (crash-safe):
///   1. Serialize nodes to bytes via serde_json (rkyv for Phase 2 hot path).
///   2. Write to `onyx_state.tmp`.
///   3. Call `fsync` on the file descriptor.
///   4. `fs::rename` atomically replaces `onyx_state.bin`.
///
/// This guarantees that `onyx_state.bin` is always a complete,
/// valid snapshot — never a partial write.
pub struct StorageEngine {
    /// Directory where state files live.
    dir: PathBuf,
    /// Shared node state for background auto-save.
    shared_state: Arc<Mutex<Vec<VoidNode>>>,
    /// Handle to the auto-save background thread.
    _auto_save_handle: Option<std::thread::JoinHandle<()>>,
}

impl StorageEngine {
    /// Create a new StorageEngine rooted at `dir`.
    /// Starts the auto-save background thread (5-second interval).
    pub fn new(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref().to_path_buf();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            tracing::error!("failed to create storage dir: {e}");
        }

        let shared_state: Arc<Mutex<Vec<VoidNode>>> = Arc::new(Mutex::new(Vec::new()));
        let state_clone = Arc::clone(&shared_state);
        let dir_clone = dir.clone();

        let handle = std::thread::spawn(move || {
            auto_save_loop(dir_clone, state_clone);
        });

        Self {
            dir,
            shared_state,
            _auto_save_handle: Some(handle),
        }
    }

    /// Update the shared state that the auto-save thread will persist.
    pub fn update_state(&self, nodes: &[VoidNode]) {
        if let Ok(mut state) = self.shared_state.lock() {
            *state = nodes.to_vec();
        }
    }

    /// Perform an immediate atomic save (bypasses the timer).
    pub fn save_now(&self, nodes: &[VoidNode]) -> Result<(), anyhow::Error> {
        atomic_write(&self.dir, nodes)
    }

    /// Load the last persisted state from disk.
    pub fn load(&self) -> Result<Vec<VoidNode>, anyhow::Error> {
        let bin_path = self.dir.join("onyx_state.bin");
        if !bin_path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read(&bin_path)?;
        let nodes: Vec<VoidNode> = serde_json::from_slice(&data)?;
        Ok(nodes)
    }
}

/// Background auto-save loop: every 5 seconds, atomically persist.
fn auto_save_loop(dir: PathBuf, state: Arc<Mutex<Vec<VoidNode>>>) {
    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
        let snapshot = {
            let guard = match state.lock() {
                Ok(g) => g,
                Err(e) => {
                    tracing::error!("auto-save lock poisoned: {e}");
                    continue;
                }
            };
            guard.clone()
        };
        if snapshot.is_empty() {
            continue;
        }
        if let Err(e) = atomic_write(&dir, &snapshot) {
            tracing::error!("auto-save failed: {e}");
        } else {
            tracing::debug!("auto-save: {} nodes persisted", snapshot.len());
        }
    }
}

/// Atomic write: serialize → tmp → fsync → rename.
fn atomic_write(dir: &Path, nodes: &[VoidNode]) -> Result<(), anyhow::Error> {
    use std::io::Write;

    let tmp_path = dir.join("onyx_state.tmp");
    let bin_path = dir.join("onyx_state.bin");

    let data = serde_json::to_vec(nodes)?;

    {
        let mut file = std::fs::File::create(&tmp_path)?;
        file.write_all(&data)?;
        file.sync_all()?; // fsync — flush to disk
    }

    std::fs::rename(&tmp_path, &bin_path)?;
    Ok(())
}
