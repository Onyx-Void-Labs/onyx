// ─── Onyx Void — Parley Text Widget ────────────────────────────────
// High-quality text rendering via `parley` layout + `vello` glyph
// drawing. Re-uses `Layout` objects — only re-shapes when dirty.
// ────────────────────────────────────────────────────────────────────

use std::borrow::Cow;

use parley::{FontContext, Layout, LayoutContext, PositionedLayoutItem, StyleProperty};
use vello::kurbo::{Affine, Rect, Size};
use vello::peniko::{self, Fill};
use vello::{Glyph, Scene};
use winit::event::WindowEvent;

use super::{Action, LayoutContext as OnyxLayoutCtx, Widget};

/// A text widget backed by a Parley `Layout`.
///
/// Uses `Cow<'static, str>` for zero-copy string storage.
/// The internal layout is only rebuilt when the text or style changes.
pub struct TextWidget {
    pub text: Cow<'static, str>,
    pub font_size: f32,
    pub color: peniko::Color,
    layout: Option<Layout<peniko::Brush>>,
    cached_size: Size,
    dirty: bool,
}

impl TextWidget {
    pub fn new(text: impl Into<Cow<'static, str>>, font_size: f32, color: peniko::Color) -> Self {
        Self {
            text: text.into(),
            font_size,
            color,
            layout: None,
            cached_size: Size::ZERO,
            dirty: true,
        }
    }

    /// Replace the displayed text and mark the layout dirty.
    pub fn set_text(&mut self, text: impl Into<Cow<'static, str>>) {
        self.text = text.into();
        self.dirty = true;
        self.layout = None;
    }

    /// The last computed size (valid after `layout()`).
    pub fn cached_size(&self) -> Size {
        self.cached_size
    }

    /// Build / rebuild the Parley layout if needed.
    pub fn ensure_layout(
        &mut self,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<peniko::Brush>,
        max_width: Option<f32>,
    ) {
        if !self.dirty && self.layout.is_some() {
            return;
        }
        let mut builder = layout_cx.ranged_builder(font_cx, &self.text, 1.0, true);
        builder.push_default(StyleProperty::FontSize(self.font_size));
        builder.push_default(StyleProperty::Brush(self.color.into()));
        let mut layout = builder.build(&self.text);
        layout.break_all_lines(max_width);

        // Measure from line metrics.
        let mut w: f32 = 0.0;
        let mut h: f32 = 0.0;
        for line in layout.lines() {
            let metrics = line.metrics();
            w = w.max(metrics.advance);
            h += metrics.size();
        }
        self.cached_size = Size::new(w as f64, h as f64);
        self.layout = Some(layout);
        self.dirty = false;
    }

    /// Render glyphs into the scene at `(x, y)`.
    fn draw_at(&self, scene: &mut Scene, x: f64, y: f64) {
        let Some(layout) = &self.layout else { return };

        for line in layout.lines() {
            for item in line.items() {
                if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                    let run = glyph_run.run();
                    let font = run.font();
                    let font_size = run.font_size();
                    let coords = run.normalized_coords();
                    let style = glyph_run.style();

                    let glyphs: Vec<Glyph> = glyph_run
                        .positioned_glyphs()
                        .map(|g| Glyph {
                            id: g.id,
                            x: g.x,
                            y: g.y,
                        })
                        .collect();

                    scene
                        .draw_glyphs(font)
                        .font_size(font_size)
                        .transform(Affine::translate((x, y)))
                        .normalized_coords(coords)
                        .brush(&style.brush)
                        .draw(Fill::NonZero, glyphs.into_iter());
                }
            }
        }
    }
}

impl Widget for TextWidget {
    fn layout(&mut self, cx: &mut OnyxLayoutCtx, constraints: Size) -> Size {
        self.ensure_layout(cx.font_cx, cx.layout_cx, Some(constraints.width as f32));
        self.cached_size
    }

    fn draw(&self, scene: &mut Scene, rect: Rect) {
        self.draw_at(scene, rect.x0, rect.y0);
    }

    fn handle_event(&mut self, _event: &WindowEvent) -> Action {
        Action::None
    }
}
