// ─── Onyx App — Grid Renderer (Lens Renderer) ──────────────────────
//
// Translates GridEngine layouts into Vello draw calls.
// Architect Mode renders slot outlines for visual debugging.
// ────────────────────────────────────────────────────────────────────

use onyx_core::document::OnyxWorkspace;
use onyx_core::grid_layout::{resolve_layout, GridRow, Rect as GridRect};
use vello::kurbo::{Affine, Rect, RoundedRect, Stroke};
use vello::peniko::{Color, Fill};
use vello::Scene;

use crate::widgets::Widget;

/// Border color for Architect Mode slot outlines.
const SLOT_BORDER: Color = Color::from_rgba8(39, 39, 42, 255); // #27272a

/// Render a single grid row into the Vello scene.
///
/// For each slot, calculates pixel position via the 12-column layout,
/// then draws either the architect mode border or the widget content.
pub fn render_grid(
    scene: &mut Scene,
    _workspace: &OnyxWorkspace,
    row: &GridRow,
    container_rect: GridRect,
    widgets: &[Box<dyn Widget>],
    is_architect_mode: bool,
) {
    let rects = resolve_layout(container_rect.width, row);

    for (i, grid_rect) in rects.iter().enumerate() {
        // Offset by container origin
        let x = container_rect.x + grid_rect.x;
        let y = container_rect.y + grid_rect.y;
        let w = grid_rect.width;
        let h = grid_rect.height;

        let vello_rect = Rect::new(x as f64, y as f64, (x + w) as f64, (y + h) as f64);

        // Draw architect mode grid outlines
        if is_architect_mode {
            scene.stroke(
                &Stroke::new(1.0),
                Affine::IDENTITY,
                SLOT_BORDER,
                None,
                &RoundedRect::from_rect(vello_rect, 2.0),
            );
        }

        // Draw the widget if one exists for this slot
        if let Some(widget) = widgets.get(i) {
            let widget_rect = GridRect {
                x,
                y,
                width: w,
                height: h,
            };
            widget.draw(scene, &widget_rect);
        }
    }
}

/// Render architect mode grid overlay for the entire document area.
/// Draws the 12-column guide lines.
pub fn render_architect_overlay(
    scene: &mut Scene,
    container_width: f64,
    container_height: f64,
    origin_y: f64,
) {
    let col_w = container_width / 12.0;
    let guide_color = Color::from_rgba8(39, 39, 42, 100); // Semi-transparent #27272a

    for i in 1..12 {
        let x = col_w * i as f64;
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            guide_color,
            None,
            &Rect::new(x, origin_y, x + 1.0, origin_y + container_height),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use onyx_core::grid_layout::Slot;

    #[test]
    fn render_grid_architect_mode_does_not_panic() {
        let ws = OnyxWorkspace::new();
        let row = GridRow {
            slots: vec![
                Slot {
                    col_span: 6,
                    widget_id: "a".into(),
                },
                Slot {
                    col_span: 6,
                    widget_id: "b".into(),
                },
            ],
        };
        let container = GridRect {
            x: 0.0,
            y: 0.0,
            width: 1200.0,
            height: 300.0,
        };
        let mut scene = Scene::new();
        render_grid(&mut scene, &ws, &row, container, &[], true);
    }

    #[test]
    fn render_architect_overlay_does_not_panic() {
        let mut scene = Scene::new();
        render_architect_overlay(&mut scene, 1200.0, 800.0, 44.0);
    }
}
