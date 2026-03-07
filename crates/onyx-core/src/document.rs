// ─── Document Model ────────────────────────────────────────────────
// The core "page" or "note" abstraction. Everything the user creates
// is a Document. Serializable via rkyv for zero-copy storage.
// ────────────────────────────────────────────────────────────────────

use crate::id::OnyxId;
use rkyv::{Archive, Deserialize, Serialize};

/// The content variant of a block.
#[derive(Debug, Clone, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize)]
pub enum BlockContent {
    /// Plain rich-text paragraph.
    Paragraph(String),
    /// Section heading (level 1-6).
    Heading(String, u8),
    /// LaTeX / Typst math block.
    Math(String),
    /// Fenced code block with optional language tag.
    Code { lang: String, source: String },
}

/// A block inside a document—paragraph, heading, math, code, etc.
#[derive(Debug, Clone, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize)]
pub struct Block {
    pub id: String,
    pub content: BlockContent,
}

impl Block {
    /// Create a new paragraph block with a generated UUID.
    pub fn paragraph(text: impl Into<String>) -> Self {
        Self {
            id: OnyxId::new().to_string(),
            content: BlockContent::Paragraph(text.into()),
        }
    }

    /// Create a new heading block with a generated UUID.
    pub fn heading(text: impl Into<String>, level: u8) -> Self {
        Self {
            id: OnyxId::new().to_string(),
            content: BlockContent::Heading(text.into(), level),
        }
    }

    /// Return a reference to this block's text content.
    pub fn text(&self) -> &str {
        match &self.content {
            BlockContent::Paragraph(t) | BlockContent::Heading(t, _) | BlockContent::Math(t) => t,
            BlockContent::Code { source, .. } => source,
        }
    }

    /// Return a mutable reference to this block's text content.
    pub fn text_mut(&mut self) -> &mut String {
        match &mut self.content {
            BlockContent::Paragraph(t) | BlockContent::Heading(t, _) | BlockContent::Math(t) => t,
            BlockContent::Code { source, .. } => source,
        }
    }
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
