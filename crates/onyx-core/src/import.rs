// ─── Onyx Core — Markdown Import / Export Bridge ───────────────────
// Converts between Markdown text and Onyx Block structures.
// ───────────────────────────────────────────────────────────────────

use crate::blocks::{Block, BlockType};

/// Parse a Markdown string into a Vec<Block>.
pub fn parse_markdown(md: &str) -> Vec<Block> {
    let mut blocks = Vec::new();

    for line in md.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let block = if let Some(heading) = parse_heading(trimmed) {
            heading
        } else if let Some(stripped) = trimmed.strip_prefix("- [x] ") {
            Block {
                id: uuid::Uuid::new_v4().to_string(),
                kind: BlockType::Checklist { checked: true },
                content: stripped.to_string(),
                attributes: Vec::new(),
                children: Vec::new(),
            }
        } else if let Some(stripped) = trimmed.strip_prefix("- [ ] ") {
            Block {
                id: uuid::Uuid::new_v4().to_string(),
                kind: BlockType::Checklist { checked: false },
                content: stripped.to_string(),
                attributes: Vec::new(),
                children: Vec::new(),
            }
        } else if let Some(stripped) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            Block {
                id: uuid::Uuid::new_v4().to_string(),
                kind: BlockType::BulletList,
                content: stripped.to_string(),
                attributes: Vec::new(),
                children: Vec::new(),
            }
        } else if let Some(stripped) = parse_numbered_list(trimmed) {
            Block {
                id: uuid::Uuid::new_v4().to_string(),
                kind: BlockType::NumberedList,
                content: stripped,
                attributes: Vec::new(),
                children: Vec::new(),
            }
        } else if let Some(stripped) = trimmed.strip_prefix("> ") {
            Block {
                id: uuid::Uuid::new_v4().to_string(),
                kind: BlockType::Quote,
                content: stripped.to_string(),
                attributes: Vec::new(),
                children: Vec::new(),
            }
        } else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            Block {
                id: uuid::Uuid::new_v4().to_string(),
                kind: BlockType::Divider,
                content: String::new(),
                attributes: Vec::new(),
                children: Vec::new(),
            }
        } else {
            Block {
                id: uuid::Uuid::new_v4().to_string(),
                kind: BlockType::Paragraph,
                content: trimmed.to_string(),
                attributes: Vec::new(),
                children: Vec::new(),
            }
        };

        blocks.push(block);
    }

    // Handle code blocks (fenced)
    blocks = merge_code_blocks(blocks, md);

    blocks
}

/// Parse heading lines (# through ######).
fn parse_heading(line: &str) -> Option<Block> {
    let level = line.chars().take_while(|c| *c == '#').count();
    if (1..=6).contains(&level) {
        if let Some(stripped) = line.get(level..).and_then(|s| s.strip_prefix(' ')) {
            return Some(Block {
                id: uuid::Uuid::new_v4().to_string(),
                kind: BlockType::Heading(level as u8),
                content: stripped.to_string(),
                attributes: Vec::new(),
                children: Vec::new(),
            });
        }
    }
    None
}

/// Parse numbered list items like "1. text".
fn parse_numbered_list(line: &str) -> Option<String> {
    let dot_pos = line.find(". ")?;
    let prefix = &line[..dot_pos];
    if prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty() {
        Some(line[dot_pos + 2..].to_string())
    } else {
        None
    }
}

/// Re-parse the source to detect fenced code blocks (```lang ... ```)
/// and replace the inline-parsed blocks with proper CodeBlock entries.
fn merge_code_blocks(mut blocks: Vec<Block>, md: &str) -> Vec<Block> {
    let lines: Vec<&str> = md.lines().collect();
    let mut result = Vec::new();
    let mut block_iter = blocks.drain(..);
    let mut skip_until_fence_close = false;
    let mut code_content = String::new();
    let mut code_lang = String::new();

    for line in &lines {
        let trimmed = line.trim();

        if skip_until_fence_close {
            if trimmed.starts_with("```") {
                // End of code block
                skip_until_fence_close = false;
                result.push(Block {
                    id: uuid::Uuid::new_v4().to_string(),
                    kind: BlockType::CodeBlock {
                        language: code_lang.clone(),
                    },
                    content: code_content.trim_end().to_string(),
                    attributes: Vec::new(),
                    children: Vec::new(),
                });
                code_content.clear();
                code_lang.clear();
            } else {
                code_content.push_str(line);
                code_content.push('\n');
            }
            continue;
        }

        if trimmed.starts_with("```") {
            // Start of code block
            skip_until_fence_close = true;
            code_lang = trimmed.trim_start_matches('`').to_string();
            code_content.clear();
            continue;
        }

        // Consume from block_iter for non-code lines
        if !trimmed.is_empty() {
            if let Some(block) = block_iter.next() {
                result.push(block);
            }
        }
    }

    result
}

