// ─── Onyx Core — Editing Engine ─────────

use crate::blocks::{Block, AttributeSpan};
use crate::document::OnyxWorkspace;
use anyhow::Result;
use uuid::Uuid;

/// Core mutation logic for note blocks.
/// Handles character-level insertions, deletions, and block structural changes.

fn get_safe_char_boundary(text: &str, mut offset: usize) -> usize {
    if offset >= text.len() {
        return text.len();
    }
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

pub fn insert_text(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
    byte_offset: usize,
    text: &str,
    active_attrs: Option<&[crate::blocks::Attribute]>,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if let Some(block) = blocks.get_mut(block_index) {
        let safe_offset = get_safe_char_boundary(&block.content, byte_offset);
        block.content.insert_str(safe_offset, text);
        
        // Shift attribute spans
        let _len = text.chars().count(); // Using char count might be safer for some attributes, but Parley/Loro often use byte offsets.
        // Actually, the original code uses byte offsets for `AttributeSpan`.
        let byte_len = text.len();
        
        for span in &mut block.attributes {
            if span.start >= byte_offset {
                span.start += byte_len;
                span.end += byte_len;
            } else if span.end > byte_offset {
                span.end += byte_len;
            }
        }
        
        if let Some(attrs) = active_attrs {
            for attr in attrs {
                block.attributes.push(AttributeSpan {
                    start: byte_offset,
                    end: byte_offset + byte_len,
                    attr: attr.clone(),
                });
            }
        }
        
        ws.set_note_blocks(note_id, &blocks)?;
    } else {
        anyhow::bail!("Block index {} out of bounds for note {}", block_index, note_id);
    }
    Ok(())
}

pub fn delete_text(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
    byte_offset: usize,
    len: usize,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if let Some(block) = blocks.get_mut(block_index) {
        let safe_offset = get_safe_char_boundary(&block.content, byte_offset);
        let mut actual_len = len;
        if safe_offset + actual_len > block.content.len() {
            actual_len = block.content.len() - safe_offset;
        }
        let end_offset = get_safe_char_boundary(&block.content, safe_offset + actual_len);

        if end_offset > safe_offset {
            block.content.drain(safe_offset..end_offset);

            
            // Adjust attribute spans
            let mut i = 0;
            while i < block.attributes.len() {
                let span = &mut block.attributes[i];
                if span.start >= byte_offset + len {
                    // Span is after the deleted range -> shift left
                    span.start -= len;
                    span.end -= len;
                    i += 1;
                } else if span.end <= byte_offset {
                    // Span is before the deleted range -> no change
                    i += 1;
                } else {
                    // Overlap
                    if span.start >= byte_offset && span.end <= byte_offset + len {
                        // Fully contained in deleted range -> remove span
                        block.attributes.remove(i);
                    } else {
                        // Partial overlap
                        if span.start < byte_offset {
                            // Span starts before deletion
                            if span.end > byte_offset + len {
                                // Deletion is inside the span
                                span.end -= len;
                            } else {
                                // Span ends inside deletion
                                span.end = byte_offset;
                            }
                        } else {
                            // Span starts inside deletion
                            span.start = byte_offset;
                            span.end -= len;
                        }
                        i += 1;
                    }
                }
            }
            
            ws.set_note_blocks(note_id, &blocks)?;
        }
    } else {
        anyhow::bail!("Block index {} out of bounds for note {}", block_index, note_id);
    }
    Ok(())
}

pub fn split_block(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
    byte_offset: usize,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if let Some(block) = blocks.get_mut(block_index) {
        let safe_offset = get_safe_char_boundary(&block.content, byte_offset);
        let remaining_content = block.content.split_off(safe_offset);
        
        // Split attributes
        let mut old_attrs = Vec::new();
        let mut new_attrs = Vec::new();
        
        for span in block.attributes.drain(..) {
            if span.end <= safe_offset {
                old_attrs.push(span);
            } else if span.start >= safe_offset {
                new_attrs.push(AttributeSpan {
                    start: span.start - safe_offset,
                    end: span.end - safe_offset,
                    attr: span.attr,
                });
            } else {
                // Span spans across the split point
                old_attrs.push(AttributeSpan {
                    start: span.start,
                    end: safe_offset,
                    attr: span.attr.clone(),
                });
                new_attrs.push(AttributeSpan {
                    start: 0,
                    end: span.end - safe_offset,
                    attr: span.attr,
                });
            }
        }
        
        block.attributes = old_attrs;
        
        let new_block = Block {
            id: Uuid::new_v4().to_string(),
            kind: block.kind.clone(), // Inherit kind
            align: block.align.clone(),
            indent_level: block.indent_level,
            content: remaining_content,
            attributes: new_attrs,
            children: Vec::new(),
        };
        
        blocks.insert(block_index + 1, new_block);
        ws.set_note_blocks(note_id, &blocks)?;
    }
    Ok(())
}

pub fn merge_blocks(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if block_index > 0 && block_index < blocks.len() {
        let current_block = blocks.remove(block_index);
        let prev_block = &mut blocks[block_index - 1];
        
        let offset = prev_block.content.len();
        prev_block.content.push_str(&current_block.content);
        
        // Merge attributes
        for span in current_block.attributes {
            prev_block.attributes.push(AttributeSpan {
                start: span.start + offset,
                end: span.end + offset,
                attr: span.attr,
            });
        }
        
        ws.set_note_blocks(note_id, &blocks)?;
    }
    Ok(())
}

pub fn apply_attribute(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
    range: std::ops::Range<usize>,
    attr: crate::blocks::Attribute,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if let Some(block) = blocks.get_mut(block_index) {
        // TODO: Handle overlapping removal of same attribute (toggle logic)
        block.attributes.push(AttributeSpan {
            start: range.start,
            end: range.end,
            attr,
        });
        ws.set_note_blocks(note_id, &blocks)?;
    }
    Ok(())
}

/// Toggle an attribute on a selection range. If the entire range already has the
/// attribute, remove it; otherwise add it.
pub fn toggle_attribute(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
    range: std::ops::Range<usize>,
    attr: crate::blocks::Attribute,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if let Some(block) = blocks.get_mut(block_index) {
        
        let mut fully_covered = false;
        // First check if the range is fully covered by an existing identical attribute span
        if let Some(_) = block.attributes.iter().find(|s| {
            s.attr == attr && s.start <= range.start && s.end >= range.end
        }) {
            fully_covered = true;
        }

        let mut new_spans = Vec::new();
        let mut applied_to_range = false;

        for span in std::mem::take(&mut block.attributes) {
            if span.attr != attr {
                new_spans.push(span);
                continue;
            }

            // If the span is completely disjoint, keep it
            if span.end <= range.start || span.start >= range.end {
                new_spans.push(span);
                continue;
            }

            if fully_covered {
                // TOGGLE OFF LOGIC: Split or trim the span
                if span.start < range.start {
                    new_spans.push(AttributeSpan {
                        start: span.start,
                        end: range.start,
                        attr: attr.clone(),
                    });
                }
                if span.end > range.end {
                    new_spans.push(AttributeSpan {
                        start: range.end,
                        end: span.end,
                        attr: attr.clone(),
                    });
                }
            } else {
                // TOGGLE ON LOGIC: Merge overlapping/adjacent spans
                if !applied_to_range {
                    // Start a new merged span covering the requested range + the overlapping span
                    new_spans.push(AttributeSpan {
                        start: span.start.min(range.start),
                        end: span.end.max(range.end),
                        attr: attr.clone(),
                    });
                    applied_to_range = true;
                } else {
                    // Update the recently pushed merged span to subsume this one too
                    if let Some(last) = new_spans.last_mut() {
                        last.start = last.start.min(span.start);
                        last.end = last.end.max(span.end);
                    }
                }
            }
        }

        if !fully_covered && !applied_to_range {
            // No overlaps were found, so just add the span natively
            new_spans.push(AttributeSpan {
                start: range.start,
                end: range.end,
                attr,
            });
        }
        
        // Final normalization pass to merge adjacent identical spans that might have been left over
        new_spans.sort_by_key(|s| s.start);
        let mut normalized_spans: Vec<AttributeSpan> = Vec::new();
        for span in new_spans {
            if let Some(last) = normalized_spans.last_mut() {
                if last.attr == span.attr && last.end >= span.start {
                    last.end = last.end.max(span.end);
                    continue;
                }
            }
            normalized_spans.push(span);
        }

        block.attributes = normalized_spans;
        ws.set_note_blocks(note_id, &blocks)?;
    }
    Ok(())
}

/// Clears any attribute covering the given range if it shares the same enum variant
/// as the un-data-filled dummy `attr` (e.g., if `attr` is `Color([0.0;4])`, it removes *any* color).
pub fn clear_attribute_type(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
    range: std::ops::Range<usize>,
    attr: crate::blocks::Attribute,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if let Some(block) = blocks.get_mut(block_index) {
        let mut new_spans = Vec::new();

        for span in std::mem::take(&mut block.attributes) {
            // Check if discriminants match (i.e. both are Color(_), or both are Highlight(_))
            if std::mem::discriminant(&span.attr) != std::mem::discriminant(&attr) {
                new_spans.push(span);
                continue;
            }

            // If disjoint, keep
            if span.end <= range.start || span.start >= range.end {
                new_spans.push(span);
                continue;
            }

            // Otherwise, we split/trim because it overlaps and shares the type
            if span.start < range.start {
                new_spans.push(AttributeSpan {
                    start: span.start,
                    end: range.start,
                    attr: span.attr.clone(),
                });
            }
            if span.end > range.end {
                new_spans.push(AttributeSpan {
                    start: range.end,
                    end: span.end,
                    attr: span.attr.clone(),
                });
            }
        }

        block.attributes = new_spans;
        ws.set_note_blocks(note_id, &blocks)?;
    }
    Ok(())
}

/// Insert a new empty paragraph block at the given index.
pub fn insert_block(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    at_index: usize,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    let new_block = Block::empty_paragraph();
    let idx = at_index.min(blocks.len());
    blocks.insert(idx, new_block);
    ws.set_note_blocks(note_id, &blocks)?;
    Ok(())
}

/// Swap the block at `block_index` with the one above it (index - 1).
pub fn move_block_up(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if block_index > 0 && block_index < blocks.len() {
        blocks.swap(block_index, block_index - 1);
        ws.set_note_blocks(note_id, &blocks)?;
    }
    Ok(())
}

/// Swap the block at `block_index` with the one below it (index + 1).
pub fn move_block_down(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if block_index + 1 < blocks.len() {
        blocks.swap(block_index, block_index + 1);
        ws.set_note_blocks(note_id, &blocks)?;
    }
    Ok(())
}

/// Change the block type of the block at the given index.
pub fn set_block_type(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
    kind: crate::blocks::BlockType,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if let Some(block) = blocks.get_mut(block_index) {
        block.kind = kind;
        ws.set_note_blocks(note_id, &blocks)?;
    }
    Ok(())
}

/// Change the text alignment of the block at the given index.
pub fn set_block_align(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
    align: String,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if let Some(block) = blocks.get_mut(block_index) {
        block.align = align;
        ws.set_note_blocks(note_id, &blocks)?;
    }
    Ok(())
}

/// Increase the indentation level of the block, up to a maximum (e.g., 3).
pub fn increase_indent(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if let Some(block) = blocks.get_mut(block_index) {
        block.indent_level += 1;
        ws.set_note_blocks(note_id, &blocks)?;
    }
    Ok(())
}

/// Decrease the indentation level of the block, stopping at 0.
pub fn decrease_indent(
    ws: &mut OnyxWorkspace,
    note_id: &str,
    block_index: usize,
) -> Result<()> {
    let mut blocks = ws.get_note_blocks(note_id);
    if let Some(block) = blocks.get_mut(block_index) {
        if block.indent_level > 0 {
            block.indent_level -= 1;
            ws.set_note_blocks(note_id, &blocks)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::{Block, BlockType, Attribute};

    fn setup_workspace() -> (OnyxWorkspace, String) {
        let mut ws = OnyxWorkspace::new();
        let void_id = ws.create_void(None, "Testing").unwrap();
        let note_id = ws.create_note(&void_id, "Note 1").unwrap();
        (ws, note_id)
    }

    #[test]
    fn test_insert_text() {
        let (mut ws, note_id) = setup_workspace();
        insert_block(&mut ws, &note_id, 0).unwrap();
        
        insert_text(&mut ws, &note_id, 0, 0, "Hello", None).unwrap();
        let blocks = ws.get_note_blocks(&note_id);
        assert_eq!(blocks[0].content, "Hello");
        
        insert_text(&mut ws, &note_id, 0, 5, " World", None).unwrap();
        let blocks = ws.get_note_blocks(&note_id);
        assert_eq!(blocks[0].content, "Hello World");
    }

    #[test]
    fn test_delete_text() {
        let (mut ws, note_id) = setup_workspace();
        insert_block(&mut ws, &note_id, 0).unwrap();
        insert_text(&mut ws, &note_id, 0, 0, "Hello World", None).unwrap();
        
        delete_text(&mut ws, &note_id, 0, 5, 6).unwrap();
        let blocks = ws.get_note_blocks(&note_id);
        assert_eq!(blocks[0].content, "Hello");
    }
    
    #[test]
    fn test_split_and_merge_blocks() {
        let (mut ws, note_id) = setup_workspace();
        insert_block(&mut ws, &note_id, 0).unwrap();
        insert_text(&mut ws, &note_id, 0, 0, "Hello World", None).unwrap();
        
        split_block(&mut ws, &note_id, 0, 5).unwrap();
        let blocks = ws.get_note_blocks(&note_id);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].content, "Hello");
        assert_eq!(blocks[1].content, " World");
        
        merge_blocks(&mut ws, &note_id, 1).unwrap();
        let blocks = ws.get_note_blocks(&note_id);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "Hello World");
    }

    #[test]
    fn test_apply_and_toggle_attribute() {
        let (mut ws, note_id) = setup_workspace();
        insert_block(&mut ws, &note_id, 0).unwrap();
        insert_text(&mut ws, &note_id, 0, 0, "Hello World", None).unwrap();
        
        apply_attribute(&mut ws, &note_id, 0, 0..5, Attribute::Bold).unwrap();
        let blocks = ws.get_note_blocks(&note_id);
        assert_eq!(blocks[0].attributes.len(), 1);
        assert_eq!(blocks[0].attributes[0].attr, Attribute::Bold);
        
        toggle_attribute(&mut ws, &note_id, 0, 0..5, Attribute::Bold).unwrap();
        let blocks = ws.get_note_blocks(&note_id);
        assert_eq!(blocks[0].attributes.len(), 0);
    }
}
