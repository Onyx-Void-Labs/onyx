// ─── Onyx Error ────────────────────────────────────────────────────
// Unified error enum for the entire workspace.
// ────────────────────────────────────────────────────────────────────

use thiserror::Error;

#[derive(Debug, Error)]
pub enum OnyxError {
    // ── Storage ──
    #[error("storage error: {0}")]
    Storage(String),

    // ── Serialization ──
    #[error("serialization error: {0}")]
    Serialization(String),

    // ── Editor ──
    #[error("editor error: {0}")]
    Editor(String),

    // ── Math rendering ──
    #[error("math render error: {0}")]
    Math(String),

    // ── CRDT ──
    #[error("crdt error: {0}")]
    Crdt(String),

    // ── Network (Phase 2) ──
    #[error("network error: {0}")]
    Network(String),

    // ── Identity ──
    #[error("identity error: {0}")]
    Identity(String),

    // ── Neural Index (Semantic Memory) ──
    #[error("neural index error: {0}")]
    NeuralIndex(String),

    // ── Vault (Encryption) ──
    #[error("vault error: {0}")]
    Vault(String),

    // ── Generic ──
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Convenience alias used throughout the codebase.
pub type OnyxResult<T> = Result<T, OnyxError>;
