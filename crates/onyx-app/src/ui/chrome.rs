// ─── Onyx Void — UI Chrome ─────────────────────────────────────────
// PathBar     — breadcrumb text labels at top-left.
// WindowControls — Min / Max / Close buttons (Windows 11 style).
// ────────────────────────────────────────────────────────────────────

use vello::kurbo::{Affine, Line, Rect, Size, Stroke};
use vello::peniko::{self, Fill};
use vello::Scene;

use crate::widgets::text::TextWidget;
use crate::widgets::{LayoutContext, Widget};

// ─── Palette ───────────────────────────────────────────────────────

/// Primary text — zinc-200.
const ZINC_200: peniko::Color = peniko::Color::from_rgba8(228, 228, 231, 255);
/// Muted text — zinc-400.
const ZINC_400: peniko::Color = peniko::Color::from_rgba8(0xa1, 0xa1, 0xaa, 0xff);
/// Separator — zinc-600.
const ZINC_600: peniko::Color = peniko::Color::from_rgba8(0x52, 0x52, 0x5b, 0xff);
/// Background (seamless with window) — #09090b.
const ONYX_BLACK: peniko::Color = peniko::Color::from_rgba8(0x09, 0x09, 0x0b, 0xff);
/// Close-button hover — Windows 11 red.
const CLOSE_HOVER_BG: peniko::Color = peniko::Color::from_rgba8(196, 43, 28, 255);
/// Subtle hover overlay — white 10 %.
const SUBTLE_HOVER_BG: peniko::Color = peniko::Color::from_rgba8(255, 255, 255, 26);
/// Icon color on close hover.
const ICON_WHITE: peniko::Color = peniko::Color::from_rgba8(255, 255, 255, 255);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  PathBar
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Breadcrumb-style path labels at the top-left of the window.
pub struct PathBar {
    segments: Vec<TextWidget>,
    separator: TextWidget,
}

