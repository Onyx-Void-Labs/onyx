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
///
/// Content text is stored separately from styling/metadata via an
/// array of `AttributeSpan` entries.  This allows multiple overlapping
/// attributes (bold + sentiment + timestamp etc.) to coexist without
/// corrupting the character buffer and enables fine‑grained CRDT
/// convergence on each attribute independently.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Block {
    pub id: String,      // UUID
    pub kind: BlockType, // What type of block
    #[serde(default = "default_align")]
    pub align: String,
    #[serde(default)]
    pub indent_level: u8,
    /// Raw Unicode string.  Attributes refer to byte indices within this
    /// string (start inclusive, end exclusive).
    pub content: String,
    pub attributes: Vec<AttributeSpan>,
    pub children: Vec<String>, // IDs of nested blocks (indentation)
}

/// A span of characters carrying a non‑visual attribute.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeSpan {
    pub start: usize,
    pub end: usize,
    pub attr: Attribute,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Attribute {
    // Standard visual styling
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Color([f32; 4]),
    Highlight([f32; 4]),
    FontFamily(String),
    FontSize(f32),
    Superscript,
    Subscript,

    // ONYX advantages
    Sentiment(f32),                             // AI‑inferred emotion score
    ClozeGap { card_id: String, hidden: bool }, // FSRS-aware cloze
    VoiceSync { timestamp_ms: u64 },            // Whisper.cpp transcription sync
    LaTeX { expression: String },               // mathematical derivation
}


fn default_align() -> String {
    "left".to_string()
}

impl Block {
    /// Create a new empty Paragraph block.
    pub fn empty_paragraph() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind: BlockType::Paragraph,
            align: default_align(),
            indent_level: 0,
            content: String::new(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Create a new Heading block with the given level and text.
    pub fn new_heading(level: u8, text: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            kind: BlockType::Heading(level),
            align: default_align(),
            indent_level: 0,
            content: text.to_string(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }
}
