// ─── Path Bar: Fractal Navigation ──────────────────────────────────
// Replaces sidebars with a dynamic breadcrumb trail.
//
// Normal state: Top-left horizontal layout: Root > Workspace > Current
//
// Expand-on-Drag mechanic:
//   When the user initiates a Drag on any Canvas Node or Slot,
//   the Path Bar transforms into a Full Vertical Ancestor Panel.
//   Every ancestor becomes a 40px-tall hoverable drop zone.
//   On Drop or MouseUp, execute LoroTree::move() to the target
//   ancestor and collapse back to a horizontal breadcrumb.
// ────────────────────────────────────────────────────────────────────

use makepad_widgets::*;

// ── Actions emitted by the PathBar ─────────────────────────────

#[derive(Clone, Debug, Default)]
pub enum PathBarAction {
    /// User clicked a breadcrumb segment to navigate.
    Navigate { depth: usize },
    /// User dropped a dragged item onto an ancestor drop zone.
    DropOnAncestor { ancestor_id: String },
    #[default]
    None,
}

// ── Breadcrumb data ────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct BreadcrumbSegment {
    pub label: String,
    pub id: String,
}

// ── Makepad DSL + Widget ───────────────────────────────────────

script_mod! {
    use mod.prelude.widgets_internal.*
    use mod.widgets.*

    mod.widgets.PathBarBase = #(PathBar::register_widget(vm))

    mod.widgets.PathBar = set_type_default() do mod.widgets.PathBarBase {
        width: Fit
        height: Fit
        show_bg: true
        draw_bg.color: #x0A0A0C00
        draw_text.text_style: theme.font_regular{font_size: 11.0}
        draw_text.color: #x666666
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct PathBar {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[redraw]
    #[live]
    draw_bg: DrawQuad,
    #[live]
    draw_text: DrawText,
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,
    #[visible]
    visible: bool,

    /// Breadcrumb segments (Root → current).
    #[rust]
    segments: Vec<BreadcrumbSegment>,
    /// Whether the panel is in expanded (vertical drop-zone) mode.
    #[rust]
    expanded: bool,
    /// Per-segment rects for hit testing.
    #[rust]
    segment_rects: Vec<(String, Rect)>,
    /// Widget rect.
    #[rust]
    widget_rect: Rect,
}

impl PathBar {
    /// Set the breadcrumb path.
    pub fn set_path(&mut self, segments: Vec<BreadcrumbSegment>) {
        self.segments = segments;
    }

    /// Expand into vertical drop-zone mode (called when drag starts).
    pub fn expand(&mut self) {
        self.expanded = true;
    }

    /// Collapse back to horizontal breadcrumb (called on drop/mouse-up).
    pub fn collapse(&mut self) {
        self.expanded = false;
    }
}

impl Widget for PathBar {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope) {
        if !self.visible {
            return;
        }
        let uid = self.uid;

        match event.hits(cx, self.draw_bg.area()) {
            Hit::FingerDown(fd) => {
                // Hit-test breadcrumb segments
                for (i, (id, rect)) in self.segment_rects.iter().enumerate() {
                    if rect.contains(fd.abs) {
                        if self.expanded {
                            // Drop on ancestor
                            cx.widget_action(
                                uid,
                                PathBarAction::DropOnAncestor {
                                    ancestor_id: id.clone(),
                                },
                            );
                            self.expanded = false;
                        } else {
                            // Navigate to breadcrumb
                            cx.widget_action(
                                uid,
                                PathBarAction::Navigate { depth: i },
                            );
                        }
                        cx.redraw_all();
                        return;
                    }
                }
            }
            Hit::FingerUp(_) => {
                if self.expanded {
                    self.expanded = false;
                    cx.redraw_all();
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
        self.segment_rects.clear();

        let start_x = self.widget_rect.pos.x + 16.0;
        let start_y = self.widget_rect.pos.y + 8.0;

        if self.expanded {
            // ── Expanded: Full Vertical Ancestor Panel ──
            // Each ancestor is a 40px-tall hoverable drop zone.
            let mut y = start_y;
            let panel_width = 260.0_f64;

            // Draw panel background
            let panel_rect = Rect {
                pos: DVec2 {
                    x: start_x - 8.0,
                    y: start_y - 4.0,
                },
                size: DVec2 {
                    x: panel_width + 16.0,
                    y: (self.segments.len() as f64) * 40.0 + 8.0,
                },
            };
            self.draw_bg.draw_abs(cx, panel_rect);

            for seg in &self.segments {
                let seg_rect = Rect {
                    pos: DVec2 { x: start_x, y },
                    size: DVec2 {
                        x: panel_width,
                        y: 40.0,
                    },
                };
                self.segment_rects.push((seg.id.clone(), seg_rect));

                // Draw drop zone background
                self.draw_bg.draw_abs(cx, seg_rect);

                // Draw label
                self.draw_text.color = Vec4 {
                    x: 0.85,
                    y: 0.85,
                    z: 0.85,
                    w: 1.0,
                };
                self.draw_text.draw_abs(
                    cx,
                    DVec2 {
                        x: start_x + 12.0,
                        y: y + 12.0,
                    },
                    &seg.label,
                );

                y += 40.0;
            }
        } else {
            // ── Collapsed: Horizontal Breadcrumb Trail ──
            let mut x = start_x;
            let separator = " › ";

            for (i, seg) in self.segments.iter().enumerate() {
                // Draw separator
                if i > 0 {
                    self.draw_text.color = Vec4 {
                        x: 0.3,
                        y: 0.3,
                        z: 0.3,
                        w: 1.0,
                    };
                    self.draw_text.draw_abs(
                        cx,
                        DVec2 { x, y: start_y },
                        separator,
                    );
                    x += 24.0; // approx separator width
                }

                // Determine color (last segment is bright, others muted)
                let is_current = i == self.segments.len() - 1;
                let color = if is_current {
                    Vec4 {
                        x: 0.85,
                        y: 0.85,
                        z: 0.85,
                        w: 1.0,
                    }
                } else {
                    Vec4 {
                        x: 0.4,
                        y: 0.4,
                        z: 0.4,
                        w: 1.0,
                    }
                };

                self.draw_text.color = color;
                let label_width = (seg.label.len() as f64) * 7.0; // approx
                let seg_rect = Rect {
                    pos: DVec2 { x, y: start_y },
                    size: DVec2 {
                        x: label_width,
                        y: 20.0,
                    },
                };
                self.segment_rects.push((seg.id.clone(), seg_rect));

                self.draw_text.draw_abs(
                    cx,
                    DVec2 { x, y: start_y },
                    &seg.label,
                );

                x += label_width + 4.0;
            }
        }

        self.draw_bg.end(cx);
        DrawStep::done()
    }
}

