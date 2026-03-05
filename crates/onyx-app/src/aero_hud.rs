// ─── Aero-HUD ──────────────────────────────────────────────────────
// The floating bottom bar of the Singularity Engine.
//
// Replaces all traditional ribbons, toolbars, and menus with a
// translucent, context-aware heads-up display that fades in on hover
// and houses:
//   • The Chronos Slider (temporal Z-axis scrubber)
//   • Lens Switcher (constellation / planet / satellite views)
//   • Quick-capture asteroid launcher
//
// Phase 1: Basic translucent bar with a gradient background.
//          Functional controls are added in subsequent phases.
// ────────────────────────────────────────────────────────────────────

use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets_internal.*

    // ── DrawAeroHud: translucent gradient background ──
    set_type_default() do #(DrawAeroHud::script_shader(vm)){
        ..mod.draw.DrawQuad
    }

    mod.widgets.DrawAeroHud = {
        hud_color: #x7B68EE
        opacity: 0.85

        pixel: fn() {
            let pos = self.pos

            // Vertical gradient: transparent at top, solid at bottom
            let grad = smoothstep(0.0, 0.6, pos.y)

            // Base colour with gradient alpha
            let alpha = grad * self.opacity

            // Subtle edge glow at the top
            let edge_glow = exp(-pos.y * 8.0) * 0.15

            let r = self.hud_color.r * 0.15 + edge_glow
            let g = self.hud_color.g * 0.15 + edge_glow
            let b = self.hud_color.b * 0.15 + edge_glow

            return Pal.premul(vec4(r, g, b, alpha))
        }
    }

    // ── AeroHud widget ──
    mod.widgets.AeroHudBase = #(AeroHud::register_widget(vm))

    mod.widgets.AeroHud = set_type_default() do mod.widgets.AeroHudBase {
        width: Fill
        height: 48
    }
}

#[derive(Script, ScriptHook)]
#[repr(C)]
pub struct DrawAeroHud {
    #[deref]
    draw_super: DrawQuad,
    #[live]
    hud_color: Vec4,
    #[live]
    opacity: f32,
}

/// The Aero-HUD widget — a translucent bottom bar that serves as the
/// primary UI surface for the Singularity Engine.
///
/// Phase 1: renders a gradient backdrop.  Subsequent phases add
/// the Chronos Slider, Lens Switcher, and Asteroid Launcher.
#[derive(Script, ScriptHook, Widget)]
pub struct AeroHud {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[redraw]
    #[live]
    draw_bg: DrawAeroHud,
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,
}

impl Widget for AeroHud {
    fn handle_event(&mut self, _cx: &mut Cx, _event: &Event, _scope: &mut Scope) {
        // Phase 1: no interactive controls yet.
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        let rect = cx.walk_turtle(walk);
        self.draw_bg.draw_abs(cx, rect);
        DrawStep::done()
    }
}
