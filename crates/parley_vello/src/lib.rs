//! Adapter crate that bridges `parley`'s layout results to `vello` scenes.
//!
//! Currently this crate is a lightweight stub used solely to satisfy the
//! editor renderer's build-time dependency.  It exposes a `render_text`
//! function but does not perform any actual drawing; the real logic lives
//! in `onyx-app` for now and can gradually migrate here if a standalone
//! crate becomes useful.

use parley::layout::{Layout, PositionedLayoutItem};
use vello::{
    kurbo::Affine,
    peniko::{Brush, Color, Fill},
    Scene,
};

/// Render the given layout into the provided `Scene` at the specified
/// transformation.  This implementation is lightweight: it iterates over
/// glyph runs and forwards them to Vello using a fixed black brush.  All
/// layout and styling decisions are assumed to have been encoded in the
/// `Layout` already.
pub fn render_text(scene: &mut Scene, transform: Affine, layout: &Layout<Brush>) {
    let brush = Brush::Solid(Color::BLACK);

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

                scene
                    .draw_glyphs(font)
                    .font_size(font_size)
                    .transform(xform)
                    .brush(&brush)
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
