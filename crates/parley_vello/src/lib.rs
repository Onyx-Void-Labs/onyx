use parley::layout::{Layout, PositionedLayoutItem};
use vello::{kurbo::Affine, peniko::{Brush, Fill}, Scene};

pub fn render_text(scene: &mut Scene, transform: Affine, layout: &Layout<Brush>) {
    for line in layout.lines() {
        for item in line.items() {
            if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                let run = glyph_run.run();
                let font = run.font();
                let font_size = run.font_size();
                let synthesis = run.synthesis();
                let glyph_xform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));

                let xform = match glyph_xform {
                    Some(gx) => transform * gx,
                    None => transform,
                };

                let brush = &glyph_run.style().brush;
                let run_y = glyph_run.baseline();

                // 1% OVERKILL: We must manually advance the pen across the screen!
                // Parley's `g.x` is merely the kerning/bearing adjustment.
                let mut cursor_x = glyph_run.offset();
                let mut vello_glyphs = Vec::with_capacity(glyph_run.glyphs().count());

                for g in glyph_run.glyphs() {
                    vello_glyphs.push(vello::Glyph {
                        id: g.id as u32,
                        x: cursor_x + g.x, // Apply minor font bearing adjustments
                        y: run_y - g.y,
                    });
                    // MARCH THE PEN FORWARD BY THE GLYPH'S ADVANCE WIDTH!
                    cursor_x += g.advance; 
                }

                scene
                    .draw_glyphs(font)
                    .font_size(font_size)
                    .transform(xform)
                    .normalized_coords(run.normalized_coords())
                    .hint(true)
                    .brush(brush)
                    .draw(Fill::NonZero, vello_glyphs.into_iter());
            }
        }
    }
}
