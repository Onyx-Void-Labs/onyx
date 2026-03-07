// ─── Onyx App — Widget Framework (Trait + SlotRenderer) ─────────────

pub mod text;

use vello::Scene;

use onyx_core::grid_layout::{resolve_layout, GridRow, Rect};

/// A minimal event enum for widget interaction.
#[derive(Clone, Debug)]
pub enum WidgetEvent {
    Click { x: f32, y: f32 },
    KeyPress { key: char },
}

/// The core widget trait. Every renderable UI element implements this.
pub trait Widget {
    /// Compute the desired size of this widget given available space.
    fn layout(&mut self, available: Rect) -> Rect;

    /// Draw this widget into the Vello scene at the given rect.
    fn draw(&self, scene: &mut Scene, rect: &Rect);

    /// Handle an input event within the widget's bounds.
    /// Returns `true` if the event was consumed.
    fn handle_event(&mut self, event: &WidgetEvent, rect: &Rect) -> bool;
}

/// Manages a list of widgets and lays them out on a 12-column grid.
pub struct SlotRenderer {
    widgets: Vec<Box<dyn Widget>>,
    grid: GridRow,
}

impl SlotRenderer {
    /// Create a new SlotRenderer from widgets and their grid row definition.
    /// The number of widgets must match the number of slots.
    pub fn new(widgets: Vec<Box<dyn Widget>>, grid: GridRow) -> Self {
        debug_assert_eq!(
            widgets.len(),
            grid.slots.len(),
            "widgets and slots count must match"
        );
        Self { widgets, grid }
    }

    /// Resolve positions and draw all widgets into the scene.
    pub fn draw_all(&self, scene: &mut Scene, container_width: f32) {
        let rects = resolve_layout(container_width, &self.grid);
        for (widget, rect) in self.widgets.iter().zip(rects.iter()) {
            widget.draw(scene, rect);
        }
    }

    /// Layout all widgets given a container width.
    pub fn layout_all(&mut self, container_width: f32) -> Vec<Rect> {
        let rects = resolve_layout(container_width, &self.grid);
        for (widget, rect) in self.widgets.iter_mut().zip(rects.iter()) {
            widget.layout(rect.clone());
        }
        rects
    }

    /// Dispatch an event to the widget whose bounds contain the event position.
    pub fn handle_event(&mut self, event: &WidgetEvent, container_width: f32) -> bool {
        let rects = resolve_layout(container_width, &self.grid);
        for (widget, rect) in self.widgets.iter_mut().zip(rects.iter()) {
            if widget.handle_event(event, rect) {
                return true;
            }
        }
        false
    }

    /// Access the grid row definition.
    pub fn grid(&self) -> &GridRow {
        &self.grid
    }

    /// Number of widgets.
    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    /// Whether the renderer has no widgets.
    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use onyx_core::grid_layout::Slot;

    /// A trivial test widget that records calls.
    struct TestWidget {
        drawn: std::cell::Cell<bool>,
    }

    impl TestWidget {
        fn new() -> Self {
            Self {
                drawn: std::cell::Cell::new(false),
            }
        }
    }

    impl Widget for TestWidget {
        fn layout(&mut self, available: Rect) -> Rect {
            available
        }

        fn draw(&self, _scene: &mut Scene, _rect: &Rect) {
            self.drawn.set(true);
        }

        fn handle_event(&mut self, _event: &WidgetEvent, _rect: &Rect) -> bool {
            false
        }
    }

    #[test]
    fn slot_renderer_layout() {
        let widgets: Vec<Box<dyn Widget>> =
            vec![Box::new(TestWidget::new()), Box::new(TestWidget::new())];
        let grid = GridRow {
            slots: vec![
                Slot {
                    col_start: 0,
                    col_span: 6,
                    widget_id: "a".into(),
                },
                Slot {
                    col_start: 6,
                    col_span: 6,
                    widget_id: "b".into(),
                },
            ],
            collapsed: false,
        };
        let mut renderer = SlotRenderer::new(widgets, grid);
        let rects = renderer.layout_all(1200.0);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].width, 600.0);
        assert_eq!(rects[1].width, 600.0);
    }

    #[test]
    fn slot_renderer_draw() {
        let widgets: Vec<Box<dyn Widget>> = vec![Box::new(TestWidget::new())];
        let grid = GridRow {
            slots: vec![Slot {
                col_start: 0,
                col_span: 12,
                widget_id: "w".into(),
            }],
            collapsed: false,
        };
        let renderer = SlotRenderer::new(widgets, grid);
        let mut scene = Scene::new();
        renderer.draw_all(&mut scene, 1200.0);
    }

    #[test]
    fn slot_renderer_empty() {
        let renderer = SlotRenderer::new(vec![], GridRow { slots: vec![], collapsed: false });
        assert!(renderer.is_empty());
        assert_eq!(renderer.len(), 0);
    }
}
