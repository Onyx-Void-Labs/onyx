use vello::kurbo::{Affine, Line, Rect, Stroke};
use vello::peniko::{self, Fill};

use super::PaintCtx;

const TITLE_H: f64 = 40.0;
const BTN_W: f64 = 46.0;

/// Background — matches ONYX_BLACK so the seam is invisible.
const BG_COLOR: peniko::Color = peniko::Color::from_rgba8(0x09, 0x09, 0x0b, 0xff);
/// Close hover background — Windows 11 red.
const CLOSE_HOVER_BG: peniko::Color = peniko::Color::from_rgba8(196, 43, 28, 255);
/// Min/Max hover background — white 10%.
const SUBTLE_HOVER_BG: peniko::Color = peniko::Color::from_rgba8(255, 255, 255, 26);
/// Icon color — zinc-200.
const ICON_COLOR: peniko::Color = peniko::Color::from_rgba8(228, 228, 231, 255);
/// Icon color when close is hovered — white.
const ICON_WHITE: peniko::Color = peniko::Color::from_rgba8(255, 255, 255, 255);

/// Which title-bar button the cursor is hovering, if any.
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
    if cursor_x >= w - 46.0 {
        return Some(HoveredButton::Close);
    }
    if cursor_x >= w - 92.0 && cursor_x < w - 46.0 {
        return Some(HoveredButton::Maximise);
    }
    if cursor_x >= w - 138.0 && cursor_x < w - 92.0 {
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

    // --- Close button ---
    let close_x = window_w - BTN_W;
    let close_hover = hover == Some(HoveredButton::Close);
    if close_hover {
        cx.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            CLOSE_HOVER_BG,
            None,
            &Rect::new(close_x, 0.0, close_x + BTN_W, TITLE_H),
        );
    }
    {
        let cx_icon = close_x + 23.0;
        let cy_icon = 20.0;
        let color = if close_hover { ICON_WHITE } else { ICON_COLOR };
        cx.scene.stroke(
            &Stroke::new(1.1),
            Affine::IDENTITY,
            color,
            None,
            &Line::new(
                (cx_icon - 5.5, cy_icon - 5.5),
                (cx_icon + 5.5, cy_icon + 5.5),
            ),
        );
        cx.scene.stroke(
            &Stroke::new(1.1),
            Affine::IDENTITY,
            color,
            None,
            &Line::new(
                (cx_icon + 5.5, cy_icon - 5.5),
                (cx_icon - 5.5, cy_icon + 5.5),
            ),
        );
    }

    // --- Maximise button ---
    let max_x = window_w - BTN_W * 2.0;
    if hover == Some(HoveredButton::Maximise) {
        cx.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            SUBTLE_HOVER_BG,
            None,
            &Rect::new(max_x, 0.0, max_x + BTN_W, TITLE_H),
        );
    }
    {
        let cx_icon = max_x + 23.0;
        let cy_icon = 20.0;
        cx.scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            ICON_COLOR,
            None,
            &Rect::new(cx_icon - 9.0, cy_icon - 8.0, cx_icon + 9.0, cy_icon + 8.0),
        );
    }

    // --- Minimise button ---
    let min_x = window_w - BTN_W * 3.0;
    if hover == Some(HoveredButton::Minimise) {
        cx.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            SUBTLE_HOVER_BG,
            None,
            &Rect::new(min_x, 0.0, min_x + BTN_W, TITLE_H),
        );
    }
    {
        let cx_icon = min_x + 23.0;
        let cy_icon = 20.0;
        cx.scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            ICON_COLOR,
            None,
            &Line::new(
                (cx_icon - 6.0, cy_icon + 3.0),
                (cx_icon + 6.0, cy_icon + 3.0),
            ),
        );
    }
}
