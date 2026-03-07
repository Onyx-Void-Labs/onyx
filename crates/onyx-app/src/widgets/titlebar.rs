use vello::kurbo::{Affine, Line, Rect, Stroke};
use vello::peniko::{Color, Fill};
use vello::Scene;

use crate::HitRegion;

// All coordinates in LOGICAL pixels.
pub const TITLE_H: f32 = 40.;
pub const BTN_W: f32 = 46.;

pub struct TitleBar {
    pub hover: Option<HitRegion>,
}

impl TitleBar {
    pub fn new() -> Self {
        Self { hover: None }
    }

    pub fn paint(&self, scene: &mut Scene, window_w: f32, _window_h: f32) {
        let w = window_w as f64;

        // Button X positions (logical)
        let close_x = w - BTN_W as f64;
        let max_x = w - (BTN_W * 2.) as f64;
        let min_x = w - (BTN_W * 3.) as f64;
        let h = TITLE_H as f64;

        // ── Hover backgrounds ──────────────────────────────
        match self.hover {
            Some(HitRegion::Close) => {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    Color::from_rgba8(196, 43, 28, 255),
                    None,
                    &Rect::new(close_x, 0., w, h),
                );
            }
            Some(HitRegion::Maximise) => {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    Color::from_rgba8(255, 255, 255, 26),
                    None,
                    &Rect::new(max_x, 0., max_x + BTN_W as f64, h),
                );
            }
            Some(HitRegion::Minimise) => {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    Color::from_rgba8(255, 255, 255, 26),
                    None,
                    &Rect::new(min_x, 0., min_x + BTN_W as f64, h),
                );
            }
            _ => {}
        }

        // ── Icon colors ────────────────────────────────────
        let normal_icon = Color::from_rgba8(228, 228, 231, 255);
        let white_icon = Color::from_rgba8(255, 255, 255, 255);

        let close_icon_color = match self.hover {
            Some(HitRegion::Close) => white_icon,
            _ => normal_icon,
        };

        // ── Icon centers ───────────────────────────────────
        let close_cx = close_x + BTN_W as f64 / 2.;
        let max_cx = max_x + BTN_W as f64 / 2.;
        let min_cx = min_x + BTN_W as f64 / 2.;
        let cy = h / 2.;

        let stroke_thin = Stroke::new(1.0);
        let stroke_x = Stroke::new(1.1);

        // ── Minimise: horizontal line ──────────────────────
        scene.stroke(
            &stroke_thin,
            Affine::IDENTITY,
            normal_icon,
            None,
            &Line::new((min_cx - 6., cy + 4.), (min_cx + 6., cy + 4.)),
        );

        // ── Maximise: hollow square ────────────────────────
        scene.stroke(
            &stroke_thin,
            Affine::IDENTITY,
            normal_icon,
            None,
            &Rect::new(max_cx - 6., cy - 6., max_cx + 6., cy + 6.),
        );

        // ── Close: × two diagonals ─────────────────────────
        scene.stroke(
            &stroke_x,
            Affine::IDENTITY,
            close_icon_color,
            None,
            &Line::new((close_cx - 5.5, cy - 5.5), (close_cx + 5.5, cy + 5.5)),
        );
        scene.stroke(
            &stroke_x,
            Affine::IDENTITY,
            close_icon_color,
            None,
            &Line::new((close_cx + 5.5, cy - 5.5), (close_cx - 5.5, cy + 5.5)),
        );
    }
}
