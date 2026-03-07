// ─── Onyx Void — Lane Editor ───────────────────────────────────────
// Single-line text editor with cursor, keyboard input, and DPI-aware
// rendering via Parley + Vello.
// ────────────────────────────────────────────────────────────────────

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use parley::{Layout, PositionedLayoutItem, StyleProperty};
use vello::kurbo::{Affine, Rect, Size};
use vello::peniko::{self, Fill};
use vello::{Glyph, Scene};
use winit::event::{ElementState, WindowEvent};
use winit::keyboard::{Key, NamedKey};

use super::style::MarkdownStyler;
use super::{Action, LayoutContext as OnyxLayoutCtx, Widget};

/// Cursor color — Blue-600 (#2563eb).
const CURSOR_COLOR: peniko::Color = peniko::Color::from_rgba8(0x25, 0x63, 0xeb, 0xff);

/// Cursor blink interval in seconds.
const BLINK_INTERVAL: f64 = 0.53;

/// A DPI-aware text editor with a blinking cursor.
pub struct LaneEditor {
    pub text: String,
    pub cursor_idx: usize,
    pub font_size: f32,
    pub color: peniko::Color,

    // Layout cache
    layout: Option<Layout<peniko::Brush>>,
    cached_size: Size,
    cursor_x: f64,
    dirty: bool,
    last_layout_hash: u64,

    // Cursor blink
    blink_epoch: std::time::Instant,
    scale_factor: f64,
}

impl LaneEditor {
    pub fn new(text: impl Into<String>, font_size: f32, color: peniko::Color) -> Self {
        let t: String = text.into();
        let cursor = t.len();
        Self {
            text: t,
            cursor_idx: cursor,
            font_size,
            color,
            layout: None,
            cached_size: Size::ZERO,
            cursor_x: 0.0,
            dirty: true,
            last_layout_hash: 0,
            blink_epoch: std::time::Instant::now(),
            scale_factor: 1.0,
        }
    }

    /// The last computed size (valid after `layout()`).
    pub fn cached_size(&self) -> Size {
        self.cached_size
    }

    /// Whether the cursor should be visible right now (blink logic).
    pub fn cursor_visible(&self) -> bool {
        let elapsed = self.blink_epoch.elapsed().as_secs_f64();
        (elapsed / BLINK_INTERVAL) as u64 % 2 == 0
    }

    /// Reset blink timer (call on any edit / cursor move).
    fn reset_blink(&mut self) {
        self.blink_epoch = std::time::Instant::now();
    }

    /// Find the byte index of the previous char boundary.
    fn prev_char_boundary(&self, idx: usize) -> usize {
        let mut i = idx.saturating_sub(1);
        while i > 0 && !self.text.is_char_boundary(i) {
            i -= 1;
        }
        i
    }

    /// Find the byte index of the next char boundary.
    fn next_char_boundary(&self, idx: usize) -> usize {
        let mut i = idx + 1;
        while i < self.text.len() && !self.text.is_char_boundary(i) {
            i += 1;
        }
        i.min(self.text.len())
    }

    /// Compute cursor X offset by measuring text before the cursor.
    fn compute_cursor_x(
        font_cx: &mut parley::FontContext,
        layout_cx: &mut parley::LayoutContext<peniko::Brush>,
        text: &str,
        cursor_idx: usize,
        font_size: f32,
        scale: f64,
    ) -> f64 {
        if cursor_idx == 0 {
            return 0.0;
        }
        let before = &text[..cursor_idx];
        let mut builder = layout_cx.ranged_builder(font_cx, before, scale as f32, true);
        builder.push_default(StyleProperty::FontSize(font_size));
        builder.push_default(StyleProperty::Brush(
            peniko::Color::from_rgba8(0, 0, 0, 0).into(),
        ));
        let mut measure = builder.build(before);
        measure.break_all_lines(None);
        if let Some(line) = measure.lines().next() {
            return line.metrics().advance as f64;
        }
        0.0
    }

