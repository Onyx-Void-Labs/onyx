use parley::layout::{Layout, PositionedLayoutItem};
use vello::{
    kurbo::Affine,
    peniko::{Brush, Fill},
    Scene,
};

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

                // 1% OVERKILL: Pull the exact Brush calculated by Parley's CRDT spans
                let brush = &glyph_run.style().brush;

                scene
                    .draw_glyphs(font)
                    .font_size(font_size)
                    .transform(xform)
                    .brush(brush)
                    .draw(
                        Fill::NonZero,
                        glyph_run.glyphs().map(|g| vello::Glyph {
                            id: g.id as u32,
                            x: g.x,
                            y: g.y,
                        }),
                    );
            }
        }
    }
}
