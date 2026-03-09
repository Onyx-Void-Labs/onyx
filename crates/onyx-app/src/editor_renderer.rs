use onyx_core::{blocks::Attribute, OnyxWorkspace};
use parley::{style::StyleProperty, FontContext, Layout};
use vello::{
    kurbo::Affine,
    peniko::{Brush, Color},
    Scene,
};

pub struct EditorRenderer {
    pub font_context: FontContext,
    pub layout_context: parley::LayoutContext<Brush>,
    pub layouts: Vec<Layout<Brush>>,
}

impl EditorRenderer {
    pub fn new() -> Self {
        let mut font_context = FontContext::new();
        let font_data = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
        font_context
            .collection
            .register_fonts(font_data.to_vec().into(), None);
        Self {
            font_context,
            layout_context: parley::LayoutContext::new(),
            layouts: Vec::new(),
        }
    }

    pub fn build_scene(
        &mut self,
        scene: &mut Scene,
        ws: &OnyxWorkspace,
        note_id: &str,
        window_width: f64,
        top_margin: f64,
        live_buffer: &str,
    ) {
        let mut y_offset = top_margin + 40.0;
        self.layouts.clear();

        // --- RENDER LIVE TYPING BUFFER ---
        if !live_buffer.is_empty() {
            let font_size = 24.0;
            let brush = Brush::Solid(Color::from_rgba8(220, 220, 230, 255));
            let mut layout_builder =
                self.layout_context
                    .ranged_builder(&mut self.font_context, live_buffer, 1.0, false);
            layout_builder.push_default(StyleProperty::FontSize(font_size));
            layout_builder.push_default(StyleProperty::Brush(brush));

            let mut layout = layout_builder.build(live_buffer);
            let max_width = (window_width - 120.0).max(300.0) as f32;

            layout.break_all_lines(Some(max_width));
            layout.align(
                Some(max_width),
                parley::layout::Alignment::Start,
                parley::layout::AlignmentOptions::default(),
            );

            let transform = Affine::translate((60.0, y_offset));
            parley_vello::render_text(scene, transform, &layout);

            y_offset += layout.height() as f64 + 40.0;
        }

        // --- RENDER CRDT BLOCKS ---
        let block_ids_opt = ws.get_note_block_ids(note_id);
        let Some(block_ids) = block_ids_opt else {
            return;
        };

        for block_id in block_ids {
            let styled_spans_opt = ws.get_styled_text(&block_id);
            let Some(styled_spans) = styled_spans_opt else {
                continue;
            };
            let content_opt = ws.get_block_content(&block_id);
            let Some(content) = content_opt else { continue };

            let font_size = 24.0;
            let default_brush = Brush::Solid(Color::from_rgba8(220, 220, 230, 255));

            let mut layout_builder = self.layout_context.ranged_builder(
                &mut self.font_context,
                content.as_str(),
                1.0,
                false,
            );

            layout_builder.push_default(StyleProperty::FontSize(font_size));
            layout_builder.push_default(StyleProperty::Brush(default_brush.clone()));

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
                            layout_builder.push(
                                StyleProperty::FontStyle(parley::style::FontStyle::Italic),
                                range.clone(),
                            );
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

            let mut layout = layout_builder.build(content.as_str());

            let max_width = (window_width - 120.0).max(300.0) as f32;

            layout.break_all_lines(Some(max_width));
            layout.align(
                Some(max_width),
                parley::layout::Alignment::Start,
                parley::layout::AlignmentOptions::default(),
            );

            let transform = Affine::translate((60.0, y_offset));
            parley_vello::render_text(scene, transform, &layout);

            y_offset += layout.height() as f64 + 24.0;
            self.layouts.push(layout);
        }
    }
}