    /// Draw glyphs into the scene at `(x, y)`.
    fn draw_text(&self, scene: &mut Scene, x: f64, y: f64) {
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

impl Widget for LaneEditor {
    fn layout(&mut self, cx: &mut OnyxLayoutCtx, constraints: Size) -> Size {
        self.scale_factor = cx.scale_factor;

        // Content + scale hash — skip layout if nothing changed
        let mut hasher = DefaultHasher::new();
        self.text.hash(&mut hasher);
        cx.scale_factor.to_bits().hash(&mut hasher);
        let hash = hasher.finish();

        if hash == self.last_layout_hash && self.layout.is_some() {
            return self.cached_size;
        }
        self.last_layout_hash = hash;

        let scale = cx.scale_factor as f32;

        // Build main text layout with Markdown styling
        let mut builder = cx
            .layout_cx
            .ranged_builder(cx.font_cx, &self.text, scale, true);
        builder.push_default(StyleProperty::FontSize(self.font_size));
        builder.push_default(StyleProperty::Brush(self.color.into()));
        builder.push_default(StyleProperty::FontFamily(parley::FontFamily::named(
            "Inter",
        )));
        MarkdownStyler::apply_styles(&self.text, &mut builder);
        let mut layout = builder.build(&self.text);
        layout.break_all_lines(Some(constraints.width as f32));

        // Measure
        let mut w: f32 = 0.0;
        let mut h: f32 = 0.0;
        for line in layout.lines() {
            let metrics = line.metrics();
            w = w.max(metrics.advance);
            h += metrics.size();
        }
        self.cached_size = Size::new(w as f64, h as f64);
        self.layout = Some(layout);

        // Compute cursor X (separate measurement layout)
        self.cursor_x = Self::compute_cursor_x(
            cx.font_cx,
            cx.layout_cx,
            &self.text,
            self.cursor_idx,
            self.font_size,
            cx.scale_factor,
        );

        self.dirty = false;
        self.cached_size
    }

    fn draw(&self, scene: &mut Scene, rect: Rect) {
        // Draw text
        self.draw_text(scene, rect.x0, rect.y0);

        // Draw cursor
        if self.cursor_visible() {
            let cursor_w = 2.0 * self.scale_factor;
            let cursor_h = self.cached_size.height;
            let cx = rect.x0 + self.cursor_x;
            let cy = rect.y0;

            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                CURSOR_COLOR,
                None,
                &Rect::new(cx, cy, cx + cursor_w, cy + cursor_h),
            );
        }
    }

    fn handle_event(&mut self, event: &WindowEvent) -> Action {
        match event {
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                match &event.logical_key {
                    Key::Named(NamedKey::Backspace) => {
                        if self.cursor_idx > 0 {
                            let prev = self.prev_char_boundary(self.cursor_idx);
                            self.text.drain(prev..self.cursor_idx);
                            self.cursor_idx = prev;
                            self.dirty = true;
                            self.reset_blink();
                            return Action::Redraw;
                        }
                        Action::None
                    }
                    Key::Named(NamedKey::Delete) => {
                        if self.cursor_idx < self.text.len() {
                            let next = self.next_char_boundary(self.cursor_idx);
                            self.text.drain(self.cursor_idx..next);
                            self.dirty = true;
                            self.reset_blink();
                            return Action::Redraw;
                        }
                        Action::None
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        if self.cursor_idx > 0 {
                            self.cursor_idx = self.prev_char_boundary(self.cursor_idx);
                            self.dirty = true;
                            self.reset_blink();
                            return Action::Redraw;
                        }
                        Action::None
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        if self.cursor_idx < self.text.len() {
                            self.cursor_idx = self.next_char_boundary(self.cursor_idx);
                            self.dirty = true;
                            self.reset_blink();
                            return Action::Redraw;
                        }
                        Action::None
                    }
                    Key::Named(NamedKey::Home) => {
                        if self.cursor_idx > 0 {
                            self.cursor_idx = 0;
                            self.dirty = true;
                            self.reset_blink();
                            return Action::Redraw;
                        }
                        Action::None
                    }
                    Key::Named(NamedKey::End) => {
                        if self.cursor_idx < self.text.len() {
                            self.cursor_idx = self.text.len();
                            self.dirty = true;
                            self.reset_blink();
                            return Action::Redraw;
                        }
                        Action::None
                    }
                    Key::Named(NamedKey::Enter) => {
                        self.text.insert(self.cursor_idx, '\n');
                        self.cursor_idx += 1;
                        self.dirty = true;
                        self.reset_blink();
                        Action::Redraw
                    }
                    Key::Named(NamedKey::Space) => {
                        self.text.insert(self.cursor_idx, ' ');
                        self.cursor_idx += 1;
                        self.dirty = true;
                        self.reset_blink();
                        Action::Redraw
                    }
                    _ => {
                        if let Some(ref text) = event.text {
                            let s = text.as_str();
                            if !s.is_empty() && !s.chars().any(|c| c.is_control()) {
                                self.text.insert_str(self.cursor_idx, s);
                                self.cursor_idx += s.len();
                                self.dirty = true;
                                self.reset_blink();
                                return Action::Redraw;
                            }
                        }
                        Action::None
                    }
                }
            }
            _ => Action::None,
        }
    }
}
