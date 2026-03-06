// ─── Glass HUD: Side-Docked Tools ─────────────────────────────────
// Separates Utilities from Knowledge without visual occlusion.
//
// The Dock: Bottom-center pill with icons: [ ⌘ ] [ 📅 ] [ ✉ ] [ ⚙ ]
//
// The Side-Panel Overlay:
//   Clicking a tool (e.g., Email) slides a frosted-glass panel
//   (#111111DD, blur: 20.0) in from the RIGHT.
//   CRITICAL LIMIT: exactly 35% of screen width.
//   Left 65% remains fully visible canvas.
//
// The Bridge: Users can drag items from the 35% HUD and drop them
//   onto a Slot in the 65% Canvas → creates a NodeReference in CRDT.
// ────────────────────────────────────────────────────────────────────

use makepad_widgets::*;

// ── Tool definitions ───────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudTool {
    Command,
    Calendar,
    Email,
    Settings,
}

// ── Actions emitted by the GlassHud ────────────────────────────

#[derive(Clone, Debug, Default)]
pub enum GlassHudAction {
    /// A tool icon was clicked → toggle side panel.
    ToolActivated(HudTool),
    /// Side panel was closed.
    PanelClosed,
    /// User wants to spawn a new node.
    SpawnNode,
    /// Item dragged from HUD panel to canvas.
    BridgeDrop {
        item_id: String,
        drop_x: f32,
        drop_y: f32,
    },
    #[default]
    None,
}

impl Default for HudTool {
    fn default() -> Self {
        HudTool::Command
    }
}

// ── Makepad DSL + Widget ───────────────────────────────────────

