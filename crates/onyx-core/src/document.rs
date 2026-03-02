// ─── Document Model ────────────────────────────────────────────────
// The core "page" or "note" abstraction. Everything the user creates
// is a Document. Serializable via rkyv for zero-copy storage.
// ────────────────────────────────────────────────────────────────────

use crate::id::OnyxId;
use rkyv::{Archive, Deserialize, Serialize};

/// A block inside a document—paragraph, heading, math, code, etc.
#[derive(Debug, Clone, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize)]
pub enum Block {
    /// Plain rich-text paragraph.
    Paragraph { text: String },
    /// LaTeX / Typst math block.
    Math { source: String },
    /// Fenced code block with optional language tag.
    Code { lang: String, source: String },
    /// Section heading (level 1-6).
    Heading { level: u8, text: String },
}

/// A document is an ordered list of blocks plus metadata.
#[derive(Debug, Clone, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize)]
pub struct Document {
    pub id: OnyxId,
    pub title: String,
    pub blocks: Vec<Block>,
    /// Unix-millis created timestamp.
    pub created_at: u64,
    /// Unix-millis last-modified timestamp.
    pub updated_at: u64,
}

impl Document {
    /// Create a new empty document.
    pub fn new(title: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            id: OnyxId::new(),
            title: title.into(),
            blocks: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Append a block and bump `updated_at`.
    pub fn push_block(&mut self, block: Block) {
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.blocks.push(block);
    }
}
