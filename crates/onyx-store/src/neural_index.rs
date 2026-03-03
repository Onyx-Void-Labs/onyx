// ─── Neural Index ──────────────────────────────────────────────────
// Semantic Memory — local AI-powered search for Onyx documents.
//
// Uses the all-MiniLM-L6-v2 sentence-transformer (quantized, ~20MB)
// to generate 384-dimensional embeddings for every text block.
// Search for "money" and find your note about "budget" — offline.
//
// Architecture:
//   1. On first launch, download the model from HuggingFace Hub
//      into the hf-hub cache directory.
//   2. NeuralIndex holds the model + tokenizer + embeddings map.
//   3. index_block(id, text) computes + stores an embedding.
//   4. search_semantic(query, top_k) finds the most similar blocks.
//
// Gated behind the `neural` feature flag.
// ────────────────────────────────────────────────────────────────────

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use candle_core::{Device, Tensor, DType};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use hf_hub::api::sync::Api;
use tokenizers::Tokenizer;
use tracing::info;

/// Dimensionality of all-MiniLM-L6-v2 embeddings.
const EMBED_DIM: usize = 384;

/// HuggingFace model repository ID.
const MODEL_REPO: &str = "sentence-transformers/all-MiniLM-L6-v2";

/// A search result with document ID and similarity score.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub doc_id: String,
    pub score: f32,
    pub snippet: String,
}

/// The Neural Index: generates and stores embeddings, provides semantic search.
pub struct NeuralIndex {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    /// Stored embeddings: doc_id → (text_snippet, embedding_vector).
    embeddings: Arc<Mutex<HashMap<String, (String, Vec<f32>)>>>,
}

impl NeuralIndex {
    /// Initialize the neural index.  Downloads the model on first launch.
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let device = Device::Cpu;

        // Download (or use cached) model files from HuggingFace Hub.
        let (config_path, tokenizer_path, weights_path) = Self::ensure_model()?;

        info!("loading neural index model (all-MiniLM-L6-v2)");

        // Load tokenizer.
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| format!("failed to load tokenizer: {e}"))?;

        // Load model config.
        let config_str = std::fs::read_to_string(&config_path)?;
        let config: BertConfig = serde_json::from_str(&config_str)?;

        // Load model weights.
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(
                &[weights_path],
                DType::F32,
                &device,
            )?
        };
        let model = BertModel::load(vb, &config)?;

        info!("neural index ready ({}D embeddings, CPU)", EMBED_DIM);

        Ok(Self {
            model,
            tokenizer,
            device,
            embeddings: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Ensure the model files are downloaded.  Uses hf-hub's built-in
    /// caching so the ~20MB download only happens once.
    fn ensure_model() -> Result<(PathBuf, PathBuf, PathBuf), Box<dyn std::error::Error + Send + Sync>> {
        let api = Api::new()?;
        let repo = api.model(MODEL_REPO.to_string());

        info!("checking/downloading neural index model from HuggingFace");

        let config = repo.get("config.json")?;
        let tokenizer = repo.get("tokenizer.json")?;
        let weights = repo.get("model.safetensors")?;

        info!(
            config = %config.display(),
            tokenizer = %tokenizer.display(),
            weights = %weights.display(),
            "model files ready"
        );

        Ok((config, tokenizer, weights))
    }

    /// Generate an embedding vector for a text string.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| format!("tokenization failed: {e}"))?;

        let ids: Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
        let mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&x| x as i64).collect();
        let seq_len = ids.len();

        let token_ids = Tensor::new(vec![ids], &self.device)?;
        let attention = Tensor::new(vec![mask], &self.device)?;
        let token_type_ids = Tensor::zeros(&[1, seq_len], DType::I64, &self.device)?;

        // Forward pass through BERT.
        let output = self.model.forward(&token_ids, &token_type_ids, Some(&attention))?;

        // Mean pooling over the sequence dimension, masked by attention.
        let attention_f32 = attention.to_dtype(DType::F32)?;     // [1, seq_len]
        let attention_3d = attention_f32.unsqueeze(2)?;           // [1, seq_len, 1]
        let masked = output.broadcast_mul(&attention_3d)?;        // [1, seq_len, hidden]
        let summed = masked.sum(1)?;                              // [1, hidden]
        let mask_sum = attention_3d.sum(1)?;                      // [1, 1]
        let pooled = summed.broadcast_div(&mask_sum)?;            // [1, hidden]

        // L2 normalize.
        let norm = pooled.sqr()?.sum(1)?.sqrt()?.unsqueeze(1)?;  // [1, 1]
        let normalized = pooled.broadcast_div(&norm)?;            // [1, hidden]

        let embedding: Vec<f32> = normalized.squeeze(0)?.to_vec1()?;
        Ok(embedding)
    }

    /// Index a document block.  Computes its embedding and stores it.
    pub fn index_block(
        &self,
        doc_id: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if text.trim().is_empty() {
            return Ok(());
        }

        let embedding = self.embed(text)?;

        let snippet = if text.len() > 200 {
            format!("{}...", &text[..200])
        } else {
            text.to_string()
        };

        if let Ok(mut map) = self.embeddings.lock() {
            map.insert(doc_id.to_string(), (snippet, embedding));
        }

        Ok(())
    }

    /// Remove a document block from the index.
    pub fn remove_block(&self, doc_id: &str) {
        if let Ok(mut map) = self.embeddings.lock() {
            map.remove(doc_id);
        }
    }

    /// Semantic search: find documents similar to the query.
    /// Returns results sorted by cosine similarity (highest first).
    pub fn search_semantic(
        &self,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
        let query_embedding = self.embed(query)?;

        let map = self
            .embeddings
            .lock()
            .map_err(|_| "embedding lock poisoned")?;

        let mut results: Vec<SearchResult> = map
            .iter()
            .map(|(doc_id, (snippet, doc_embedding))| {
                let score = cosine_similarity(&query_embedding, doc_embedding);
                SearchResult {
                    doc_id: doc_id.clone(),
                    score,
                    snippet: snippet.clone(),
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);

        Ok(results)
    }

    /// Get a thread-safe handle to the embeddings store.
    pub fn embeddings_handle(&self) -> Arc<Mutex<HashMap<String, (String, Vec<f32>)>>> {
        Arc::clone(&self.embeddings)
    }

    /// Number of indexed blocks.
    pub fn index_size(&self) -> usize {
        self.embeddings.lock().map(|m| m.len()).unwrap_or(0)
    }
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
