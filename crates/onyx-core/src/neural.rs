// ─── Onyx Core — Neural / Semantic Engine (Candle) ─────────────────
// Loads all-MiniLM-L6-v2 for text embedding and cosine-similarity search.
// ───────────────────────────────────────────────────────────────────

use anyhow::{Context, Result};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use tokenizers::Tokenizer;

use crate::document::OnyxWorkspace;

const MODEL_ID: &str = "sentence-transformers/all-MiniLM-L6-v2";

/// The Semantic Engine — wraps a BERT model for text embeddings.
pub struct SemanticEngine {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl SemanticEngine {
    /// Load the model and tokenizer.
    ///
    /// Tries HuggingFace Hub cache first, then downloads.
    /// This is a blocking operation — call from a background thread.
    pub fn load() -> Result<Self> {
        let device = Device::Cpu;

        let repo = hf_hub::api::sync::Api::new()?.model(MODEL_ID.to_string());

        let config_path = repo.get("config.json").context("downloading config.json")?;
        let tokenizer_path = repo
            .get("tokenizer.json")
            .context("downloading tokenizer.json")?;
        let weights_path = repo
            .get("model.safetensors")
            .context("downloading model.safetensors")?;

        let config_str = std::fs::read_to_string(&config_path)?;
        let config: BertConfig = serde_json::from_str(&config_str)?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow::anyhow!("tokenizer load error: {e}"))?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, &device)?
        };
        let model = BertModel::load(vb, &config)?;

        tracing::info!("🧠 SemanticEngine loaded ({MODEL_ID})");
        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    /// Embed a single text string into a dense vector.
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("tokenize error: {e}"))?;

        let ids = encoding.get_ids().to_vec();
        let type_ids = encoding.get_type_ids().to_vec();
        let len = ids.len();

        let input_ids = Tensor::new(ids, &self.device)?.unsqueeze(0)?;
        let type_ids = Tensor::new(type_ids, &self.device)?.unsqueeze(0)?;

        let output = self.model.forward(&input_ids, &type_ids, None)?;

        // Mean pooling over token dimension
        let sum = output.sum(1)?;
        let count = Tensor::new(vec![len as f32], &self.device)?
            .unsqueeze(0)?
            .broadcast_as(sum.shape())?;
        let mean = (sum / count)?;

        // L2 normalize
        let norm = mean
            .sqr()?
            .sum_keepdim(1)?
            .sqrt()?
            .broadcast_as(mean.shape())?;
        let normalized = (mean / norm)?;

        let vec: Vec<f32> = normalized.squeeze(0)?.to_vec1()?;
        Ok(vec)
    }

    /// Semantic search: embed query, compare against stored vectors, return top results.
    pub fn semantic_search(
        &self,
        query: &str,
        workspace: &OnyxWorkspace,
        limit: usize,
    ) -> Result<Vec<(String, f32)>> {
        let query_vec = self.embed_text(query)?;

        let note_ids = workspace.all_note_ids();
        let mut scored: Vec<(String, f32)> = Vec::new();

        for note_id in &note_ids {
            if let Some(stored_vec) = workspace.get_vector(note_id) {
                let sim = cosine_similarity(&query_vec, &stored_vec);
                scored.push((note_id.clone(), sim));
            }
        }

        // Sort descending by score
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
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
