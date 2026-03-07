use parley::{FontContext, LayoutContext, Layout, PositionedLayoutItem, StyleProperty};
use vello::kurbo::Affine;
use vello::peniko::{self, Fill};
use vello::{Glyph, Scene};

pub struct SimpleText {
    pub text: String,
    pub font_size: f32,
    pub color: peniko::Color,
    pub(crate) layout: Option<Layout<peniko::Brush>>,
}

impl SimpleText {
    pub fn new(text: impl Into<String>, font_size: f32, color: peniko::Color) -> Self {
        Self {
            text: text.into(),
            font_size,
            color,
            layout: None,
        }
    }

    /// Shape and lay out the text. Call once (or when text/style changes).
    pub fn build(&mut self, font_cx: &mut FontContext, layout_cx: &mut LayoutContext<peniko::Brush>) {
        let mut builder = layout_cx.ranged_builder(font_cx, &self.text, 1.0, true);
        builder.push_default(StyleProperty::FontSize(self.font_size));
        builder.push_default(StyleProperty::Brush(self.color.into()));
        let mut layout = builder.build(&self.text);
        layout.break_all_lines(None);
        self.layout = Some(layout);
    }

    /// Draw the laid-out text into the scene at `(x, y)`.
    pub fn draw(&self, scene: &mut Scene, x: f64, y: f64) {
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
