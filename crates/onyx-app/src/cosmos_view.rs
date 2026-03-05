use onyx_core::void_node::NodeType;
// ─── CosmosView: The Spatial Canvas ────────────────────────────────
// A zoomable, pannable Makepad widget that renders VoidNodes as
// glowing bodies in 2D space.
//
// The widget:
//   • Draws each VoidNode as a circle with type-dependent colour
//     and heat-dependent glow intensity.
//   • Supports camera pan (drag on empty space) and zoom (scroll).
//   • Reports hit events (click on node, drag node) back to the App
//     via WidgetActions.
//
// Data flow:
//   App owns Cosmos → calls cosmos_view.set_draw_data() before draw
//   → CosmosView renders nodes using DrawNodeBody shader instances.
//
// Phase 2: 2D rendering.  Z-axis / 3D perspective comes later.
// ────────────────────────────────────────────────────────────────────

use makepad_widgets::*;

// ── Draw data passed from App → CosmosView each frame ───────────

/// Lightweight snapshot of one node for rendering.
#[derive(Clone, Debug)]
pub struct NodeDrawData {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub heat: f32,
    pub node_type: onyx_core::void_node::NodeType,
    pub selected: bool,
    pub label: &'static str,
}

// ── Widget actions emitted by CosmosView ────────────────────────

#[derive(Clone, Debug, Default)]
pub enum CosmosViewAction {
    /// User clicked on empty space — deselect.
    Deselect,
    /// User clicked on a node.
    NodeClicked(usize),
    /// User started dragging a node.
    NodeDragStart(usize),
    /// User is dragging — world-space coordinates.
    NodeDragging { x: f32, y: f32 },
    /// User released the drag.
    NodeDragEnd,
    /// Camera panned — new camera x, y.
    CameraPanned { x: f32, y: f32 },
    /// Camera zoomed — new zoom level.
    CameraZoomed(f32),
    #[default]
    None,
}

// ── Makepad shader + widget DSL ─────────────────────────────────

script_mod! {
    use mod.prelude.widgets_internal.*

    // DrawNodeBody: a circle shader with glow (extends DrawQuad)
    set_type_default() do #(DrawNodeBody::script_shader(vm)){
        ..mod.draw.DrawQuad

        node_color: #x7B68EE
        heat: 1.0
        selected: 0.0

        pixel: fn() {
            let uv = self.pos * 2.0 - 1.0
            let dist = length(uv)

            // Circle body (anti-aliased edge)
            let body = 1.0 - smoothstep(0.75, 0.85, dist)

            // Glow halo (heat-driven intensity)
            let glow_intensity = self.heat * 0.4 + 0.1
            let glow = exp(-(dist - 0.7) * (dist - 0.7) * 8.0) * glow_intensity

            // Selection ring
            let ring_dist = abs(dist - 0.9)
            let ring = smoothstep(0.06, 0.02, ring_dist) * self.selected

            let alpha = clamp(body + glow + ring, 0.0, 1.0)

            if alpha < 0.005 {
                return vec4(0.0, 0.0, 0.0, 0.0)
            }

            // Color: base node_color, brightened by heat
            let brightness = 0.6 + self.heat * 0.4
            let r = self.node_color.r * brightness + ring * 0.3
            let g = self.node_color.g * brightness + ring * 0.3
            let b = self.node_color.b * brightness + ring * 0.3

            return Pal.premul(vec4(r, g, b, alpha))
        }
    }

    // CosmosView widget base
    mod.widgets.CosmosViewBase = #(CosmosView::register_widget(vm))

    mod.widgets.CosmosView = set_type_default() do mod.widgets.CosmosViewBase {
        width: Fill
        height: Fill
        draw_bg.color: #x0A0A14
        draw_text.text_style: theme.font_regular{font_size: 9.0}
        draw_text.color: #xCCCCDD
    }
}

// ── DrawNodeBody ────────────────────────────────────────────────

#[derive(Script, ScriptHook)]
#[repr(C)]
pub struct DrawNodeBody {
    #[deref]
    draw_super: DrawQuad,
    #[live]
    node_color: Vec4,
    #[live]
    heat: f32,
    #[live]
    selected: f32,
}

// ── Colour palette for node types ───────────────────────────────

