// --- Glowing Remote Cursor ---
// GPU-driven cursor shader for showing a friend's cursor position.
// Renders a thin vertical bar with a bloom/glow falloff and a
// gentle pulse animation.
//
// Usage:
//   1. Place RemoteCursorWidget in the widget tree (DSL).
//   2. Position it via abs_pos and toggle visibility from the App.
//   3. The shader renders a glowing bar with a pulse animation.
// ---

use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets_internal.*

    set_type_default() do #(DrawRemoteCursor::script_shader(vm)){
        ..mod.draw.DrawQuad
    }

    mod.widgets.DrawRemoteCursor = {
        cursor_color: #x7B68EE
        time: 0.0
        glow_radius: 10.0
        bar_width: 2.0

        pixel: fn() {
            let pos = self.pos * self.rect_size

            // Horizontal centre of the quad is the cursor bar
            let cx = self.rect_size.x * 0.5
            let dx = abs(pos.x - cx)

            // Main bar (hard edge)
            let bar = smoothstep(self.bar_width, 0.0, dx)

            // Glow halo (Gaussian falloff)
            let sigma = self.glow_radius
            let glow = exp(-(dx * dx) / (2.0 * sigma * sigma)) * 0.35

            // Pulse animation (subtle breathing)
            let pulse = sin(self.time * 3.14159) * 0.12 + 0.88

            let alpha = clamp((bar + glow) * pulse, 0.0, 1.0)

            // Hard discard -- kill the invisible box artifact
            if alpha < 0.01 {
                return vec4(0.0, 0.0, 0.0, 0.0)
            }

            return Pal.premul(vec4(self.cursor_color.rgb, alpha))
        }
    }

    // RemoteCursorWidget -- a minimal Widget wrapper
    mod.widgets.RemoteCursorWidgetBase = #(RemoteCursorWidget::register_widget(vm))

    mod.widgets.RemoteCursorWidget = set_type_default() do mod.widgets.RemoteCursorWidgetBase {
        width: 24
        height: 18
    }
}

#[derive(Script, ScriptHook)]
#[repr(C)]
pub struct DrawRemoteCursor {
    #[deref]
    draw_super: DrawQuad,
    #[live]
    cursor_color: Vec4,
    #[live]
    time: f32,
    #[live]
    glow_radius: f32,
    #[live]
    bar_width: f32,
}

impl DrawRemoteCursor {
    /// Draw the remote cursor at an absolute rect.
    pub fn draw_cursor(&mut self, cx: &mut Cx2d, rect: Rect, time_secs: f32) {
        self.time = time_secs;
        self.draw_abs(cx, rect);
    }
}

// Predefined peer cursor colours

/// Predefined colours for remote peer cursors.
/// Used by the App to assign distinct colours to each peer slot.
#[allow(dead_code)]
pub const PEER_CURSOR_COLORS: [(f32, f32, f32); 4] = [
    (0.93, 0.41, 0.63), // Pink
    (0.41, 0.93, 0.75), // Cyan
    (0.93, 0.80, 0.41), // Gold
    (0.41, 0.93, 0.56), // Green
];

// Widget wrapper for the DSL

/// A minimal Makepad widget that renders a DrawRemoteCursor shader.
#[derive(Script, ScriptHook, Widget)]
pub struct RemoteCursorWidget {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[redraw]
    #[live]
    draw_cursor_bg: DrawRemoteCursor,
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,
}

impl Widget for RemoteCursorWidget {
    fn handle_event(&mut self, _cx: &mut Cx, _event: &Event, _scope: &mut Scope) {}

    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        let rect = cx.walk_turtle(walk);
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f32())
            .unwrap_or(0.0);
        self.draw_cursor_bg.draw_cursor(cx, rect, time);
        DrawStep::done()
    }
}