/// Convert a Vec<Block> back into a Markdown string.
pub fn blocks_to_markdown(blocks: &[Block]) -> String {
    let mut lines = Vec::new();

    for block in blocks {
        match &block.kind {
            BlockType::Heading(level) => {
                let hashes = "#".repeat(*level as usize);
                lines.push(format!("{} {}", hashes, block.content));
            }
            BlockType::Paragraph => {
                lines.push(block.content.clone());
            }
            BlockType::BulletList => {
                lines.push(format!("- {}", block.content));
            }
            BlockType::NumberedList => {
                lines.push(format!("1. {}", block.content));
            }
            BlockType::Checklist { checked } => {
                let marker = if *checked { "[x]" } else { "[ ]" };
                lines.push(format!("- {} {}", marker, block.content));
            }
            BlockType::CodeBlock { language } => {
                lines.push(format!("```{}", language));
                lines.push(block.content.clone());
                lines.push("```".to_string());
            }
            BlockType::MathBlock => {
                lines.push("$$".to_string());
                lines.push(block.content.clone());
                lines.push("$$".to_string());
            }
            BlockType::Quote => {
                lines.push(format!("> {}", block.content));
            }
            BlockType::Divider => {
                lines.push("---".to_string());
            }
            BlockType::Link { target_id } => {
                lines.push(format!("[{}](onyx://{})", block.content, target_id));
            }
            BlockType::Canvas { .. } => {
                lines.push("[Canvas block]".to_string());
            }
            BlockType::Math { latex, is_display } => {
                if *is_display {
                    lines.push("$$".to_string());
                    lines.push(latex.clone());
                    lines.push("$$".to_string());
                } else {
                    lines.push(format!("${}$", latex));
                }
            }
            BlockType::Embed { provider, url, .. } => {
                lines.push(format!("[{} embed]({})", provider, url));
            }
        }
        lines.push(String::new()); // blank line between blocks
    }

    // Remove trailing blank line
    while lines.last().map(|l| l.is_empty()).unwrap_or(false) {
        lines.pop();
    }

    lines.join("\n")
}

/// Import a markdown file and return (title, Vec<Block>).
/// Title is extracted from the first H1 heading, or defaults to the filename.
pub fn import_markdown_text(md: &str, fallback_title: &str) -> (String, Vec<Block>) {
    let blocks = parse_markdown(md);

    // Extract title from first H1
    let title = blocks
        .iter()
        .find_map(|b| match &b.kind {
            BlockType::Heading(1) => Some(b.content.clone()),
            _ => None,
        })
        .unwrap_or_else(|| fallback_title.to_string());

    (title, blocks)
}

/// Export blocks to a Markdown string.
pub fn export_markdown(blocks: &[Block]) -> String {
    blocks_to_markdown(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_roundtrip() {
        let md =
            "# Hello World\n\nThis is a paragraph.\n\n- Item one\n- Item two\n\n> A quote\n\n---\n";
        let blocks = parse_markdown(md);

        assert!(blocks.len() >= 5);
        assert!(matches!(blocks[0].kind, BlockType::Heading(1)));
        assert_eq!(blocks[0].content, "Hello World");
        assert!(matches!(blocks[1].kind, BlockType::Paragraph));
        assert!(matches!(blocks[2].kind, BlockType::BulletList));
        assert!(matches!(blocks[3].kind, BlockType::BulletList));
        assert!(matches!(blocks[4].kind, BlockType::Quote));
    }

    #[test]
    fn code_block_parsing() {
        let md = "```rust\nfn main() {}\n```\n";
        let blocks = parse_markdown(md);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0].kind, BlockType::CodeBlock { .. }));
        assert_eq!(blocks[0].content, "fn main() {}");
    }

    #[test]
    fn export_markdown_roundtrip() {
        let blocks = vec![
            Block {
                id: "1".into(),
                kind: BlockType::Heading(1),
                content: "Title".into(),
                attributes: Vec::new(),
                children: Vec::new(),
            },
            Block {
                id: "2".into(),
                kind: BlockType::Paragraph,
                content: "Body text.".into(),
                attributes: Vec::new(),
                children: Vec::new(),
            },
        ];
        let md = export_markdown(&blocks);
        assert!(md.contains("# Title"));
        assert!(md.contains("Body text."));
    }
}