fn node_type_color(nt: onyx_core::void_node::NodeType) -> Vec4 {
    use onyx_core::void_node::NodeType;
    match nt {
        NodeType::Planet      => Vec4 { x: 0.48, y: 0.41, z: 0.93, w: 1.0 }, // Purple
        NodeType::Asteroid    => Vec4 { x: 0.55, y: 0.65, z: 0.75, w: 1.0 }, // Blue-grey
        NodeType::Satellite   => Vec4 { x: 0.93, y: 0.60, z: 0.30, w: 1.0 }, // Amber
        NodeType::DysonSphere => Vec4 { x: 0.93, y: 0.80, z: 0.20, w: 1.0 }, // Gold
    }
}

// ── CosmosView Widget ───────────────────────────────────────────

#[derive(Script, ScriptHook, Widget)]
pub struct CosmosView {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[redraw]
    #[live]
    draw_bg: DrawQuad,
    #[live]
    draw_node: DrawNodeBody,
    #[live]
    draw_text: DrawText,
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,
    /// Visibility flag — toggled by the App via set_visible.
    #[visible]
    visible: bool,

    // ── Camera state (screen-space ↔ world-space transform) ──
    // Using f64 to match Makepad's coordinate system (DVec2/Rect).
    #[rust]
    cam_x: f64,
    #[rust]
    cam_y: f64,
    #[rust]
    cam_zoom: f64,

    // ── Draw data (set each frame by the App) ──
    #[rust]
    draw_data: Vec<NodeDrawData>,

    // ── Interaction state ──
    #[rust]
    pan_start: Option<(f64, f64, f64, f64)>, // (mouse_x, mouse_y, cam_x, cam_y)
    #[rust]
    widget_rect: Rect,
}

impl CosmosView {
    /// Set the node draw data for the current frame.
    /// Called by the App before redraw.
    pub fn set_draw_data(&mut self, data: Vec<NodeDrawData>) {
        self.draw_data = data;
    }

    /// Set camera position.
    pub fn set_camera(&mut self, x: f32, y: f32, zoom: f32) {
        self.cam_x = x as f64;
        self.cam_y = y as f64;
        self.cam_zoom = (zoom as f64).max(0.1);
    }

    /// Convert screen-space coordinates to world-space.
    /// Accepts f64 (from Makepad events), returns f64 for internal math.
    fn screen_to_world(&self, sx: f64, sy: f64) -> (f64, f64) {
        let cx = self.widget_rect.pos.x + self.widget_rect.size.x * 0.5;
        let cy = self.widget_rect.pos.y + self.widget_rect.size.y * 0.5;
        let wx = (sx - cx) / self.cam_zoom + self.cam_x;
        let wy = (sy - cy) / self.cam_zoom + self.cam_y;
        (wx, wy)
    }

    /// Convert world-space coordinates to screen-space.
    /// Accepts f32 node positions, returns f64 screen coordinates.
    fn world_to_screen(&self, wx: f32, wy: f32) -> (f64, f64) {
        let cx = self.widget_rect.pos.x + self.widget_rect.size.x * 0.5;
        let cy = self.widget_rect.pos.y + self.widget_rect.size.y * 0.5;
        let sx = (wx as f64 - self.cam_x) * self.cam_zoom + cx;
        let sy = (wy as f64 - self.cam_y) * self.cam_zoom + cy;
        (sx, sy)
    }
}

impl Widget for CosmosView {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope) {
        let uid = self.uid;

