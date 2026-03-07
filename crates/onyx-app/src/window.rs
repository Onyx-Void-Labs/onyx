// ─── Onyx Void — Custom Borderless Window Frame ────────────────────
// Wraps `winit::Window` with manual hit-testing for resize edges,
// title-bar drag, and window-control buttons (the "Discord" feel).
// ────────────────────────────────────────────────────────────────────

use std::sync::Arc;
use winit::event_loop::ActiveEventLoop;
use winit::window::{CursorIcon, ResizeDirection, Window};

/// Resize-edge sensitivity in physical pixels.
const EDGE: f32 = 6.0;
/// Title-bar height in physical pixels.
const TITLE_H: f32 = 40.0;
/// Width of each window-control button in physical pixels.
const BTN_W: f32 = 46.0;

// ─── HitRegion ─────────────────────────────────────────────────────

/// Regions of the custom window chrome for hit-testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitRegion {
    TitleBar,
    Close,
    Minimise,
    Maximise,
    ResizeN,
    ResizeS,
    ResizeE,
    ResizeW,
    ResizeNE,
    ResizeNW,
    ResizeSE,
    ResizeSW,
    Content,
}

/// Determine the hit-test region for cursor coordinates (physical pixels).
pub fn hit_test_region(x: f32, y: f32, w: f32, h: f32) -> HitRegion {
    // Corners (EDGE × EDGE squares)
    if x < EDGE && y < EDGE {
        return HitRegion::ResizeNW;
    }
    if x >= w - EDGE && y < EDGE {
        return HitRegion::ResizeNE;
    }
    if x < EDGE && y >= h - EDGE {
        return HitRegion::ResizeSW;
    }
    if x >= w - EDGE && y >= h - EDGE {
        return HitRegion::ResizeSE;
    }
    // Edges
    if y < EDGE {
        return HitRegion::ResizeN;
    }
    if y >= h - EDGE {
        return HitRegion::ResizeS;
    }
    if x < EDGE {
        return HitRegion::ResizeW;
    }
    if x >= w - EDGE {
        return HitRegion::ResizeE;
    }
    // Title-bar buttons (right-to-left)
    if x >= w - BTN_W && y < TITLE_H {
        return HitRegion::Close;
    }
    if x >= w - BTN_W * 2.0 && x < w - BTN_W && y < TITLE_H {
        return HitRegion::Maximise;
    }
    if x >= w - BTN_W * 3.0 && x < w - BTN_W * 2.0 && y < TITLE_H {
        return HitRegion::Minimise;
    }
    if y < TITLE_H {
        return HitRegion::TitleBar;
    }

    HitRegion::Content
}

// ─── WindowContext ─────────────────────────────────────────────────

/// Wraps a `winit::Window` with borderless-chrome state (cursor
/// position, window dimensions, double-click timer).
pub struct WindowContext {
    pub window: Arc<Window>,
    /// Last cursor position in **physical** pixels.
    pub cursor_pos: (f32, f32),
    /// Window dimensions in **physical** pixels.
    pub window_size: (f32, f32),
    /// Display scale factor (physical / logical).
    pub scale_factor: f64,
    /// For double-click detection on the title bar.
    last_click_time: Option<std::time::Instant>,
}

impl WindowContext {
    pub fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let scale = window.scale_factor();
        Self {
            window,
            cursor_pos: (0.0, 0.0),
            window_size: (size.width.max(1) as f32, size.height.max(1) as f32),
            scale_factor: scale,
            last_click_time: None,
        }
    }

    /// Logical window size (physical / scale_factor).
    pub fn logical_size(&self) -> (f32, f32) {
        let s = self.scale_factor as f32;
        (self.window_size.0 / s, self.window_size.1 / s)
    }

    /// Update cursor position and set the appropriate resize cursor icon.
    pub fn update_cursor(&mut self, x: f32, y: f32) {
        self.cursor_pos = (x, y);
        let (w, h) = self.window_size;
        let region = hit_test_region(x, y, w, h);
        let icon = match region {
            HitRegion::ResizeN => CursorIcon::NResize,
            HitRegion::ResizeS => CursorIcon::SResize,
            HitRegion::ResizeE => CursorIcon::EResize,
            HitRegion::ResizeW => CursorIcon::WResize,
            HitRegion::ResizeNE => CursorIcon::NeResize,
            HitRegion::ResizeNW => CursorIcon::NwResize,
            HitRegion::ResizeSE => CursorIcon::SeResize,
            HitRegion::ResizeSW => CursorIcon::SwResize,
            _ => CursorIcon::Default,
        };
        self.window.set_cursor(icon);
    }

    /// Current hit-test region under the cursor.
    pub fn current_region(&self) -> HitRegion {
        let (x, y) = self.cursor_pos;
        let (w, h) = self.window_size;
        hit_test_region(x, y, w, h)
    }

    /// Handle a left mouse-button press. Returns `true` if the app should exit.
    pub fn handle_click(&mut self, event_loop: &ActiveEventLoop) -> bool {
        let region = self.current_region();
        let now = std::time::Instant::now();

        match region {
            HitRegion::Close => {
                event_loop.exit();
                return true;
            }
            HitRegion::Minimise => {
                self.window.set_minimized(true);
            }
            HitRegion::Maximise => {
                self.window.set_maximized(!self.window.is_maximized());
            }
            HitRegion::TitleBar => {
                let is_dbl = self
                    .last_click_time
                    .map(|t| now.duration_since(t).as_millis() < 400)
                    .unwrap_or(false);
                if is_dbl {
                    self.window.set_maximized(!self.window.is_maximized());
                    self.last_click_time = None;
                } else {
                    self.last_click_time = Some(now);
                    let _ = self.window.drag_window();
                }
                return false;
            }
            HitRegion::ResizeN => {
                self.window.drag_resize_window(ResizeDirection::North).ok();
            }
            HitRegion::ResizeS => {
                self.window.drag_resize_window(ResizeDirection::South).ok();
            }
            HitRegion::ResizeE => {
                self.window.drag_resize_window(ResizeDirection::East).ok();
            }
            HitRegion::ResizeW => {
                self.window.drag_resize_window(ResizeDirection::West).ok();
            }
            HitRegion::ResizeNE => {
                self.window
                    .drag_resize_window(ResizeDirection::NorthEast)
                    .ok();
            }
            HitRegion::ResizeNW => {
                self.window
                    .drag_resize_window(ResizeDirection::NorthWest)
                    .ok();
            }
            HitRegion::ResizeSE => {
                self.window
                    .drag_resize_window(ResizeDirection::SouthEast)
                    .ok();
            }
            HitRegion::ResizeSW => {
                self.window
                    .drag_resize_window(ResizeDirection::SouthWest)
                    .ok();
            }
            HitRegion::Content => {}
        }
        self.last_click_time = None;
        false
    }

    /// Update stored size after a `Resized` event (physical pixels).
    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_size = (width.max(1) as f32, height.max(1) as f32);
        self.scale_factor = self.window.scale_factor();
    }
}
