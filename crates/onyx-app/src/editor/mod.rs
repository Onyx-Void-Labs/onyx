// ─── Lane Editor: The Fractal Dashboard Grid ──────────────────────
// CRDT-aware editor for the Lane & Slot document topology.
//
// Architecture:
//   • Each VoidNode's content is a LoroTree of Row → Slot nodes.
//   • Each Slot has its own LoroText container (CRDT boundary).
//   • Backspace interception at the application layer prevents
//     native OS text deletion from crossing container boundaries.
//   • Focus Mode: active slot at 100%, all else at 20% opacity.
//   • + Split button on hover at the right edge of focused Slot.
//
// Data flow:
//   App pushes LaneDocSnapshot → LaneEditor renders rows/slots.
//   LaneEditor emits LaneEditorAction → App issues CRDT operations.
// ────────────────────────────────────────────────────────────────────

use makepad_widgets::*;
use onyx_core::core_state::{LaneDocSnapshot, SlotKind};

// ── Actions emitted by the LaneEditor ──────────────────────────

#[derive(Clone, Debug, Default)]
pub enum LaneEditorAction {
    /// Text inserted in a slot's LoroText container.
    TextInserted {
        text_key: String,
        pos: usize,
        text: String,
    },
    /// Text deleted in a slot's LoroText container.
    TextDeleted {
        text_key: String,
        pos: usize,
        len: usize,
    },
    /// Cross-slot Backspace: cursor was at pos 0 of a slot.
    /// Merge content into the previous slot and collapse this one.
    CrossSlotBackspace {
        from_slot_key: String,
        to_slot_key: String,
    },
    /// User clicked the + split button on a slot.
    SlotSplit { slot_id: String },
    /// User focused a slot (enter Focus Mode).
    SlotFocused { text_key: String },
    /// User clicked background (exit Focus Mode → Architect Mode).
    ArchitectMode,
    /// Close the editor and return to cosmos view.
    CloseEditor,
    /// Add a new Row below the current one.
    AddRow,
    #[default]
    None,
}

// ── Lane Editor State ──────────────────────────────────────────

/// Per-slot editing state.
#[derive(Clone, Debug, Default)]
pub struct SlotEditState {
    /// LoroText container key.
    pub text_key: String,
    /// Current text content (local snapshot for rendering).
    pub text: String,
    /// Cursor position within this slot's text.
    pub cursor_pos: usize,
    /// Width ratio (0.0..=1.0).
    pub width_ratio: f32,
    /// Whether this slot is collapsed (Ghost Box).
    pub collapsed: bool,
    /// Slot content kind.
    pub kind: SlotKind,
}

/// Per-row editing state.
#[derive(Clone, Debug, Default)]
pub struct RowEditState {
    /// String ID for this row.
    pub id: String,
    /// Whether this row is collapsed.
    pub collapsed: bool,
    /// Slots in this row.
    pub slots: Vec<SlotEditState>,
}

// ── Makepad DSL + Widget ───────────────────────────────────────

