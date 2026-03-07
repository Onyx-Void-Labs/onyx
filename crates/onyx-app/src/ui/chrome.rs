// ─── Onyx Void — UI Chrome ─────────────────────────────────────────
// CommandDock  — frosted pill at bottom-center.
// PathBar     — breadcrumb text labels at top-left.
// WindowControls — Min / Max / Close buttons (Windows 11 style).
// ────────────────────────────────────────────────────────────────────

use vello::kurbo::{Affine, Line, Rect, RoundedRect, Size, Stroke};
use vello::peniko::{self, Fill};
use vello::Scene;

use crate::widgets::text::TextWidget;
use crate::widgets::{LayoutContext, Widget};

// ─── Palette ───────────────────────────────────────────────────────

/// Dock fill — #18181b at 80 % alpha.
const DOCK_COLOR: peniko::Color = peniko::Color::from_rgba8(24, 24, 27, 204);
/// Dock border — zinc-800.
const DOCK_BORDER: peniko::Color = peniko::Color::from_rgba8(0x27, 0x27, 0x2a, 0xff);
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
//  FormattingRibbon  (replaces CommandDock)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

const DOCK_WIDTH: f64 = 520.0;
const DOCK_HEIGHT: f64 = 52.0;
const DOCK_RADIUS: f64 = 26.0;
const DOCK_BOTTOM_MARGIN: f64 = 30.0;

/// Ribbon button inner size.
const BTN_SIZE: f64 = 32.0;
const BTN_RADIUS: f64 = 6.0;

/// Button pressed/active background — zinc-700.
const BTN_ACTIVE_BG: peniko::Color = peniko::Color::from_rgba8(0x3f, 0x3f, 0x46, 0xff);
/// Button hover background — zinc-800.
const BTN_HOVER_BG: peniko::Color = peniko::Color::from_rgba8(0x27, 0x27, 0x2a, 0xff);

/// Which ribbon button the cursor is currently over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RibbonHit {
    Bold,
    Italic,
    FontSizeMinus,
    FontSizePlus,
    Settings,
}

/// A frosted-glass formatting ribbon pinned to the bottom-center.
pub struct FormattingRibbon {
    // Text labels
    mode_label: TextWidget,
    bold_label: TextWidget,
    italic_label: TextWidget,
    minus_label: TextWidget,
    plus_label: TextWidget,
    settings_label: TextWidget,
    size_label: TextWidget,

    // State
    pub is_bold: bool,
    pub is_italic: bool,
    pub font_size: f32,
}

impl FormattingRibbon {
    pub fn new(font_size: f32) -> Self {
        Self {
            mode_label: TextWidget::new("\u{1F4C4} WRITE", 11.0, ZINC_200),
            bold_label: TextWidget::new("B", 13.0, ZINC_200),
            italic_label: TextWidget::new("I", 13.0, ZINC_200),
            minus_label: TextWidget::new("\u{2212}", 13.0, ZINC_200),
            plus_label: TextWidget::new("+", 13.0, ZINC_200),
            settings_label: TextWidget::new("\u{2699}", 13.0, ZINC_200),
            size_label: TextWidget::new(format!("{}", font_size as u32), 11.0, ZINC_200),
            is_bold: false,
            is_italic: false,
            font_size,
        }
    }

    /// Rebuild the font-size label when the value changes.
    pub fn update_size_label(&mut self) {
        self.size_label = TextWidget::new(format!("{}", self.font_size as u32), 11.0, ZINC_200);
    }

    /// Build / refresh text layouts.
    pub fn layout_labels(&mut self, cx: &mut LayoutContext) {
        let c = Size::new(120.0, 30.0);
        self.mode_label.layout(cx, c);
        self.bold_label.layout(cx, c);
        self.italic_label.layout(cx, c);
        self.minus_label.layout(cx, c);
        self.plus_label.layout(cx, c);
        self.settings_label.layout(cx, c);
        self.size_label.layout(cx, c);
    }

    /// Hit-test: which button (if any) is at `(px, py)` in physical coords.
    pub fn hit_test(&self, px: f64, py: f64, window_w: f64, window_h: f64) -> Option<RibbonHit> {
        let dock_x = (window_w - DOCK_WIDTH) / 2.0;
        let dock_y = window_h - DOCK_HEIGHT - DOCK_BOTTOM_MARGIN;
        if px < dock_x || px > dock_x + DOCK_WIDTH || py < dock_y || py > dock_y + DOCK_HEIGHT {
            return None;
        }
        // Compute button positions (same as draw)
        let cy = dock_y + (DOCK_HEIGHT - BTN_SIZE) / 2.0;
        let positions = self.button_positions(dock_x);
        for (hit, bx) in positions {
            let rect = Rect::new(bx, cy, bx + BTN_SIZE, cy + BTN_SIZE);
            if rect.contains(vello::kurbo::Point::new(px, py)) {
                return Some(hit);
            }
        }
        None
    }

