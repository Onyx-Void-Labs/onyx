
use onyx_core::OnyxWorkspace;
use onyx_core::editing::*;
use onyx_core::blocks::{Attribute, AttributeSpan};

fn setup_workspace() -> (OnyxWorkspace, String) {
    let mut ws = OnyxWorkspace::new();
    let void_id = ws.create_void(None, "Abuse Root").unwrap();
    let note_id = ws.create_note(&void_id, "Abuse Note").unwrap();
    insert_block(&mut ws, &note_id, 0).unwrap();
    (ws, note_id)
}

#[test]
fn t101_unicode_grapheme_boundary_abuse() {
    let (mut ws, note_id) = setup_workspace();
    // Complex emoji family (multiple code points joined by ZWJ)
    let rainbow_flag = "🏳️‍🌈"; // 🏳 (Waving White Flag) + FE0F (Variation Selector) + 200D (ZWJ) + 🌈 (Rainbow)
    insert_text(&mut ws, &note_id, 0, 0, rainbow_flag, None).unwrap();
    
    // Attempt deletion in the middle of the sequence
    // Using 3 (middle of flag + variation selector)
    delete_text(&mut ws, &note_id, 0, 3, 1).unwrap(); 
    
    let blocks = ws.get_note_blocks(&note_id);
    // Should not panic, should result in valid UTF-8
    let content = &blocks[0].content;
    assert!(std::str::from_utf8(content.as_bytes()).is_ok());
}

#[test]
fn t102_massive_attribute_normalization_stress() {
    let (mut ws, note_id) = setup_workspace();
    let text = "X".repeat(500);
    insert_text(&mut ws, &note_id, 0, 0, &text, None).unwrap();
    
    // Add 200 overlapping toggle-on operations
    for i in 0..200 {
        toggle_attribute(&mut ws, &note_id, 0, i..i+10, Attribute::Bold).unwrap();
    }
    
    let blocks = ws.get_note_blocks(&note_id);
    // Should be normalized to a single continuous Bold span
    assert_eq!(blocks[0].attributes.iter().filter(|s| s.attr == Attribute::Bold).count(), 1);
    let span = &blocks[0].attributes[0];
    assert_eq!(span.start, 0);
    assert_eq!(span.end, 209);
}

#[test]
fn t103_zero_width_attribute_spans() {
    let (mut ws, note_id) = setup_workspace();
    insert_text(&mut ws, &note_id, 0, 0, "Stability", None).unwrap();
    
    // Apply 0-width attribute
    apply_attribute(&mut ws, &note_id, 0, 4..4, Attribute::Underline).unwrap();
    
    // Split at that point
    split_block(&mut ws, &note_id, 0, 4).unwrap();
    
    let blocks = ws.get_note_blocks(&note_id);
    assert_eq!(blocks.len(), 2);
    // Ensure no panics during split/rendering logic simulation
}

#[test]
fn t104_deep_split_merge_cycles() {
    let (mut ws, note_id) = setup_workspace();
    insert_text(&mut ws, &note_id, 0, 0, "Long text for splitting", None).unwrap();
    
    for _ in 0..10 {
        split_block(&mut ws, &note_id, 0, 1).unwrap();
    }
    
    assert_eq!(ws.get_note_blocks(&note_id).len(), 11);
    
    for _ in 0..10 {
        merge_blocks(&mut ws, &note_id, 1).unwrap();
    }
    
    let blocks = ws.get_note_blocks(&note_id);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].content, "Long text for splitting");
}

#[test]
fn t105_overlapping_different_attribute_types() {
    let (mut ws, note_id) = setup_workspace();
    insert_text(&mut ws, &note_id, 0, 0, "Color and Highlight", None).unwrap();
    
    let red = Attribute::Color([1.0, 0.0, 0.0, 1.0]);
    let yellow = Attribute::Highlight([1.0, 1.0, 0.0, 1.0]);
    
    apply_attribute(&mut ws, &note_id, 0, 0..10, red).unwrap();
    apply_attribute(&mut ws, &note_id, 0, 5..15, yellow).unwrap();
    
    let blocks = ws.get_note_blocks(&note_id);
    assert_eq!(blocks[0].attributes.len(), 2);
}

#[test]
fn t106_out_of_bounds_resilience() {
    let (mut ws, note_id) = setup_workspace();
    
    // These should return Err rather than panicking
    assert!(insert_text(&mut ws, &note_id, 5, 0, "oops", None).is_err());
    
    // Delete in empty block
    delete_text(&mut ws, &note_id, 0, 0, 100).unwrap();
    assert_eq!(ws.get_note_blocks(&note_id)[0].content, "");
}
