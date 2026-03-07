// ─── Onyx Void — Micro Widget Framework ────────────────────────────
// Lightweight trait-based widget system. Pure Vello rendering,
// no HTML/CSS, no retained mode. Layout → Draw → Event.
// ────────────────────────────────────────────────────────────────────

pub mod text;

use vello::kurbo::{Rect, Size};
use vello::Scene;
use winit::event::WindowEvent;

// ─── Contexts ──────────────────────────────────────────────────────

/// Layout context passed during the layout pass.
pub struct LayoutContext<'a> {
    pub font_cx: &'a mut parley::FontContext,
    pub layout_cx: &'a mut parley::LayoutContext<vello::peniko::Brush>,
    pub scale: f32,
}

// ─── Action ────────────────────────────────────────────────────────

/// Result of handling an input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// No state changed.
    None,
    /// Widget needs a repaint.
    Redraw,
}

// ─── Widget Trait ──────────────────────────────────────────────────

/// Core widget trait for the Onyx micro-framework.
///
/// Every visual element implements this: layout determines size,
/// draw renders into a `vello::Scene`, and handle_event processes
/// `winit` events.
pub trait Widget {
    /// Compute the desired size given max constraints.
    fn layout(&mut self, cx: &mut LayoutContext, constraints: Size) -> Size;

    /// Render the widget into `scene` at the given `rect`.
    fn draw(&self, scene: &mut Scene, rect: Rect);

    /// Process an input event. Returns an `Action` indicating whether
    /// a redraw is needed.
    fn handle_event(&mut self, _event: &WindowEvent) -> Action {
        Action::None
    }
}
