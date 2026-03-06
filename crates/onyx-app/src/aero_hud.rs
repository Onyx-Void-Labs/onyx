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
// Phase 2: Translucent bar with node-spawn buttons, view toggle,
//          and cosmos status readout.
// ────────────────────────────────────────────────────────────────────

use makepad_widgets::*;

// ── Actions emitted by the Aero-HUD ────────────────────────────

#[derive(Clone, Debug, Default)]
pub enum AeroHudAction {
    /// User wants to spawn a new node (type is emergent).
    SpawnNode,
    /// Reset the cosmos to initial state.
    ResetCosmos,
    /// Purge all tombstoned nodes.
    PurgeTombstones,
    #[default]
    None,
}

script_mod! {
    use mod.prelude.widgets_internal.*
    use mod.widgets.*

    // ── AeroHud widget ──
    mod.widgets.AeroHudBase = #(AeroHud::register_widget(vm))

    mod.widgets.AeroHud = set_type_default() do mod.widgets.AeroHudBase {
        width: 400.0
        height: 60.0
        show_bg: true
        draw_bg.color: #x12121A
        align: Align{x: 0.5, y: 1.0}

        // ── Child widgets: HUD controls ──
        flow: Right
        spacing: 12
        padding: Inset{left: 20, top: 8, right: 20, bottom: 8}
        align: Center

        hud_spawn := Button {
            text: "✦ Spawn"
            width: 80
            height: 32
        }

        View { width: Fill, height: 1 }

        hud_view_toggle := Button {
            text: "⟁ Editor"
            width: 85
            height: 32
        }

        hud_delete := Button {
            text: "✕ Delete"
            width: 80
            height: 32
        }

        View { width: Fill, height: 1 }

        hud_status := Label {
            text: ""
            draw_text.color: #x5A5A7A
            draw_text.text_style.font_size: 10.0
        }
    }
}

/// The Aero-HUD widget — a translucent bottom bar that serves as the
/// primary UI surface for the Singularity Engine.
///
/// Phase 2: renders dark backdrop + spawn buttons + view toggle.
/// Uses View as deref base for child widget management.
#[derive(Script, ScriptHook, Widget)]
pub struct AeroHud {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[deref]
    view: View,
}

impl Widget for AeroHud {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}