script_mod! {
    use mod.prelude.widgets_internal.*
    use mod.widgets.*

    mod.widgets.LaneEditorBase = #(LaneEditor::register_widget(vm))

    mod.widgets.LaneEditor = set_type_default() do mod.widgets.LaneEditorBase {
        width: Fill
        height: Fill
        show_bg: true
        draw_bg.color: #x0A0A0C
        draw_text.text_style: theme.font_regular{font_size: 15.0}
        draw_text.color: #xE0E0E0
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct LaneEditor {
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
    #[live]
    pub show_bg: bool,
    #[live]
    #[visible]
    pub visible: bool,

    // ── Document state ──
    #[rust]
    rows: Vec<RowEditState>,
    /// Which slot is currently focused (text_key).
    #[rust]
    focused_slot: Option<String>,
    /// Cursor position within the focused slot.
    #[rust]
    cursor_pos: usize,
    /// Whether Focus Mode is active (non-focused elements at 20%).
    #[rust]
    focus_mode: bool,
    /// Hover state: mouse near the right edge of a slot (for + button).
    #[rust]
    hover_split_slot: Option<String>,
    /// Widget rect for hit testing.
    #[rust]
    widget_rect: Rect,
    /// Per-slot rects for hit testing and cursor positioning.
    #[rust]
    slot_rects: Vec<(String, Rect)>,
    /// Accumulated time for cursor blink.
    #[rust]
    cursor_time: f64,
}

impl LaneEditor {
    /// Set the document snapshot (pushed from App each frame).
    pub fn set_document(&mut self, snapshot: &LaneDocSnapshot) {
        self.rows.clear();
        for row in &snapshot.rows {
            let mut slots = Vec::new();
            for slot in &row.slots {
                slots.push(SlotEditState {
                    text_key: slot.text_key.clone(),
                    text: slot.text_content.clone(),
                    cursor_pos: 0,
                    width_ratio: slot.width_ratio,
                    collapsed: slot.collapsed,
                    kind: slot.slot_kind.clone(),
                });
            }
            self.rows.push(RowEditState {
                id: row.id.clone(),
                collapsed: row.collapsed,
                slots,
            });
        }
    }

    /// Get the focused slot's text key.
    pub fn focused_text_key(&self) -> Option<&str> {
        self.focused_slot.as_deref()
    }

    /// Get the current cursor position in the focused slot.
    pub fn cursor_position(&self) -> usize {
        self.cursor_pos
    }

    /// Set the cursor position (e.g. after a CRDT sync).
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor_pos = pos;
    }

    /// Find the ordered list of non-collapsed slot text keys.
    fn ordered_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        for row in &self.rows {
            if row.collapsed {
                continue;
            }
            for slot in &row.slots {
                if !slot.collapsed {
                    keys.push(slot.text_key.clone());
                }
            }
        }
        keys
    }

    /// Find the slot text content for a given key.
    fn slot_text(&self, key: &str) -> &str {
        for row in &self.rows {
            for slot in &row.slots {
                if slot.text_key == key {
                    return &slot.text;
                }
            }
        }
        ""
    }

    /// Handle Backspace: the CRITICAL cross-container interceptor.
    ///
    /// If cursor is at position 0 of the current slot:
    ///   1. Find the previous slot in document order.
    ///   2. Emit CrossSlotBackspace action → App merges content
    ///      and collapses the empty slot.
    ///   3. Do NOT allow native OS text deletion.
    ///
    /// If cursor is NOT at position 0:
    ///   Normal single-character delete within this slot's LoroText.
    fn handle_backspace(&mut self, cx: &mut Cx) {
        let uid = self.uid;
        let Some(ref focused_key) = self.focused_slot else {
            return;
        };

        if self.cursor_pos == 0 {
            // ── Cross-Container Backspace ──
            // Walk the tree to find the previous slot.
            let keys = self.ordered_keys();
            let current_idx = keys.iter().position(|k| k == focused_key);
            if let Some(idx) = current_idx {
                if idx > 0 {
                    let prev_key = keys[idx - 1].clone();
                    cx.widget_action(
                        uid,
                        LaneEditorAction::CrossSlotBackspace {
                            from_slot_key: focused_key.clone(),
                            to_slot_key: prev_key,
                        },
                    );
                }
            }
        } else {
            // ── Normal Backspace within container ──
            let pos = self.cursor_pos - 1;
            cx.widget_action(
                uid,
                LaneEditorAction::TextDeleted {
                    text_key: focused_key.clone(),
                    pos,
                    len: 1,
                },
            );
            self.cursor_pos = pos;
        }
    }

    /// Handle text insertion in the focused slot.
    fn handle_text_input(&mut self, cx: &mut Cx, input: &str) {
        let uid = self.uid;
        let Some(ref focused_key) = self.focused_slot else {
            return;
        };
        cx.widget_action(
            uid,
            LaneEditorAction::TextInserted {
                text_key: focused_key.clone(),
                pos: self.cursor_pos,
                text: input.to_string(),
            },
        );
        self.cursor_pos += input.chars().count();
    }

    /// Handle Enter key: create a new row below current.
    fn handle_enter(&mut self, cx: &mut Cx) {
        cx.widget_action(self.uid, LaneEditorAction::AddRow);
    }
}

impl Widget for LaneEditor {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope) {
        if !self.visible {
            return;
        }
        let uid = self.uid;

