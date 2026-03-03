// ─── Glowing Remote Cursor ──────────────────────────────────────────
// GPU-driven cursor shader for showing a friend's cursor position.
// Renders a thin vertical bar with a bloom/glow falloff and a
// gentle pulse animation.
//
// Usage:
//   1. Create a `DrawRemoteCursor` via the DSL.
//   2. Set `cursor_color` to the peer's assigned colour.
//   3. Each frame, update `time` and call `draw_abs` at the cursor rect.
// ─────────────────────────────────────────────────────────────────────

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

            // ── Main bar (hard edge) ──
            let bar = smoothstep(self.bar_width, 0.0, dx)

            // ── Glow halo (Gaussian falloff) ──
            let sigma = self.glow_radius
            let glow = exp(-(dx * dx) / (2.0 * sigma * sigma)) * 0.35

            // ── Pulse animation (subtle breathing) ──
            let pulse = sin(self.time * 3.14159) * 0.12 + 0.88

            let alpha = clamp((bar + glow) * pulse, 0.0, 1.0)

            // ── Hard discard — kill the "invisible box" artifact ──
            // When opacity is negligible, don't even attempt to draw
            // the pixel. This prevents the R8 SDF logic from lifting
            // a near-zero alpha into a visible block.
            if alpha < 0.01 {
                return vec4(0.0, 0.0, 0.0, 0.0)
            }

            return Pal.premul(vec4(self.cursor_color.rgb, alpha))
        }
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
    /// `rect` should be a tall, narrow rect (e.g. 24×line_height) centred on
    /// the peer's cursor x-position.
    pub fn draw_cursor(&mut self, cx: &mut Cx2d, rect: Rect, time_secs: f32) {
        self.time = time_secs;
        self.draw_abs(cx, rect);
    }
}
