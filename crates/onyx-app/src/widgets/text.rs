// ─── Onyx App — Text Widget (Cached Parley Layout) ─────────────────

use onyx_core::grid_layout::Rect;
use parley::layout::{Alignment, AlignmentOptions, PositionedLayoutItem};
use parley::style::StyleProperty;
use parley::{FontContext, Layout, LayoutContext};
use vello::kurbo::Affine;
use vello::peniko::{Brush, Color, Fill};
use vello::Scene;

use crate::widgets::{Widget, WidgetEvent};

/// A text widget that caches its Parley layout to avoid re-computation.
/// Set `dirty = true` when the text content changes.
pub struct TextWidget {
    pub text: String,
    pub font_size: f32,
    pub color: Color,
    pub dirty: bool,
    cached_layout: Option<Layout<Brush>>,
}

impl TextWidget {
    pub fn new(text: &str, font_size: f32, color: Color) -> Self {
        Self {
            text: text.to_string(),
            font_size,
            color,
            dirty: true,
            cached_layout: None,
        }
    }

    /// Update the text content, marking the layout as dirty.
    pub fn set_text(&mut self, text: &str) {
        if self.text != text {
            self.text = text.to_string();
            self.dirty = true;
            self.cached_layout = None;
        }
    }

    /// Rebuild the Parley layout if dirty. Call this before draw.
    pub fn ensure_layout(
        &mut self,
        font_cx: &mut FontContext,
        layout_cx: &mut LayoutContext<Brush>,
    ) {
        if !self.dirty && self.cached_layout.is_some() {
            return;
        }

        let brush = Brush::Solid(self.color);
        let mut builder = layout_cx.ranged_builder(font_cx, &self.text, 1.0, false);
        builder.push_default(StyleProperty::FontSize(self.font_size));
        builder.push_default(StyleProperty::Brush(brush));
        let mut layout = builder.build(&self.text);
        layout.break_all_lines(None);
        layout.align(None, Alignment::Start, AlignmentOptions::default());

        self.cached_layout = Some(layout);
        self.dirty = false;
    }

    /// Draw the cached layout into the scene at the given position.
    pub fn draw_at(&self, scene: &mut Scene, x: f64, y: f64) {
        let layout = match &self.cached_layout {
            Some(l) => l,
            None => return,
        };

        let brush = Brush::Solid(self.color);
        let transform = Affine::translate((x, y));

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
}

impl Widget for TextWidget {
    fn layout(&mut self, available: Rect) -> Rect {
        available
    }

    fn draw(&self, scene: &mut Scene, rect: &Rect) {
        self.draw_at(scene, rect.x as f64, rect.y as f64);
    }

    fn handle_event(&mut self, _event: &WidgetEvent, _rect: &Rect) -> bool {
        false
    }
}