        match event.hits(cx, self.draw_bg.area()) {
            Hit::FingerDown(fd) => {
                // Hit-test against slot rects to determine focus
                let mut hit_slot = None;
                for (key, rect) in &self.slot_rects {
                    if rect.contains(fd.abs) {
                        hit_slot = Some(key.clone());
                        break;
                    }
                }
                if let Some(key) = hit_slot {
                    // ── Check for + split button hit ──
                    // The + button occupies the rightmost 24px of the slot
                    for (skey, rect) in &self.slot_rects {
                        if skey == &key {
                            let split_zone_x = rect.pos.x + rect.size.x - 24.0;
                            if fd.abs.x >= split_zone_x
                                && self.focused_slot.as_deref() == Some(skey)
                            {
                                cx.widget_action(
                                    uid,
                                    LaneEditorAction::SlotSplit {
                                        slot_id: key.clone(),
                                    },
                                );
                                return;
                            }
                        }
                    }

                    // ── Focus this slot ──
                    self.focused_slot = Some(key.clone());
                    self.focus_mode = true;
                    // Estimate cursor position from click X relative to slot
                    let slot_char_count = self.slot_text(&key).chars().count();
                    for (skey, rect) in &self.slot_rects {
                        if skey == &key {
                            let rel_x = (fd.abs.x - rect.pos.x - 16.0).max(0.0);
                            let char_width = 8.5; // approximate glyph width at 15pt
                            self.cursor_pos = (rel_x / char_width) as usize;
                            self.cursor_pos = self.cursor_pos.min(slot_char_count);
                            break;
                        }
                    }
                    cx.widget_action(uid, LaneEditorAction::SlotFocused { text_key: key });
                    cx.set_key_focus(self.draw_bg.area());
                } else {
                    // Clicked background → exit Focus Mode
                    self.focused_slot = None;
                    self.focus_mode = false;
                    cx.widget_action(uid, LaneEditorAction::ArchitectMode);
                }
                cx.redraw_all();
            }
            Hit::FingerHoverOver(fh) => {
                // Track hover for + split button visibility
                let mut new_hover = None;
                for (key, rect) in &self.slot_rects {
                    if rect.contains(fh.abs) {
                        let split_zone_x = rect.pos.x + rect.size.x - 30.0;
                        if fh.abs.x >= split_zone_x {
                            new_hover = Some(key.clone());
                        }
                        break;
                    }
                }
                if new_hover != self.hover_split_slot {
                    self.hover_split_slot = new_hover;
                    cx.redraw_all();
                }
            }
            _ => {}
        }