    /// Button X positions relative to dock_x.
    fn button_positions(&self, dock_x: f64) -> Vec<(RibbonHit, f64)> {
        // Layout: [📄 WRITE] | [B] [I] | [-] [size] [+] | [⚙]
        let mut positions = Vec::new();
        let mut x = dock_x + 90.0; // after mode label + divider
        positions.push((RibbonHit::Bold, x));
        x += BTN_SIZE + 6.0;
        positions.push((RibbonHit::Italic, x));
        x += BTN_SIZE + 20.0; // gap for divider
        positions.push((RibbonHit::FontSizeMinus, x));
        x += BTN_SIZE + 40.0; // skip size label
        positions.push((RibbonHit::FontSizePlus, x));
        x += BTN_SIZE + 20.0; // gap for divider
        positions.push((RibbonHit::Settings, x));
        positions
    }

    /// Draw the ribbon at the bottom-center of the viewport.
    pub fn draw(&self, scene: &mut Scene, window_w: f64, window_h: f64) {
        let dock_x = (window_w - DOCK_WIDTH) / 2.0;
        let dock_y = window_h - DOCK_HEIGHT - DOCK_BOTTOM_MARGIN;

        // Pill background
        let pill = RoundedRect::new(
            dock_x,
            dock_y,
            dock_x + DOCK_WIDTH,
            dock_y + DOCK_HEIGHT,
            DOCK_RADIUS,
        );
        scene.fill(Fill::NonZero, Affine::IDENTITY, DOCK_COLOR, None, &pill);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            DOCK_BORDER,
            None,
            &pill,
        );

        let cy = dock_y + (DOCK_HEIGHT - BTN_SIZE) / 2.0;
        let label_cy =
            |label: &TextWidget| dock_y + (DOCK_HEIGHT - label.cached_size().height) / 2.0;

        // ── Left: Mode label ──
        let mode_x = dock_x + 16.0;
        let mode_y = label_cy(&self.mode_label);
        let msz = self.mode_label.cached_size();
        self.mode_label.draw(
            scene,
            Rect::new(mode_x, mode_y, mode_x + msz.width, mode_y + msz.height),
        );

        // Divider 1
        let div1_x = dock_x + 80.0;
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            ZINC_600,
            None,
            &Line::new(
                (div1_x, dock_y + 12.0),
                (div1_x, dock_y + DOCK_HEIGHT - 12.0),
            ),
        );

        // ── Center-left: Bold / Italic ──
        let positions = self.button_positions(dock_x);

        for &(ref hit, bx) in &positions {
            let is_active = match hit {
                RibbonHit::Bold => self.is_bold,
                RibbonHit::Italic => self.is_italic,
                _ => false,
            };
            let btn_rect = RoundedRect::new(bx, cy, bx + BTN_SIZE, cy + BTN_SIZE, BTN_RADIUS);
            if is_active {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    BTN_ACTIVE_BG,
                    None,
                    &btn_rect,
                );
            }
            let label = match hit {
                RibbonHit::Bold => &self.bold_label,
                RibbonHit::Italic => &self.italic_label,
                RibbonHit::FontSizeMinus => &self.minus_label,
                RibbonHit::FontSizePlus => &self.plus_label,
                RibbonHit::Settings => &self.settings_label,
            };
            let lsz = label.cached_size();
            let lx = bx + (BTN_SIZE - lsz.width) / 2.0;
            let ly = cy + (BTN_SIZE - lsz.height) / 2.0;
            label.draw(scene, Rect::new(lx, ly, lx + lsz.width, ly + lsz.height));
        }

        // Font size value label (between – and +)
        let size_x = dock_x + 90.0 + BTN_SIZE + 6.0 + BTN_SIZE + 20.0 + BTN_SIZE + 6.0;
        let size_y = label_cy(&self.size_label);
        let ssz = self.size_label.cached_size();
        self.size_label.draw(
            scene,
            Rect::new(size_x, size_y, size_x + ssz.width, size_y + ssz.height),
        );

        // Divider 2 (before –/+)
        let div2_x = dock_x + 90.0 + BTN_SIZE + 6.0 + BTN_SIZE + 10.0;
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            ZINC_600,
            None,
            &Line::new(
                (div2_x, dock_y + 12.0),
                (div2_x, dock_y + DOCK_HEIGHT - 12.0),
            ),
        );

        // Divider 3 (before ⚙)
        let div3_x =
            dock_x + 90.0 + BTN_SIZE + 6.0 + BTN_SIZE + 20.0 + BTN_SIZE + 40.0 + BTN_SIZE + 10.0;
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            ZINC_600,
            None,
            &Line::new(
                (div3_x, dock_y + 12.0),
                (div3_x, dock_y + DOCK_HEIGHT - 12.0),
            ),
        );
    }
}

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
