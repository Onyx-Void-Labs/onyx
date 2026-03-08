// In `crates/onyx-app/src/editor_renderer.rs`

use onyx_core::{blocks::Attribute, OnyxWorkspace};

use parley::{
    style::{FontStyle, StyleProperty},
    FontContext, Layout,
};
use vello::{
    kurbo::{Affine, BezPath, Rect, Shape},
    peniko::{Brush, Color, Fill},
    Scene,
};

// The adapter to render parley layouts with Vello
use parley_vello; // free function available

pub struct Cursor {
    pub block_id: String,
    pub char_offset: usize,
}

pub struct EditorRenderer {
    pub font_context: FontContext,
    pub layout_context: parley::LayoutContext<Brush>,
    pub layouts: Vec<Layout<Brush>>,
    pub cursor: Option<Cursor>,
}

impl EditorRenderer {
    pub fn new() -> Self {
        let mut font_context = FontContext::new();
        // Load Inter-Regular.ttf from assets/fonts at compile time
        let font_data = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
        font_context
            .collection
            .register_fonts(font_data.to_vec().into(), None);
        Self {
            font_context,
            layout_context: parley::LayoutContext::new(),
            layouts: Vec::new(),
            cursor: None,
        }
    }

    pub fn build_scene(&mut self, scene: &mut Scene, ws: &OnyxWorkspace, note_id: &str) {
        tracing::info!("[DEBUG] build_scene called with note_id: {}", note_id);
        let block_ids_opt = ws.get_note_block_ids(note_id);
        tracing::info!(
            "[DEBUG] get_note_block_ids({}): {:?}",
            note_id,
            block_ids_opt
        );
        let Some(block_ids) = block_ids_opt else {
            return;
        };
        let mut y_offset = 50.0;

        self.layouts.clear();

        for block_id in block_ids {
            tracing::info!("[DEBUG] Rendering block_id: {}", block_id);
            let styled_spans_opt = ws.get_styled_text(&block_id);
            tracing::info!(
                "[DEBUG] get_styled_text({}): {:?}",
                block_id,
                styled_spans_opt.as_ref().map(|v| v.len())
            );
            let Some(styled_spans) = styled_spans_opt else {
                continue;
            };
            let content_opt = ws.get_block_content(&block_id);
            tracing::info!("[DEBUG] get_block_content({}): {:?}", block_id, content_opt);
            let Some(content) = content_opt else { continue };

            // --- VISIBILITY FIX: Establish a visible default color ---

            let font_size = 16.0;
            let default_brush = Brush::Solid(Color::WHITE);
            tracing::info!("[DEBUG] font_size: {}", font_size);
            tracing::info!("[DEBUG] font_family: (none specified, using Parley default)");

            let mut layout_builder = self.layout_context.ranged_builder(
                &mut self.font_context,
                content.as_str(),
                font_size,
                false,
            );
            layout_builder.push_default(StyleProperty::Brush(default_brush.clone()));

            // apply individual attribute spans via ranged pushes
            let mut byte_pos = 0usize;
            for (text_segment, attributes) in styled_spans {
                let seg_len = text_segment.as_bytes().len();
                let range = byte_pos..(byte_pos + seg_len);
                for attr in attributes {
                    match attr {
                        Attribute::Bold => {
                            layout_builder.push(
                                StyleProperty::FontWeight(parley::style::FontWeight::BOLD),
                                range.clone(),
                            );
                        }
                        Attribute::Italic => {
                            layout_builder
                                .push(StyleProperty::FontStyle(FontStyle::Italic), range.clone());
                        }
                        Attribute::Color(c) => {
                            let color = Color::new([c[0], c[1], c[2], c[3]]);
                            layout_builder
                                .push(StyleProperty::Brush(Brush::Solid(color)), range.clone());
                        }
                        Attribute::ClozeGap { hidden, .. } if hidden => {
                            layout_builder.push(
                                StyleProperty::Brush(Brush::Solid(Color::BLACK)),
                                range.clone(),
                            );
                        }
                        _ => {}
                    }
                }
                byte_pos += seg_len;
            }

            // Log constraints and font context state before build
            tracing::info!("[DEBUG] layout_builder.build for block_id: {}", block_id);
            tracing::info!(
                "[DEBUG] layout_builder.build: content.len()={}, font_size={}",
                content.len(),
                font_size
            );
            // No explicit font family specified; using Parley default.

            let mut layout = layout_builder.build(content.as_str());
            tracing::info!("[DEBUG] layout built: height={}", layout.height());
            layout.break_all_lines(Some(800.0));
            tracing::info!("height after break_lines: {}", layout.height());
            layout.align(
                Some(800.0),
                parley::layout::Alignment::Start,
                parley::layout::AlignmentOptions::default(),
            );
            // Debug: log layout geometry before rendering
            let num_lines = layout.lines().clone().count();
            tracing::info!("[DEBUG] layout has {} lines", num_lines);
            for (i, line) in layout.lines().clone().enumerate() {
                let mut run_count = 0;
                let mut glyph_count = 0;
                for item in line.items() {
                    if let parley::layout::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                        run_count += 1;
                        glyph_count += glyph_run.glyphs().count();
                    }
                }
                tracing::info!(
                    "[DEBUG] line {}: {} glyph runs, {} glyphs",
                    i,
                    run_count,
                    glyph_count
                );
            }

            // TEST: Big red rectangle to verify Vello pipeline
            let rect_path = Rect::new(100.0, 100.0, 500.0, 200.0).to_path(0.0);
            scene.fill(
                Fill::NonZero,                              // ARG 1: Fill rule FIRST
                Affine::IDENTITY,                           // ARG 2: Transform SECOND
                &Brush::Solid(Color::from_rgb8(255, 0, 0)), // Red via from_rgb8(u8)
                None,
                &rect_path,
            );
            tracing::info!("✅ Added red rect (100,100 -> 500,200)");

            // Use debug transform for text too
            let debug_transform = Affine::translate((150.0, 150.0));
            parley_vello::render_text(scene, debug_transform, &layout);
            tracing::info!("height after align: {}", layout.height());

            y_offset += layout.height() as f64 + 20.0;
            self.layouts.push(layout);
        }
        tracing::info!(
            "[DEBUG] build_scene complete, scene elements: {}",
            self.layouts.len()
        );
    }

    pub fn on_key_down(&mut self, key: &str) {
        println!("Key Down: {}", key);
    }

    pub fn on_mouse_click(&mut self, x: f64, y: f64) {
        println!("Mouse Click: ({}, {})", x, y);
    }
}
