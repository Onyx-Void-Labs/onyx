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
    /// User double-clicked on a node — "dive" into it.
    NodeDoubleClicked(usize),
    /// User started dragging a node.
    NodeDragStart(usize),
    /// User is dragging — world-space coordinates.
    NodeDragging { x: f32, y: f32 },
    /// User released the drag.
    NodeDragEnd,
    /// Mouse is hovering over a node (for hover arrest).
    NodeHovered(usize),
    /// Mouse left a node (clear hover arrest).
    NodeUnhovered,
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

    // DrawNodeBody: SDF circle shader with atmospheric effects (extends DrawQuad)
    set_type_default() do #(DrawNodeBody::script_shader(vm)){
        ..mod.draw.DrawQuad

        node_color: #x7B68EE
        heat: 1.0
        selected: 0.0
        node_type_id: 0.0
        zoom_level: 1.0

        pixel: fn() {
            let uv = self.pos * 2.0 - 1.0
            let dist = length(uv)

            // ── Early discard: anything beyond the bounding circle ──
            // Forces GPU Early-Z rejection — saves massive fill-rate.
            if dist > 1.05 {
                return vec4(0.0, 0.0, 0.0, 0.0)
            }

            // ── SDF anti-aliased circle (perfectly smooth edges) ──
            let edge_width = 0.02 + 0.5 / max(self.zoom_level, 0.1)
            let body = 1.0 - smoothstep(0.85 - edge_width, 0.85, dist)

            // ── LOD: simplified path for tiny on-screen nodes ──
            if self.zoom_level < 0.3 {
                // Ultra-simplified: just a colored circle, no glow
                let alpha = body
                if alpha < 0.005 { return vec4(0.0, 0.0, 0.0, 0.0) }
                let r = self.node_color.r * 0.8
                let g = self.node_color.g * 0.8
                let b = self.node_color.b * 0.8
                return Pal.premul(vec4(r, g, b, alpha))
            }

            // ── Fresnel rim-lighting (atmospheric glow) ──
            let fresnel = pow(smoothstep(0.45, 0.90, dist), 2.5)
            let rim_strength = fresnel * (0.5 + self.heat * 0.5)

            // ── Atmospheric halo (soft outer glow) ──
            let glow_intensity = self.heat * 0.35 + 0.08
            let glow = exp(-(dist - 0.75) * (dist - 0.75) * 10.0) * glow_intensity

            // ── Node-type specific effects ──
            // Use `var` (mutable) since these are reassigned per node-type.
            var type_mod_r = 0.0
            var type_mod_g = 0.0
            var type_mod_b = 0.0
            var type_alpha_mod = 0.0

            // Planet (0): gas-giant fresnel atmosphere
            if self.node_type_id < 0.5 {
                type_mod_r = rim_strength * 0.6
                type_mod_g = rim_strength * 0.3
                type_mod_b = rim_strength * 0.9
                type_alpha_mod = glow * 1.5
            }

            // Asteroid (1): darker, sharper, slightly translucent
            if self.node_type_id > 0.5 && self.node_type_id < 1.5 {
                let body2 = 1.0 - smoothstep(0.75, 0.78, dist)
                let body_val = body2 * 0.75
                let alpha2 = clamp(body_val + glow * 0.3, 0.0, 0.9)
                if alpha2 < 0.005 { return vec4(0.0, 0.0, 0.0, 0.0) }
                let r = self.node_color.r * 0.55
                let g = self.node_color.g * 0.55
                let b = self.node_color.b * 0.60
                return Pal.premul(vec4(r, g, b, alpha2))
            }

            // Satellite (2): bright tiny glowing star
            if self.node_type_id > 1.5 && self.node_type_id < 2.5 {
                let star_core = 1.0 - smoothstep(0.0, 0.5, dist)
                let star_flare = exp(-dist * dist * 4.0) * 1.2
                let star_alpha = clamp(star_core + star_flare + glow * 2.0, 0.0, 1.0)
                if star_alpha < 0.005 { return vec4(0.0, 0.0, 0.0, 0.0) }
                let brightness = 0.9 + self.heat * 0.1
                let r = self.node_color.r * brightness + star_flare * 0.4
                let g = self.node_color.g * brightness + star_flare * 0.3
                let b = self.node_color.b * brightness + star_flare * 0.2
                return Pal.premul(vec4(r, g, b, star_alpha))
            }

            // DysonSphere (3): golden shell with inner ring
            if self.node_type_id > 2.5 {
                let shell_ring = smoothstep(0.03, 0.01, abs(dist - 0.7))
                type_mod_r = rim_strength * 0.9 + shell_ring * 0.4
                type_mod_g = rim_strength * 0.7 + shell_ring * 0.3
                type_mod_b = rim_strength * 0.1
                type_alpha_mod = glow * 1.2 + shell_ring * 0.5
            }

            // ── Selection ring ──
            let ring_dist = abs(dist - 0.92)
            let ring = smoothstep(0.05, 0.015, ring_dist) * self.selected

            let alpha = clamp(body + glow + ring + type_alpha_mod, 0.0, 1.0)
            if alpha < 0.005 {
                return vec4(0.0, 0.0, 0.0, 0.0)
            }

            // ── Final colour: base tint + atmospheric modulation ──
            let brightness = 0.55 + self.heat * 0.45
            let r = self.node_color.r * brightness + type_mod_r + ring * 0.3
            let g = self.node_color.g * brightness + type_mod_g + ring * 0.3
            let b = self.node_color.b * brightness + type_mod_b + ring * 0.3

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
    #[live]
    node_type_id: f32,
    #[live]
    zoom_level: f32,
}

// ── Colour palette for node types ───────────────────────────────