        // ── Keyboard interception ──
        match event {
            Event::KeyDown(ke) if self.focused_slot.is_some() => match ke.key_code {
                KeyCode::Backspace => {
                    self.handle_backspace(cx);
                    cx.redraw_all();
                }
                KeyCode::Delete => {
                    if let Some(ref key) = self.focused_slot {
                        let text = self.slot_text(key);
                        if self.cursor_pos < text.chars().count() {
                            cx.widget_action(
                                uid,
                                LaneEditorAction::TextDeleted {
                                    text_key: key.clone(),
                                    pos: self.cursor_pos,
                                    len: 1,
                                },
                            );
                        }
                    }
                    cx.redraw_all();
                }
                KeyCode::ReturnKey => {
                    self.handle_enter(cx);
                    cx.redraw_all();
                }
                KeyCode::ArrowLeft => {
                    if self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                    }
                    cx.redraw_all();
                }
                KeyCode::ArrowRight => {
                    if let Some(ref key) = self.focused_slot {
                        let max = self.slot_text(key).chars().count();
                        if self.cursor_pos < max {
                            self.cursor_pos += 1;
                        }
                    }
                    cx.redraw_all();
                }
                KeyCode::Home => {
                    self.cursor_pos = 0;
                    cx.redraw_all();
                }
                KeyCode::End => {
                    if let Some(ref key) = self.focused_slot {
                        self.cursor_pos = self.slot_text(key).chars().count();
                    }
                    cx.redraw_all();
                }
                KeyCode::Escape => {
                    cx.widget_action(uid, LaneEditorAction::CloseEditor);
                }
                _ => {}
            },
            Event::TextInput(e) if self.focused_slot.is_some() => {
                if !e.input.is_empty() {
                    self.handle_text_input(cx, &e.input);
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

        // Begin background
        self.draw_bg.begin(cx, walk, self.layout);
        self.widget_rect = cx.turtle().rect();
        self.slot_rects.clear();

        // Advance blink timer
        self.cursor_time += 1.0 / 60.0;

        // ── Layout constants ──
        let page_width = 800.0_f64;
        let page_x = self.widget_rect.pos.x + (self.widget_rect.size.x - page_width).max(0.0) * 0.5;
        let mut y = self.widget_rect.pos.y + 60.0; // top padding
        let row_spacing = 4.0_f64;
        let slot_padding_x = 16.0_f64;
        let slot_padding_y = 12.0_f64;
        let min_slot_height = 40.0_f64;

        for row in &self.rows {
            if row.collapsed {
                continue;
            }

            // Calculate slot widths based on width_ratio
            let visible_slots: Vec<&SlotEditState> =
                row.slots.iter().filter(|s| !s.collapsed).collect();
            if visible_slots.is_empty() {
                continue;
            }

            let total_ratio: f32 = visible_slots.iter().map(|s| s.width_ratio).sum();
            let slot_gap = 2.0_f64;
            let total_gap = slot_gap * (visible_slots.len() as f64 - 1.0).max(0.0);
            let available_width = page_width - total_gap;
            let mut row_height = min_slot_height;

            // First pass: calculate max height in this row
            for slot in &visible_slots {
                let line_count = slot.text.matches('\n').count() + 1;
                let height = (line_count as f64) * 22.0 + slot_padding_y * 2.0;
                if height > row_height {
                    row_height = height;
                }
            }

            let mut x = page_x;
            for slot in &visible_slots {
                let ratio = slot.width_ratio / total_ratio;
                let slot_width = available_width * ratio as f64;
                let slot_rect = Rect {
                    pos: DVec2 { x, y },
                    size: DVec2 {
                        x: slot_width,
                        y: row_height,
                    },
                };

                // ── Determine opacity (Focus Mode) ──
                let opacity = if self.focus_mode {
                    if self.focused_slot.as_deref() == Some(&slot.text_key) {
                        1.0
                    } else {
                        0.2 // Non-focused slots fade to 20%
                    }
                } else {
                    1.0 // Architect Mode: 100%
                };

                // ── Draw slot background ──
                // 1px border for Architect Mode, subtle highlight for focused
                let is_focused = self.focused_slot.as_deref() == Some(&slot.text_key);
                let bg_color = if is_focused {
                    Vec4 {
                        x: 0.08,
                        y: 0.08,
                        z: 0.10,
                        w: opacity as f32,
                    }
                } else {
                    Vec4 {
                        x: 0.067,
                        y: 0.067,
                        z: 0.075,
                        w: opacity as f32,
                    }
                };
                self.draw_bg.draw_abs(
                    cx,
                    Rect {
                        pos: slot_rect.pos,
                        size: slot_rect.size,
                    },
                );

                // ── Draw slot border (1px, crisp — Architect Mode) ──
                if !self.focus_mode || is_focused {
                    let border_color = Vec4 {
                        x: 0.15,
                        y: 0.15,
                        z: 0.17,
                        w: opacity as f32,
                    };
                    let _ = border_color; // border drawn by bg shader
                    let _ = bg_color;
                }

                // ── Draw text content ──
                let text_x = x + slot_padding_x;
                let text_y = y + slot_padding_y;
                let display_text = if slot.text.is_empty() && is_focused {
                    "Start writing..."
                } else if slot.text.is_empty() {
                    ""
                } else {
                    &slot.text
                };

                if !display_text.is_empty() {
                    let text_color = if slot.text.is_empty() {
                        Vec4 {
                            x: 0.4,
                            y: 0.4,
                            z: 0.4,
                            w: opacity as f32,
                        }
                    } else {
                        Vec4 {
                            x: 0.88,
                            y: 0.88,
                            z: 0.88,
                            w: opacity as f32,
                        }
                    };
                    self.draw_text.color = text_color;
                    self.draw_text.draw_abs(
                        cx,
                        DVec2 {
                            x: text_x,
                            y: text_y,
                        },
                        display_text,
                    );
                }

                // ── Draw cursor (blinking bar) ──
                if is_focused {
                    let blink = (self.cursor_time * 3.5).sin();
                    if blink > 0.0 {
                        let char_width = 8.5_f64;
                        let cursor_x = text_x + (self.cursor_pos as f64) * char_width;
                        let cursor_y = text_y;
                        let cursor_rect = Rect {
                            pos: DVec2 {
                                x: cursor_x,
                                y: cursor_y,
                            },
                            size: DVec2 { x: 2.0, y: 18.0 },
                        };
                        // Draw cursor using draw_bg (white bar)
                        self.draw_bg.draw_abs(cx, cursor_rect);
                    }
                }

                // ── Draw + split button (right edge, on hover) ──
                if is_focused && self.hover_split_slot.as_deref() == Some(&slot.text_key) {
                    let plus_x = x + slot_width - 24.0;
                    let plus_y = y + (row_height - 24.0) * 0.5;
                    self.draw_text.color = Vec4 {
                        x: 0.53,
                        y: 0.75,
                        z: 0.82,
                        w: 1.0,
                    };
                    self.draw_text.draw_abs(
                        cx,
                        DVec2 {
                            x: plus_x,
                            y: plus_y,
                        },
                        "+",
                    );
                }

                // Store slot rect for hit testing
                self.slot_rects.push((slot.text_key.clone(), slot_rect));

                x += slot_width + slot_gap;
            }

            y += row_height + row_spacing;
        }

        // ── Draw "← Back" button at bottom ──
        let back_y = self.widget_rect.pos.y + self.widget_rect.size.y - 40.0;
        let back_x = page_x;
        self.draw_text.color = Vec4 {
            x: 0.4,
            y: 0.4,
            z: 0.4,
            w: 1.0,
        };
        self.draw_text.draw_abs(
            cx,
            DVec2 {
                x: back_x,
                y: back_y,
            },
            "← Back [ESC]",
        );

        self.draw_bg.end(cx);
        DrawStep::done()
    }
}