script_mod! {
    use mod.prelude.widgets_internal.*
    use mod.widgets.*

    mod.widgets.GlassHudBase = #(GlassHud::register_widget(vm))

    mod.widgets.GlassHud = set_type_default() do mod.widgets.GlassHudBase {
        width: Fill
        height: Fill
        show_bg: false
        draw_text.text_style: theme.font_regular{font_size: 11.0}
        draw_text.color: #xD8DEE9
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct GlassHud {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[redraw]
    #[live]
    draw_bg: DrawColor,
    #[live]
    draw_text: DrawText,
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,
    #[visible]
    visible: bool,

    /// Currently active tool (if side panel is open).
    #[rust]
    active_tool: Option<HudTool>,
    /// Button rects for hit testing.
    #[rust]
    button_rects: Vec<(HudTool, Rect)>,
    /// Widget rect.
    #[rust]
    widget_rect: Rect,
    /// Side panel items (populated by tool content).
    #[rust]
    panel_items: Vec<String>,
}

impl GlassHud {
    /// Check if the side panel is currently open.
    pub fn is_panel_open(&self) -> bool {
        self.active_tool.is_some()
    }

    /// Close the side panel.
    pub fn close_panel(&mut self) {
        self.active_tool = None;
    }
}

impl Widget for GlassHud {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope) {
        if !self.visible {
            return;
        }
        let uid = self.uid;

        match event.hits(cx, self.draw_bg.area()) {
            Hit::FingerDown(fd) => {
                // Hit-test dock buttons
                for (tool, rect) in &self.button_rects {
                    if rect.contains(fd.abs) {
                        if self.active_tool == Some(*tool) {
                            // Toggle off
                            self.active_tool = None;
                            cx.widget_action(uid, GlassHudAction::PanelClosed);
                        } else {
                            // Activate tool
                            self.active_tool = Some(*tool);
                            cx.widget_action(
                                uid,
                                GlassHudAction::ToolActivated(*tool),
                            );
                        }
                        cx.redraw_all();
                        return;
                    }
                }

                // Check if click is OUTSIDE the side panel → close it
                if self.active_tool.is_some() {
                    let panel_x = self.widget_rect.size.x * 0.65;
                    if fd.abs.x < self.widget_rect.pos.x + panel_x {
                        // Click in the 65% canvas area → pass through
                        self.active_tool = None;
                        cx.widget_action(uid, GlassHudAction::PanelClosed);
                        cx.redraw_all();
                    }
                }
            }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        if !self.visible {
            return DrawStep::done();
        }

        self.draw_bg.begin(cx, walk, self.layout);
        self.widget_rect = cx.turtle().rect();
        self.button_rects.clear();

        let screen_w = self.widget_rect.size.x;
        let screen_h = self.widget_rect.size.y;

        // ── Draw Bottom-Center Pill Dock ──
        let dock_icons = [
            (HudTool::Command, "⌘"),
            (HudTool::Calendar, "📅"),
            (HudTool::Email, "✉"),
            (HudTool::Settings, "⚙"),
        ];
        let button_w = 40.0_f64;
        let button_h = 36.0_f64;
        let dock_spacing = 8.0_f64;
        let dock_padding = 16.0_f64;
        let dock_total_w = (button_w * dock_icons.len() as f64)
            + (dock_spacing * (dock_icons.len() as f64 - 1.0))
            + dock_padding * 2.0;
        let dock_x = self.widget_rect.pos.x + (screen_w - dock_total_w) * 0.5;
        let dock_y = self.widget_rect.pos.y + screen_h - button_h - dock_padding * 2.0 - 12.0;

        // Dock background pill
        let dock_rect = Rect {
            pos: DVec2 {
                x: dock_x,
                y: dock_y,
            },
            size: DVec2 {
                x: dock_total_w,
                y: button_h + dock_padding * 2.0,
            },
        };
        // Draw frosted glass pill background
        self.draw_bg.color = Vec4f {
            x: 0.067,
            y: 0.067,
            z: 0.067,
            w: 0.8, // #111111CC
        };
        self.draw_bg.draw_abs(cx, dock_rect);

        // Draw dock buttons
        let mut bx = dock_x + dock_padding;
        let by = dock_y + dock_padding;
        for (tool, icon) in &dock_icons {
            let btn_rect = Rect {
                pos: DVec2 { x: bx, y: by },
                size: DVec2 {
                    x: button_w,
                    y: button_h,
                },
            };
            self.button_rects.push((*tool, btn_rect));

            // Highlight active tool
            let is_active = self.active_tool == Some(*tool);
            let text_color = if is_active {
                Vec4 {
                    x: 0.53,
                    y: 0.75,
                    z: 0.82,
                    w: 1.0,
                } // #88C0D0
            } else {
                Vec4 {
                    x: 0.85,
                    y: 0.87,
                    z: 0.91,
                    w: 1.0,
                } // #D8DEE9
            };
            self.draw_text.color = text_color;
            self.draw_text.draw_abs(
                cx,
                DVec2 {
                    x: bx + 8.0,
                    y: by + 8.0,
                },
                icon,
            );

            bx += button_w + dock_spacing;
        }

        // ── Draw Side-Panel Overlay (35% width, right side) ──
        if let Some(tool) = self.active_tool {
            let panel_width = screen_w * 0.35; // CRITICAL: exactly 35%
            let panel_x = self.widget_rect.pos.x + screen_w - panel_width;
            let panel_y = self.widget_rect.pos.y;

            // Frosted glass background: #111111DD
            let panel_rect = Rect {
                pos: DVec2 {
                    x: panel_x,
                    y: panel_y,
                },
                size: DVec2 {
                    x: panel_width,
                    y: screen_h,
                },
            };
            self.draw_bg.color = Vec4f {
                x: 0.067,
                y: 0.067,
                z: 0.067,
                w: 0.867, // #111111DD
            };
            self.draw_bg.draw_abs(cx, panel_rect);

            // Panel header
            let header_text = match tool {
                HudTool::Command => "COMMAND PALETTE",
                HudTool::Calendar => "CALENDAR",
                HudTool::Email => "EMAIL",
                HudTool::Settings => "SETTINGS",
            };

            self.draw_text.color = Vec4 {
                x: 0.4,
                y: 0.4,
                z: 0.4,
                w: 1.0,
            };
            self.draw_text.draw_abs(
                cx,
                DVec2 {
                    x: panel_x + 24.0,
                    y: panel_y + 24.0,
                },
                header_text,
            );

            // Panel content (placeholder items)
            let tool_items: Vec<&str> = match tool {
                HudTool::Command => vec!["+ New Node", "Search...", "Import", "Export"],
                HudTool::Calendar => {
                    vec!["Today", "This Week", "Upcoming"]
                }
                HudTool::Email => vec!["Inbox (3)", "Drafts", "Sent"],
                HudTool::Settings => {
                    vec!["Profile", "Theme", "Keybindings", "About"]
                }
            };

            let mut item_y = panel_y + 60.0;
            self.draw_text.color = Vec4 {
                x: 0.75,
                y: 0.75,
                z: 0.75,
                w: 1.0,
            };
            for item in &tool_items {
                // Draw item background (hoverable card)
                let item_rect = Rect {
                    pos: DVec2 {
                        x: panel_x + 16.0,
                        y: item_y,
                    },
                    size: DVec2 {
                        x: panel_width - 32.0,
                        y: 36.0,
                    },
                };
                self.draw_bg.color = Vec4f {
                    x: 0.094,
                    y: 0.094,
                    z: 0.106,
                    w: 0.5,
                };
                self.draw_bg.draw_abs(cx, item_rect);

                self.draw_text.draw_abs(
                    cx,
                    DVec2 {
                        x: panel_x + 28.0,
                        y: item_y + 10.0,
                    },
                    item,
                );
                item_y += 44.0;
            }

            // Drag hint at bottom
            self.draw_text.color = Vec4 {
                x: 0.3,
                y: 0.3,
                z: 0.3,
                w: 1.0,
            };
            self.draw_text.draw_abs(
                cx,
                DVec2 {
                    x: panel_x + 24.0,
                    y: panel_y + screen_h - 40.0,
                },
                "Drag items to canvas →",
            );
        }

        self.draw_bg.end(cx);
        DrawStep::done()
    }
}

