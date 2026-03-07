// ─── Onyx Void — Markdown Style Applier ────────────────────────────
// Detects Markdown syntax in plain text and applies Parley styles
// (font size, weight, italic, color) to a RangedBuilder.
// ────────────────────────────────────────────────────────────────────

use parley::style::{FontWeight, StyleProperty};
use parley::RangedBuilder;
use vello::peniko;

/// Header 1 color — pure white.
const H1_COLOR: peniko::Color = peniko::Color::from_rgba8(0xFF, 0xFF, 0xFF, 0xFF);
/// Header 2 color — zinc-200.
const H2_COLOR: peniko::Color = peniko::Color::from_rgba8(0xE4, 0xE4, 0xE7, 0xFF);
/// Quote color — zinc-400.
const QUOTE_COLOR: peniko::Color = peniko::Color::from_rgba8(0xA1, 0xA1, 0xAA, 0xFF);

/// Applies Markdown syntax highlighting styles to a Parley `RangedBuilder`.
pub struct MarkdownStyler;

impl MarkdownStyler {
    /// Scan `text` for Markdown patterns and push ranged styles onto `builder`.
    ///
    /// Detected patterns (per-line):
    ///   - `# Header`   → size 24, bold, white
    ///   - `## Sub`      → size 20, bold, zinc-200
    ///   - `> Quote`     → italic, zinc-400
    ///
    /// Detected inline (across document):
    ///   - `**bold**`    → bold
    ///   - `*italic*`    → italic
    pub fn apply_styles(text: &str, builder: &mut RangedBuilder<peniko::Brush>) {
        // ── Line-level patterns ──
        let mut offset = 0usize;
        for line in text.split('\n') {
            let line_len = line.len();
            let trimmed = line.trim_start();

            if trimmed.starts_with("## ") {
                // H2
                builder.push(StyleProperty::FontSize(20.0), offset..offset + line_len);
                builder.push(
                    StyleProperty::FontWeight(FontWeight::BOLD),
                    offset..offset + line_len,
                );
                builder.push(
                    StyleProperty::Brush(H2_COLOR.into()),
                    offset..offset + line_len,
                );
            } else if trimmed.starts_with("# ") {
                // H1
                builder.push(StyleProperty::FontSize(24.0), offset..offset + line_len);
                builder.push(
                    StyleProperty::FontWeight(FontWeight::BOLD),
                    offset..offset + line_len,
                );
                builder.push(
                    StyleProperty::Brush(H1_COLOR.into()),
                    offset..offset + line_len,
                );
            } else if trimmed.starts_with("> ") {
                // Quote
                builder.push(
                    StyleProperty::FontStyle(parley::style::FontStyle::Italic),
                    offset..offset + line_len,
                );
                builder.push(
                    StyleProperty::Brush(QUOTE_COLOR.into()),
                    offset..offset + line_len,
                );
            }

            // +1 for the '\n' separator (if not at end)
            offset += line_len + 1;
        }

        // ── Inline patterns ──
        Self::apply_inline_bold(text, builder);
        Self::apply_inline_italic(text, builder);
    }

    /// Find `**bold**` spans and apply bold weight.
    fn apply_inline_bold(text: &str, builder: &mut RangedBuilder<peniko::Brush>) {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i + 4 < bytes.len() {
            if bytes[i] == b'*' && bytes[i + 1] == b'*' {
                // Find closing **
                if let Some(rel) = text[i + 2..].find("**") {
                    let start = i + 2;
                    let end = i + 2 + rel;
                    if end > start {
                        builder.push(StyleProperty::FontWeight(FontWeight::BOLD), start..end);
                    }
                    i = end + 2;
                    continue;
                }
            }
            i += 1;
        }
    }

    /// Find `*italic*` spans (but not `**`) and apply italic style.
    fn apply_inline_italic(text: &str, builder: &mut RangedBuilder<peniko::Brush>) {
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            // Single * but not **
            if bytes[i] == b'*'
                && (i + 1 >= bytes.len() || bytes[i + 1] != b'*')
                && (i == 0 || bytes[i - 1] != b'*')
            {
                // Find closing single *
                let search_start = i + 1;
                let mut j = search_start;
                while j < bytes.len() {
                    if bytes[j] == b'*'
                        && (j + 1 >= bytes.len() || bytes[j + 1] != b'*')
                        && bytes[j - 1] != b'*'
                    {
                        let start = search_start;
                        let end = j;
                        if end > start {
                            builder.push(
                                StyleProperty::FontStyle(parley::style::FontStyle::Italic),
                                start..end,
                            );
                        }
                        i = j + 1;
                        break;
                    }
                    j += 1;
                }
                if j >= bytes.len() {
                    break;
                }
            } else {
                i += 1;
            }
        }
    }
}
