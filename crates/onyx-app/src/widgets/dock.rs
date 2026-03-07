use vello::kurbo::{Affine, Circle, RoundedRect};
use vello::peniko::{self, Fill};

use super::{LayoutCtx, PaintCtx, Widget};

const DOCK_WIDTH: f64 = 320.0;
const DOCK_HEIGHT: f64 = 52.0;
const CORNER_RADIUS: f64 = 26.0;
const BUTTON_RADIUS: f64 = 20.0;
const BUTTON_SPACING: f64 = 48.0;

/// Semi-transparent zinc-900 pill: rgba(24, 24, 27, 0.8).
const DOCK_COLOR: peniko::Color = peniko::Color::from_rgba8(24, 24, 27, 204);
/// Zinc-700 circle placeholders.
const BUTTON_COLOR: peniko::Color = peniko::Color::from_rgba8(0x3f, 0x3f, 0x46, 0xff);

/// A rounded-rect command dock pinned to the bottom-center of the window.
pub struct CommandDock {
    pub window_height: f64,
    pub window_width: f64,
}

impl CommandDock {
    pub fn new() -> Self {
        Self {
            window_height: 800.0,
            window_width: 1280.0,
        }
    }
}

impl Widget for CommandDock {
    fn layout(&mut self, _cx: &mut LayoutCtx, _max_width: f32) -> (f32, f32) {
        (DOCK_WIDTH as f32, DOCK_HEIGHT as f32)
    }

    fn paint(&self, cx: &mut PaintCtx, _x: f32, _y: f32) {
        let dock_x = (self.window_width - DOCK_WIDTH) / 2.0;
        let dock_y = self.window_height - 72.0;

        // Pill background.
        let pill = RoundedRect::new(
            dock_x,
            dock_y,
            dock_x + DOCK_WIDTH,
            dock_y + DOCK_HEIGHT,
            CORNER_RADIUS,
        );
        cx.scene
            .fill(Fill::NonZero, Affine::IDENTITY, DOCK_COLOR, None, &pill);

        // Three placeholder circle buttons centered inside the pill.
        let center_y = dock_y + DOCK_HEIGHT / 2.0;
        let center_x = self.window_width / 2.0;

        for i in [-1i32, 0, 1] {
            let bx = center_x + (i as f64) * BUTTON_SPACING;
            cx.scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                BUTTON_COLOR,
                None,
                &Circle::new((bx, center_y), BUTTON_RADIUS),
            );
        }
    }
}
