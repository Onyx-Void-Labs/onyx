use vello::kurbo::{Affine, Circle, Rect};
use vello::peniko::{self, Fill};

use super::PaintCtx;

const TITLE_H: f64 = 36.0;
const BTN_W: f64 = 46.0;
const BTN_RADIUS: f64 = 7.0;

/// Background — matches ONYX_BLACK so the seam is invisible.
const BG_COLOR: peniko::Color = peniko::Color::from_rgba8(0x09, 0x09, 0x0b, 0xff);
/// Default button fill — zinc-700.
const BTN_DEFAULT: peniko::Color = peniko::Color::from_rgba8(0x3f, 0x3f, 0x46, 0xff);
/// Close hover — red-500.
const BTN_CLOSE_HOVER: peniko::Color = peniko::Color::from_rgba8(0xef, 0x44, 0x44, 0xff);
/// Maximise hover — green-500.
const BTN_MAX_HOVER: peniko::Color = peniko::Color::from_rgba8(0x22, 0xc5, 0x5e, 0xff);
/// Minimise hover — yellow-500.
const BTN_MIN_HOVER: peniko::Color = peniko::Color::from_rgba8(0xea, 0xb3, 0x08, 0xff);

/// Which traffic-light button the cursor is hovering, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoveredButton {
    Close,
    Maximise,
    Minimise,
}

/// Determine which button (if any) is hovered at the given cursor position.
pub fn hovered_button(cursor_x: f32, cursor_y: f32, window_w: f32) -> Option<HoveredButton> {
    if cursor_y < 0.0 || cursor_y > TITLE_H as f32 {
        return None;
    }
    let w = window_w;
    // Close: [w - BTN_W, 0, BTN_W, TITLE_H]
    if cursor_x >= w - BTN_W as f32 && cursor_x <= w {
        return Some(HoveredButton::Close);
    }
    // Maximise: [w - BTN_W*2, 0, BTN_W, TITLE_H]
    if cursor_x >= w - (BTN_W * 2.0) as f32 && cursor_x < w - BTN_W as f32 {
        return Some(HoveredButton::Maximise);
    }
    // Minimise: [w - BTN_W*3, 0, BTN_W, TITLE_H]
    if cursor_x >= w - (BTN_W * 3.0) as f32 && cursor_x < w - (BTN_W * 2.0) as f32 {
        return Some(HoveredButton::Minimise);
    }
    None
}

/// Paint the custom title bar chrome at the top of the window.
pub fn paint(cx: &mut PaintCtx, window_w: f64, hover: Option<HoveredButton>) {
    // Title bar background — seamless with ONYX_BLACK.
    cx.scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        BG_COLOR,
        None,
        &Rect::new(0.0, 0.0, window_w, TITLE_H),
    );

    // Traffic-light circles — top-right, Windows/Linux convention.
    let cy = TITLE_H / 2.0; // vertical center = 18

    // Close button: center at (w - 23, 18)
    let close_cx = window_w - 23.0;
    let close_color = if hover == Some(HoveredButton::Close) {
        BTN_CLOSE_HOVER
    } else {
        BTN_DEFAULT
    };
    cx.scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        close_color,
        None,
        &Circle::new((close_cx, cy), BTN_RADIUS),
    );

    // Maximise button: center at (w - 23 - 46, 18)
    let max_cx = window_w - 23.0 - BTN_W;
    let max_color = if hover == Some(HoveredButton::Maximise) {
        BTN_MAX_HOVER
    } else {
        BTN_DEFAULT
    };
    cx.scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        max_color,
        None,
        &Circle::new((max_cx, cy), BTN_RADIUS),
    );

    // Minimise button: center at (w - 23 - 92, 18)
    let min_cx = window_w - 23.0 - BTN_W * 2.0;
    let min_color = if hover == Some(HoveredButton::Minimise) {
        BTN_MIN_HOVER
    } else {
        BTN_DEFAULT
    };
    cx.scene.fill(
        Fill::NonZero,
        Affine::IDENTITY,
        min_color,
        None,
        &Circle::new((min_cx, cy), BTN_RADIUS),
    );
}
