use onyx_core::{blocks::{Attribute, BlockType}, OnyxWorkspace};
use parley::{style::StyleProperty, FontContext, Layout};
use vello::{
    kurbo::{Affine, Line, Rect, Stroke},
    peniko::{Brush, Color, Fill},
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

    pub fn draw(
        &mut self,
        scene: &mut Scene,
        ws: &OnyxWorkspace,
        note_id: &str,
        window_width: f64,
        top_margin: f64,
        cursor: &crate::cursor::CursorState,
        active_block_idx: Option<usize>,
    ) {
        let mut y_offset = top_margin + 40.0;
        self.layouts.clear();

        // --- RENDER CRDT BLOCKS ---
        let block_ids_opt = ws.get_note_block_ids(note_id);
        let Some(block_ids) = block_ids_opt else {
            return;
        };

        let blocks = ws.get_note_blocks(note_id);

        for (i, block_id) in block_ids.iter().enumerate() {
            let styled_spans_opt = ws.get_styled_text(block_id);
            let Some(styled_spans) = styled_spans_opt else {
                continue;
            };
            let content_opt = ws.get_block_content(block_id);
            let Some(content) = content_opt else { continue };

            // Determine font size based on block type
            let block_kind = blocks.get(i).map(|b| &b.kind);
            let font_size = match block_kind {
                Some(BlockType::Heading(1)) => 36.0f32,
                Some(BlockType::Heading(2)) => 28.0f32,
                Some(BlockType::Heading(3)) => 24.0f32,
                Some(BlockType::Heading(4)) => 20.0f32,
                Some(BlockType::Heading(5)) => 18.0f32,
                Some(BlockType::Heading(6)) => 16.0f32,
                _ => 18.0f32,
            };
            let is_heading = matches!(block_kind, Some(BlockType::Heading(_)));
            let is_code = matches!(block_kind, Some(BlockType::CodeBlock { .. }));
            let is_math = matches!(block_kind, Some(BlockType::MathBlock) | Some(BlockType::Math { .. }));

            let default_brush = if is_code {
                Brush::Solid(Color::from_rgba8(200, 200, 210, 255))
            } else if is_math {
                Brush::Solid(Color::from_rgba8(240, 240, 210, 255))
            } else {
                Brush::Solid(Color::from_rgba8(220, 220, 230, 255))
            };

            let mut layout_builder = self.layout_context.ranged_builder(
                &mut self.font_context,
                content.as_str(),
                1.0,
                false,
            );

            layout_builder.push_default(StyleProperty::FontSize(font_size));
            layout_builder.push_default(StyleProperty::Brush(default_brush.clone()));
            if is_heading {
                layout_builder.push_default(StyleProperty::FontWeight(parley::style::FontWeight::BOLD));
            }
            if is_code {
                // Approximate a monospace look using a generic family fallback in parley
                layout_builder.push_default(StyleProperty::FontStack(
                    parley::style::FontStack::Single(parley::style::FontFamily::Generic(
                        parley::style::GenericFamily::Monospace,
                    )),
                ));
            }
            if is_math {
                layout_builder.push_default(StyleProperty::FontStyle(parley::style::FontStyle::Italic));
                layout_builder.push_default(StyleProperty::FontStack(
                    parley::style::FontStack::Single(parley::style::FontFamily::Generic(
                        parley::style::GenericFamily::Serif,
                    )),
                ));
            }

            let mut byte_pos = 0usize;
            for (text_segment, attributes) in &styled_spans {
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
                        Attribute::Underline => {
                            // Manual drawing handled below
                        }
                        Attribute::Strikethrough => {
                            // Manual drawing handled below
                        }
                        Attribute::Superscript => {
                            layout_builder.push(StyleProperty::FontSize(font_size * 0.7), range.clone());
                        }
                        Attribute::Subscript => {
                            layout_builder.push(StyleProperty::FontSize(font_size * 0.7), range.clone());
                        }
                        Attribute::Color(c) => {
                            let color = Color::new([c[0], c[1], c[2], c[3]]);
                            layout_builder
                                .push(StyleProperty::Brush(Brush::Solid(color)), range.clone());
                        }
                        Attribute::FontSize(size) => {
                            layout_builder.push(StyleProperty::FontSize(*size), range.clone());
                        }
                        Attribute::FontFamily(name) => {
                            layout_builder.push(
                                StyleProperty::FontStack(
                                    parley::style::FontStack::Single(parley::style::FontFamily::Named(name.into()))
                                ),
                                range.clone()
                            );
                        }
                        Attribute::ClozeGap { hidden, .. } if *hidden => {
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

            let mut parley_align = if let Some(block) = blocks.get(i) {
                match block.align.as_str() {
                    "center" => parley::layout::Alignment::Center,
                    "right" => parley::layout::Alignment::End,
                    _ => parley::layout::Alignment::Start,
                }
            } else {
                parley::layout::Alignment::Start
            };

            if is_math {
                parley_align = parley::layout::Alignment::Center;
            }

            let indent_x = if let Some(block) = blocks.get(i) {
                (block.indent_level as f64) * 30.0
            } else {
                0.0
            };

            // block UI handles — `+` on the right side of the block
            if active_block_idx == Some(i) {
                let right_edge = 60.0 + indent_x + max_width as f64 + 16.0;
                let handle_y = y_offset + 5.0;

                // `+` icon background
                let plus_rect = Rect::new(right_edge, handle_y, right_edge + 18.0, handle_y + 18.0);
                scene.fill(
                    Fill::NonZero, Affine::IDENTITY,
                    &Brush::Solid(Color::from_rgba8(255, 255, 255, 30)), None,
                    &vello::kurbo::RoundedRect::new(plus_rect.x0, plus_rect.y0, plus_rect.x1, plus_rect.y1, 4.0),
                );
                // Plus vertical line
                scene.stroke(
                    &Stroke::new(1.5), Affine::IDENTITY,
                    &Brush::Solid(Color::from_rgba8(180, 180, 190, 255)), None,
                    &Line::new((right_edge + 9.0, handle_y + 4.0), (right_edge + 9.0, handle_y + 14.0)),
                );
                // Plus horizontal line
                scene.stroke(
                    &Stroke::new(1.5), Affine::IDENTITY,
                    &Brush::Solid(Color::from_rgba8(180, 180, 190, 255)), None,
                    &Line::new((right_edge + 4.0, handle_y + 9.0), (right_edge + 14.0, handle_y + 9.0)),
                );
            }

            let max_w = (window_width as f32 - 120.0 - indent_x as f32).max(1.0);
            layout.align(
                Some(max_w),
                parley_align,
                parley::layout::AlignmentOptions::default(),
            );

            let transform = Affine::translate((60.0 + indent_x, y_offset));

            // Code block background
            if is_code {
                let bg_rect = Rect::new(
                    60.0 + indent_x - 10.0,
                    y_offset - 4.0,
                    60.0 + indent_x + max_w as f64 + 10.0,
                    y_offset + layout.height() as f64 + 4.0,
                );
                scene.fill(
                    Fill::NonZero, Affine::IDENTITY,
                    &Brush::Solid(Color::from_rgba8(30, 30, 35, 255)), None,
                    &vello::kurbo::RoundedRect::new(bg_rect.x0, bg_rect.y0, bg_rect.x1, bg_rect.y1, 6.0),
                );
                scene.stroke(
                    &Stroke::new(1.0), Affine::IDENTITY,
                    &Brush::Solid(Color::from_rgba8(60, 60, 65, 255)), None,
                    &vello::kurbo::RoundedRect::new(bg_rect.x0, bg_rect.y0, bg_rect.x1, bg_rect.y1, 6.0),
                );
            }

            // Math block background
            if is_math {
                let bg_rect = Rect::new(
                    60.0 + indent_x,
                    y_offset - 4.0,
                    60.0 + indent_x + max_w as f64,
                    y_offset + layout.height() as f64 + 4.0,
                );
                scene.fill(
                    Fill::NonZero, Affine::IDENTITY,
                    &Brush::Solid(Color::from_rgba8(20, 25, 30, 255)), None,
                    &vello::kurbo::RoundedRect::new(bg_rect.x0, bg_rect.y0, bg_rect.x1, bg_rect.y1, 4.0),
                );
            }

            // --- ATTRIBUTE DECORATIONS (Highlights, Underlines, Strikethroughs) ---
            if let Some(block) = blocks.get(i) {
                for span in &block.attributes {
                    let is_hl = matches!(&span.attr, Attribute::Highlight(_));
                    let is_ul = matches!(&span.attr, Attribute::Underline);
                    let is_st = matches!(&span.attr, Attribute::Strikethrough);
                    
                    if !is_hl && !is_ul && !is_st { continue; }

                    let hl_color = if let Attribute::Highlight(c) = &span.attr {
                        Color::new([c[0], c[1], c[2], c[3]])
                    } else { Color::TRANSPARENT };

                    // Find geometry for this byte range using layout lines
                    for line in layout.lines() {
                        let line_metrics = line.metrics();
                        let line_y = line_metrics.baseline - line_metrics.ascent;
                        let line_height = line_metrics.ascent + line_metrics.descent;
                        let baseline = line_metrics.baseline as f64;
                        for item in line.items() {
                            if let parley::layout::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                                let run = glyph_run.run();
                                let run_range = run.text_range();

                                // Quick overlap check between the run and the attribute span
                                if run_range.start < span.end && run_range.end > span.start {
                                    let mut x_start = None;
                                    let mut x_end = None;

                                    let run_x = glyph_run.offset() as f64;
                                    let mut cluster_x = run_x;

                                    for cluster in run.clusters() {
                                        let cluster_range = cluster.text_range();
                                        let c_len = cluster.advance() as f64;

                                        // If this cluster overlaps the span
                                        if cluster_range.start < span.end && cluster_range.end > span.start {
                                            if x_start.is_none() {
                                                x_start = Some(cluster_x);
                                            }
                                            x_end = Some(cluster_x + c_len);
                                        }
                                        cluster_x += c_len;
                                    }

                                    if let (Some(xs), Some(xe)) = (x_start, x_end) {
                                        if is_hl {
                                            let hl_rect = Rect::new(
                                                60.0 + indent_x + xs,
                                                y_offset + line_y as f64,
                                                60.0 + indent_x + xe,
                                                y_offset + line_y as f64 + line_height as f64,
                                            );
                                            scene.fill(
                                                Fill::NonZero, Affine::IDENTITY,
                                                &Brush::Solid(hl_color), None, &hl_rect,
                                            );
                                        }
                                        if is_ul || is_st {
                                            let stroke_y = if is_ul {
                                                y_offset + baseline + 2.0
                                            } else {
                                                y_offset + baseline - (line_height as f64 * 0.3)
                                            };
                                            scene.stroke(
                                                &vello::kurbo::Stroke::new(1.5), Affine::IDENTITY,
                                                &Brush::Solid(Color::WHITE), None,
                                                &vello::kurbo::Line::new(
                                                    (60.0 + indent_x + xs, stroke_y),
                                                    (60.0 + indent_x + xe, stroke_y)
                                                ),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // --- CURSOR RENDERING ---
            if i == cursor.block_index && cursor.is_visible {
                let byte_off = cursor.byte_offset;
                let mut cursor_x: f64 = 0.0;
                let mut cursor_y: f64 = 0.0;
                let mut cursor_h: f64 = font_size as f64 + 4.0;
                let mut found = false;

                // Walk layout lines to find the cursor position
                for line in layout.lines() {
                    let line_metrics = line.metrics();
                    let line_top = (line_metrics.baseline - line_metrics.ascent) as f64;
                    let line_height = (line_metrics.ascent + line_metrics.descent) as f64;

                    for item in line.items() {
                        if let parley::layout::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                            let run = glyph_run.run();
                            let run_range = run.text_range();
                            let run_start = run_range.start;
                            let run_end = run_range.end;

                            if byte_off >= run_start && byte_off <= run_end {
                                // Cursor is within this glyph run
                                // Calculate X offset within the run
                                let run_x = glyph_run.offset() as f64;
                                
                                // Walk clusters to find exact x position
                                let mut cluster_x = run_x;
                                
                                for cluster in run.clusters() {
                                    let cluster_range = cluster.text_range();
                                    if byte_off <= cluster_range.start {
                                        break;
                                    }
                                    if byte_off >= cluster_range.end {
                                        cluster_x += cluster.advance() as f64;
                                    } else {
                                        // Cursor is within this cluster's range
                                        // Proportional positioning
                                        let frac = (byte_off - cluster_range.start) as f64 
                                            / (cluster_range.end - cluster_range.start) as f64;
                                        cluster_x += cluster.advance() as f64 * frac;
                                        break;
                                    }
                                }

                                cursor_x = cluster_x;
                                cursor_y = line_top;
                                cursor_h = line_height;
                                found = true;
                                break;
                            }
                        }
                    }
                    if found { break; }

                    // If cursor is at the very end past all runs on this line
                    if !found {
                        // Check if this is the last line and cursor is at end of content
                        let mut line_end = 0usize;
                        let mut last_x = 0.0f64;
                        for item in line.items() {
                            if let parley::layout::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                                let run = glyph_run.run();
                                let run_range = run.text_range();
                                if run_range.end > line_end {
                                    line_end = run_range.end;
                                    last_x = glyph_run.offset() as f64 + glyph_run.advance() as f64;
                                }
                            }
                        }
                        if byte_off == line_end || (byte_off >= line_end && content.len() == byte_off) {
                            cursor_x = last_x;
                            cursor_y = line_top;
                            cursor_h = line_height;
                            found = true;
                        }
                    }
                    if found { break; }
                }

                // Fallback: if layout has no runs (empty block), place cursor at start
                if !found {
                    cursor_x = 0.0;
                    cursor_y = 0.0;
                    cursor_h = font_size as f64 + 4.0;
                }


                // parley alignment shifts the text within the layout width
                // We need to find how much the line was shifted.
                // Actually, parley's GlyphRun::offset() is relative to the layout start.
                // If layout.align() was called, the offset() already includes the alignment shift!
                // HOWEVER, we need to ensure the layout has a width to align against.
                
                let cursor_rect = Rect::new(
                    60.0 + indent_x + cursor_x - 1.0,
                    y_offset + cursor_y,
                    60.0 + indent_x + cursor_x + 1.0,
                    y_offset + cursor_y + cursor_h,
                );
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    &Brush::Solid(Color::from_rgba8(96, 165, 250, 255)),
                    None,
                    &cursor_rect,
                );
            }

            // --- SELECTION HIGHLIGHT ---
            if let Some((start, end)) = cursor.get_selection_range() {
                let curr_block = i;
                if curr_block >= start.0 && curr_block <= end.0 {
                    let sel_start_byte = if curr_block == start.0 { start.1 } else { 0 };
                    let sel_end_byte = if curr_block == end.0 { end.1 } else { content.len() };

                    for line in layout.lines() {
                        let line_metrics = line.metrics();
                        let line_top = (line_metrics.baseline - line_metrics.ascent) as f64;
                        let line_height = (line_metrics.ascent + line_metrics.descent) as f64;

                        for item in line.items() {
                            if let parley::layout::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                                let run = glyph_run.run();
                                let run_range = run.text_range();
                                // Check overlap with selection
                                if run_range.start < sel_end_byte && run_range.end > sel_start_byte {
                                    let x_start = glyph_run.offset() as f64;
                                    let x_end = x_start + glyph_run.advance() as f64;
                                    let sel_rect = Rect::new(
                                        60.0 + indent_x + x_start,
                                        y_offset + line_top,
                                        60.0 + indent_x + x_end,
                                        y_offset + line_top + line_height,
                                    );
                                    scene.fill(
                                        Fill::NonZero, Affine::IDENTITY,
                                        &Brush::Solid(Color::from_rgba8(96, 165, 250, 60)),
                                        None, &sel_rect,
                                    );
                                }
                            }
                        }
                    }
                }
            }

            // --- BULLET / NUMBER PREFIX ---
            if let Some(block) = blocks.get(i) {
                match &block.kind {
                    BlockType::BulletList => {
                        let bullet_y = y_offset + font_size as f64 * 0.5;
                        let bullet_x = 45.0 + indent_x;
                        let brush = Brush::Solid(Color::from_rgba8(180, 180, 200, 255));
                        match block.indent_level % 3 {
                            0 => {
                                scene.fill(Fill::NonZero, Affine::IDENTITY, &brush, None, &vello::kurbo::Circle::new((bullet_x, bullet_y), 3.0));
                            }
                            1 => {
                                scene.stroke(&Stroke::new(1.5), Affine::IDENTITY, &brush, None, &vello::kurbo::Circle::new((bullet_x, bullet_y), 2.5));
                            }
                            _ => {
                                scene.fill(Fill::NonZero, Affine::IDENTITY, &brush, None, &Rect::new(bullet_x - 2.5, bullet_y - 2.5, bullet_x + 2.5, bullet_y + 2.5));
                            }
                        }
                    }
                    BlockType::NumberedList => {
                        // Find this block's index among consecutive numbered-list blocks
                        let mut num = 1;
                        for j in (0..i).rev() {
                            if matches!(blocks.get(j).map(|b| &b.kind), Some(BlockType::NumberedList)) {
                                num += 1;
                            } else {
                                break;
                            }
                        }
                        let num_str = format!("{}.", num);
                        // Simple text rendering for the number prefix
                        let mut builder = self.layout_context.ranged_builder(
                            &mut self.font_context, &num_str, 1.0, false,
                        );
                        builder.push_default(StyleProperty::FontSize(font_size));
                        builder.push_default(StyleProperty::Brush(
                            Brush::Solid(Color::from_rgba8(180, 180, 200, 255)),
                        ));
                        let mut num_layout = builder.build(&num_str);
                        num_layout.break_all_lines(None);
                        num_layout.align(None, parley::layout::Alignment::End, parley::layout::AlignmentOptions::default());
                        parley_vello::render_text(
                            scene,
                            Affine::translate((30.0 + indent_x, y_offset)),
                            &num_layout,
                        );
                    }
                    _ => {}
                }
            }

            parley_vello::render_text(scene, transform, &layout);

            y_offset += layout.height() as f64 + 24.0;
            self.layouts.push(layout);
        }
    }

    /// Convert a local click coordinate (x, y) into a byte offset within the given layout block.
    pub fn hit_test_x(&self, block_index: usize, x: f64, y: f64) -> usize {
        if let Some(layout) = self.layouts.get(block_index) {
            let mut closest_byte = 0;
            let mut min_dist = f64::MAX;

            for line in layout.lines() {
                let metrics = line.metrics();
                let line_top = (metrics.baseline - metrics.ascent) as f64;
                let line_bottom = (metrics.baseline + metrics.descent) as f64;

                for item in line.items() {
                    if let parley::layout::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                        let run = glyph_run.run();
                        let mut run_x_start = glyph_run.offset() as f64;

                        for cluster in run.clusters() {
                            let adv = cluster.advance() as f64;
                            let cluster_center = run_x_start + adv / 2.0;

                            let dist_x = (x - cluster_center).abs();
                            let dist_y = if y < line_top {
                                line_top - y
                            } else if y > line_bottom {
                                y - line_bottom
                            } else {
                                0.0
                            };
                            let dist = (dist_x * dist_x + dist_y * dist_y).sqrt();

                            if dist < min_dist {
                                min_dist = dist;
                                if x < cluster_center {
                                    closest_byte = cluster.text_range().start;
                                } else {
                                    closest_byte = cluster.text_range().end;
                                }
                            }
                            run_x_start += adv;

                            if x >= run_x_start {
                                let dist_end = ((x - run_x_start).abs().powi(2) + dist_y * dist_y).sqrt();
                                if dist_end < min_dist {
                                    min_dist = dist_end;
                                    closest_byte = cluster.text_range().end;
                                }
                            }
                        }
                    }
                }
            }
            return closest_byte;
        }
        0
    }
}