impl PathBar {
    pub fn new(path: &[&'static str]) -> Self {
        let last = path.len().saturating_sub(1);
        let segments = path
            .iter()
            .enumerate()
            .map(|(i, &s)| {
                // Last segment is bright, others are muted.
                let color = if i == last { ZINC_200 } else { ZINC_400 };
                TextWidget::new(s, 13.0, color)
            })
            .collect();
        Self {
            segments,
            separator: TextWidget::new(" / ", 13.0, ZINC_600),
        }
    }

    /// Build / refresh text layouts.
    pub fn layout_all(&mut self, cx: &mut LayoutContext) {
        let constraints = Size::new(400.0, 30.0);
        for seg in &mut self.segments {
            seg.layout(cx, constraints);
        }
        self.separator.layout(cx, constraints);
    }

    /// Draw the path bar at a given vertical offset.
    pub fn draw(&self, scene: &mut Scene, y_offset: f64) {
        let mut x = 30.0;
        let y = y_offset + 10.0;
        for (i, seg) in self.segments.iter().enumerate() {
            if i > 0 {
                let sw = self.separator.cached_size().width;
                self.separator
                    .draw(scene, Rect::new(x, y, x + sw, y + 20.0));
                x += sw;
            }
            let w = seg.cached_size().width;
            seg.draw(scene, Rect::new(x, y, x + w, y + 20.0));
            x += w;
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//  WindowControls (Min / Max / Close — Windows 11 style)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Title-bar height in physical pixels.
const TITLE_H: f64 = 54.0;
/// Width of each window-control button.
const BTN_W: f64 = 58.0;

/// Which title-bar button the cursor is hovering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoveredButton {
    Close,
    Maximise,
    Minimise,
}

/// Static helpers for the three window-control buttons.
pub struct WindowControls;

impl WindowControls {
    /// Which button (if any) is hovered at the given cursor position.
    pub fn hovered_button(cx: f32, cy: f32, window_w: f32) -> Option<HoveredButton> {
        if cy < 0.0 || cy > TITLE_H as f32 {
            return None;
        }
        let w = window_w;
        if cx >= w - BTN_W as f32 {
            return Some(HoveredButton::Close);
        }
        if cx >= w - (BTN_W * 2.0) as f32 && cx < w - BTN_W as f32 {
            return Some(HoveredButton::Maximise);
        }
        if cx >= w - (BTN_W * 3.0) as f32 && cx < w - (BTN_W * 2.0) as f32 {
            return Some(HoveredButton::Minimise);
        }
        None
    }

    /// Paint the title-bar background and the three control buttons.
    pub fn draw(
        scene: &mut Scene,
        window_w: f64,
        hover: Option<HoveredButton>,
        is_maximized: bool,
    ) {
        // Title-bar background (seamless with ONYX_BLACK).
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            ONYX_BLACK,
            None,
            &Rect::new(0.0, 0.0, window_w, TITLE_H),
        );

        // ── Close ──
        let close_x = window_w - BTN_W;
        let close_hover = hover == Some(HoveredButton::Close);
        if close_hover {
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                CLOSE_HOVER_BG,
                None,
                &Rect::new(close_x, 0.0, close_x + BTN_W, TITLE_H),
            );
        }
        {
            let cx_i = close_x + BTN_W / 2.0;
            let cy_i = TITLE_H / 2.0;
            let color = if close_hover { ICON_WHITE } else { ZINC_200 };
            scene.stroke(
                &Stroke::new(1.3),
                Affine::IDENTITY,
                color,
                None,
                &Line::new((cx_i - 7.0, cy_i - 7.0), (cx_i + 7.0, cy_i + 7.0)),
            );
            scene.stroke(
                &Stroke::new(1.3),
                Affine::IDENTITY,
                color,
                None,
                &Line::new((cx_i + 7.0, cy_i - 7.0), (cx_i - 7.0, cy_i + 7.0)),
            );
        }

        // ── Maximise ──
        let max_x = window_w - BTN_W * 2.0;
        if hover == Some(HoveredButton::Maximise) {
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                SUBTLE_HOVER_BG,
                None,
                &Rect::new(max_x, 0.0, max_x + BTN_W, TITLE_H),
            );
        }
        {
            let cx_m = max_x + BTN_W / 2.0;
            let cy_m = TITLE_H / 2.0;
            if is_maximized {
                // Two overlapping rectangles (restore icon).
                scene.stroke(
                    &Stroke::new(1.1),
                    Affine::IDENTITY,
                    ZINC_200,
                    None,
                    &Rect::new(cx_m - 8.0, cy_m - 6.0, cx_m + 2.0, cy_m + 8.0),
                );
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    ONYX_BLACK,
                    None,
                    &Rect::new(cx_m - 2.0, cy_m - 2.0, cx_m + 8.0, cy_m + 12.0),
                );
                scene.stroke(
                    &Stroke::new(1.1),
                    Affine::IDENTITY,
                    ZINC_200,
                    None,
                    &Rect::new(cx_m - 2.0, cy_m - 2.0, cx_m + 8.0, cy_m + 12.0),
                );
            } else {
                // Single outlined square (maximise icon).
                scene.stroke(
                    &Stroke::new(1.1),
                    Affine::IDENTITY,
                    ZINC_200,
                    None,
                    &Rect::new(cx_m - 10.0, cy_m - 10.0, cx_m + 10.0, cy_m + 10.0),
                );
            }
        }

        // ── Minimise ──
        let min_x = window_w - BTN_W * 3.0;
        if hover == Some(HoveredButton::Minimise) {
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                SUBTLE_HOVER_BG,
                None,
                &Rect::new(min_x, 0.0, min_x + BTN_W, TITLE_H),
            );
        }
        {
            let cx_n = min_x + BTN_W / 2.0;
            let cy_n = TITLE_H / 2.0;
            scene.stroke(
                &Stroke::new(1.1),
                Affine::IDENTITY,
                ZINC_200,
                None,
                &Line::new((cx_n - 8.0, cy_n + 6.0), (cx_n + 8.0, cy_n + 6.0)),
            );
        }
    }
}