fn node_type_color(nt: onyx_core::void_node::NodeType) -> Vec4 {
    use onyx_core::void_node::NodeType;
    match nt {
        NodeType::Planet => Vec4 {
            x: 0.48,
            y: 0.41,
            z: 0.93,
            w: 1.0,
        }, // Purple
        NodeType::Asteroid => Vec4 {
            x: 0.55,
            y: 0.65,
            z: 0.75,
            w: 1.0,
        }, // Blue-grey
        NodeType::Satellite => Vec4 {
            x: 0.93,
            y: 0.60,
            z: 0.30,
            w: 1.0,
        }, // Amber
        NodeType::DysonSphere => Vec4 {
            x: 0.93,
            y: 0.80,
            z: 0.20,
            w: 1.0,
        }, // Gold
    }
}

/// Map NodeType to a float ID for the shader.
/// Planet=0, Asteroid=1, Satellite=2, DysonSphere=3.
fn node_type_to_id(nt: onyx_core::void_node::NodeType) -> f32 {
    use onyx_core::void_node::NodeType;
    match nt {
        NodeType::Planet => 0.0,
        NodeType::Asteroid => 1.0,
        NodeType::Satellite => 2.0,
        NodeType::DysonSphere => 3.0,
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
    /// Index of the currently hovered node (for hover arrest).
    #[rust]
    hovered_node: Option<usize>,
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
                // Hit-test in screen-space so the leniency is in pixels,
                // not world units (which change with zoom).
                let mut hit_idx = None;
                for (i, nd) in self.draw_data.iter().enumerate().rev() {
                    let (sx, sy) = self.world_to_screen(nd.x, nd.y);
                    let dx = fd.abs.x - sx;
                    let dy = fd.abs.y - sy;
                    // Screen radius of the drawn circle + 10px flat leniency
                    let screen_r = (nd.radius as f64) * self.cam_zoom + 10.0;
                    if dx * dx + dy * dy <= screen_r * screen_r {
                        hit_idx = Some(i);
                        break;
                    }
                }

                if let Some(idx) = hit_idx {
                    // Use Makepad's native tap_count for double-click detection
                    if fd.tap_count == 2 {
                        cx.widget_action(uid, CosmosViewAction::NodeDoubleClicked(idx));
                    } else {
                        cx.widget_action(uid, CosmosViewAction::NodeClicked(idx));
                        cx.widget_action(uid, CosmosViewAction::NodeDragStart(idx));
                    }
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
                    cx.widget_action(
                        uid,
                        CosmosViewAction::CameraPanned {
                            x: new_cx as f32,
                            y: new_cy as f32,
                        },
                    );
                } else {
                    // Node dragging
                    let (wx, wy) = self.screen_to_world(fm.abs.x, fm.abs.y);
                    cx.widget_action(
                        uid,
                        CosmosViewAction::NodeDragging {
                            x: wx as f32,
                            y: wy as f32,
                        },
                    );
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
            Hit::FingerHoverOver(fh) => {
                // Hover detection in screen-space (same formula as hit-test)
                let mut hover_idx = None;
                for (i, nd) in self.draw_data.iter().enumerate().rev() {
                    let (sx, sy) = self.world_to_screen(nd.x, nd.y);
                    let dx = fh.abs.x - sx;
                    let dy = fh.abs.y - sy;
                    let screen_r = (nd.radius as f64) * self.cam_zoom + 10.0;
                    if dx * dx + dy * dy <= screen_r * screen_r {
                        hover_idx = Some(i);
                        break;
                    }
                }
                if hover_idx != self.hovered_node {
                    if let Some(idx) = hover_idx {
                        cx.widget_action(uid, CosmosViewAction::NodeHovered(idx));
                    } else {
                        cx.widget_action(uid, CosmosViewAction::NodeUnhovered);
                    }
                    self.hovered_node = hover_idx;
                }
            }
            Hit::FingerHoverOut(_) => {
                if self.hovered_node.is_some() {
                    cx.widget_action(uid, CosmosViewAction::NodeUnhovered);
                    self.hovered_node = None;
                }
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

            // Screen diameter: mass × zoom, clamped to minimum 8px.
            // This ensures planets grow dramatically when zooming in,
            // and never vanish when zooming out.
            let screen_diameter = ((nd.radius as f64 * 2.0) * self.cam_zoom).max(8.0);
            let screen_radius = screen_diameter * 0.5;

            // Cull nodes outside the viewport (with margin for glow)
            let margin = screen_diameter;
            if screen_x + margin < rect.pos.x
                || screen_x - margin > rect.pos.x + rect.size.x
                || screen_y + margin < rect.pos.y
                || screen_y - margin > rect.pos.y + rect.size.y
            {
                continue;
            }

            // Set shader uniforms
            self.draw_node.node_color = node_type_color(nd.node_type);
            self.draw_node.heat = nd.heat;
            self.draw_node.selected = if nd.selected { 1.0 } else { 0.0 };
            self.draw_node.node_type_id = node_type_to_id(nd.node_type);
            self.draw_node.zoom_level = self.cam_zoom as f32;

            // Draw the node body — quad centered on screen position,
            // sized exactly to screen_diameter so shader SDF fills it.
            let node_rect = Rect {
                pos: DVec2 {
                    x: screen_x - screen_radius,
                    y: screen_y - screen_radius,
                },
                size: DVec2 {
                    x: screen_diameter,
                    y: screen_diameter,
                },
            };
            self.draw_node.draw_abs(cx, node_rect);

            // Draw the label below the node
            self.draw_text.draw_abs(
                cx,
                dvec2(
                    screen_x - screen_radius * 0.5,
                    screen_y + screen_radius + 4.0,
                ),
                nd.label,
            );
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