        match event.hits(cx, self.draw_bg.area()) {
            Hit::FingerDown(fd) => {
                // Check if we hit a node
                let (wx, wy) = self.screen_to_world(fd.abs.x, fd.abs.y);
                let mut hit_idx = None;
                for (i, nd) in self.draw_data.iter().enumerate().rev() {
                    let dx = wx - nd.x as f64;
                    let dy = wy - nd.y as f64;
                    if dx * dx + dy * dy <= (nd.radius as f64) * (nd.radius as f64) {
                        hit_idx = Some(i);
                        break;
                    }
                }

                if let Some(idx) = hit_idx {
                    cx.widget_action(uid, CosmosViewAction::NodeClicked(idx));
                    cx.widget_action(uid, CosmosViewAction::NodeDragStart(idx));
                } else {
                    // Start camera pan
                    self.pan_start = Some((fd.abs.x, fd.abs.y, self.cam_x, self.cam_y));
                    cx.widget_action(uid, CosmosViewAction::Deselect);
                }
            }
            Hit::FingerMove(fm) => {
                if let Some((sx, sy, cx_start, cy_start)) = self.pan_start {
                    // Camera pan
                    let new_cx = cx_start - (fm.abs.x - sx) / self.cam_zoom;
                    let new_cy = cy_start - (fm.abs.y - sy) / self.cam_zoom;
                    self.cam_x = new_cx;
                    self.cam_y = new_cy;
                    cx.widget_action(uid, CosmosViewAction::CameraPanned {
                        x: new_cx as f32,
                        y: new_cy as f32,
                    });
                } else {
                    // Node dragging
                    let (wx, wy) = self.screen_to_world(fm.abs.x, fm.abs.y);
                    cx.widget_action(uid, CosmosViewAction::NodeDragging { x: wx as f32, y: wy as f32 });
                }
                cx.redraw_all();
            }
            Hit::FingerUp(_) => {
                if self.pan_start.is_some() {
                    self.pan_start = None;
                } else {
                    cx.widget_action(uid, CosmosViewAction::NodeDragEnd);
                }
            }
            Hit::FingerScroll(fs) => {
                // Zoom towards mouse position
                let zoom_factor = if fs.scroll.y > 0.0 { 1.1 } else { 0.9 };
                let old_zoom = self.cam_zoom;
                self.cam_zoom = (self.cam_zoom * zoom_factor).clamp(0.1, 5.0);

                // Adjust cam position to zoom toward the mouse cursor
                let (wx, wy) = self.screen_to_world(fs.abs.x, fs.abs.y);
                self.cam_x = wx - (wx - self.cam_x) * old_zoom / self.cam_zoom;
                self.cam_y = wy - (wy - self.cam_y) * old_zoom / self.cam_zoom;

                cx.widget_action(uid, CosmosViewAction::CameraZoomed(self.cam_zoom as f32));
                cx.redraw_all();
            }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        // Skip drawing entirely when hidden
        if !self.visible {
            return DrawStep::done();
        }

        // Get our allocated rect
        let rect = cx.walk_turtle(walk);
        self.widget_rect = rect;

        // Ensure zoom is initialized
        if self.cam_zoom < 0.01 {
            self.cam_zoom = 1.0;
        }

        // Draw background (deep space)
        self.draw_bg.draw_abs(cx, rect);

        // Draw each node using world_to_screen mapping
        for nd in &self.draw_data {
            // Skip nodes with NaN positions to prevent layout crashes
            if nd.x.is_nan() || nd.y.is_nan() {
                continue;
            }
            let (screen_x, screen_y) = self.world_to_screen(nd.x, nd.y);

            // Set explicit draw sizes for node types
            let draw_size = match nd.node_type {
                NodeType::Asteroid => 40.0 * self.cam_zoom,
                NodeType::Planet => 100.0 * self.cam_zoom,
                _ => nd.radius as f64 * self.cam_zoom * 2.5,
            };

            // Cull nodes outside the viewport (with margin for glow)
            let margin = draw_size * 2.0;
            if screen_x + margin < rect.pos.x || screen_x - margin > rect.pos.x + rect.size.x
                || screen_y + margin < rect.pos.y || screen_y - margin > rect.pos.y + rect.size.y
            {
                continue;
            }

            // Set shader uniforms
            self.draw_node.node_color = node_type_color(nd.node_type);
            self.draw_node.heat = nd.heat;
            self.draw_node.selected = if nd.selected { 1.0 } else { 0.0 };

            // Draw the node body (quad sized to encompass circle + glow)
            let node_rect = Rect {
                pos: DVec2 {
                    x: screen_x - draw_size * 0.5,
                    y: screen_y - draw_size * 0.5,
                },
                size: DVec2 {
                    x: draw_size,
                    y: draw_size,
                },
            };
            self.draw_node.draw_abs(cx, node_rect);

            // Draw the label below the node
            self.draw_text.draw_abs(cx, dvec2(
                screen_x - draw_size * 0.25,
                screen_y + draw_size * 0.5 + 4.0,
            ), nd.label);
        }

        DrawStep::done()
    }
}

impl CosmosViewRef {
    pub fn set_draw_data(&self, data: Vec<NodeDrawData>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_draw_data(data);
        }
    }

    pub fn set_camera(&self, x: f32, y: f32, zoom: f32) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_camera(x, y, zoom);
        }
    }

    pub fn set_visible(&self, cx: &mut Cx, visible: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_visible(cx, visible);
        }
    }
}
