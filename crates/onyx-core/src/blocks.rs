// ─── Onyx Core — Block Engine (Rich Content Blocks) ────────────────

use serde::{Deserialize, Serialize};

/// A single stroke in a Canvas block (ink/drawing data).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stroke {
    pub points: Vec<(f32, f32)>,
    pub color: String,
    pub width: f32,
}

/// The type of a content block within a Note.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum BlockType {
    Paragraph,
    Heading(u8), // Level 1-6
    BulletList,
    NumberedList,
    CodeBlock {
        language: String,
    },
    MathBlock, // KaTeX logic later
    Checklist {
        checked: bool,
    },
    Quote,
    Divider,
    Link {
        target_id: String,
    }, // References another Note/Void
    Canvas {
        strokes: Vec<Stroke>,
        width: f32,
        height: f32,
    },
    Math {
        latex: String,
        is_display: bool,
    },
    Embed {
        provider: String,
        url: String,
        meta: String,
    }, // JSON metadata
}

/// A single content block. Notes are composed of a list of Blocks.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Block {
    pub id: String,            // UUID
    pub kind: BlockType,       // What type of block
    pub content: String,       // The text inside
    pub children: Vec<String>, // IDs of nested blocks (indentation)
}

impl Block {
    /// Create a new empty Paragraph block.
    pub fn empty_paragraph() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind: BlockType::Paragraph,
            content: String::new(),
            children: Vec::new(),
        }
    }

    /// Create a new Heading block with the given level and text.
    pub fn new_heading(level: u8, text: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind: BlockType::Heading(level),
            content: text.to_string(),
            children: Vec::new(),
        }
    }
}
