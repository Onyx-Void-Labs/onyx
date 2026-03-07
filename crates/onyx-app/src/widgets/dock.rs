use vello::kurbo::{Affine, Circle, RoundedRect};
use vello::peniko::{Color, Fill};
use vello::Scene;

/// A rounded-rect command dock pinned to the bottom-center of the window.
pub struct CommandDock;

impl CommandDock {
    pub fn new() -> Self {
        Self
    }

    /// Paint the dock pill and its three placeholder circles.
    /// `window_w` / `window_h` are **logical** pixels.
    pub fn paint(&self, scene: &mut Scene, window_w: f32, window_h: f32) {
        let pill_w: f64 = 320.;
        let pill_h: f64 = 52.;
        let pill_x = (window_w as f64 - pill_w) / 2.;
        let pill_y = window_h as f64 - 72.;

        // Three circles CENTERED inside pill:
        // Total span of 3 circles at 48px spacing = 96px
        // Center of pill = pill_x + 160.
        // circle centers: pill_x+112, pill_x+160, pill_x+208
        let cy = pill_y + pill_h / 2.;
        let c1x = pill_x + 112.;
        let c2x = pill_x + 160.;
        let c3x = pill_x + 208.;
        let cr = 10_f64; // radius

        // Draw pill first, then circles on top
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgba8(24, 24, 27, 204),
            None,
            &RoundedRect::new(pill_x, pill_y, pill_x + pill_w, pill_y + pill_h, 26.),
        );

        for cx in [c1x, c2x, c3x] {
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                Color::from_rgba8(63, 63, 70, 255),
                None,
                &Circle::new((cx, cy), cr),
            );
        }
    }
}
