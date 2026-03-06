// ─── Makepad Integration Stub ─────────────────────────────────────
// Demonstrates how the Phase 4 PhysicsEngine is driven by the
// Makepad event loop at display refresh rate (target: 144Hz).
//
// This is a reference stub — the actual integration lives in app.rs
// where the physics bridge feeds updated positions into CosmosView.
//
// Integration Pattern:
//
//   ┌─────────────────────────────────────────────────┐
//   │  Timer fires (1/144s = ~6.9ms)                  │
//   │  ├── physics_engine.tick(&mut nodes, dt)         │
//   │  ├── Build Vec<NodeDrawData> from positions      │
//   │  ├── cosmos_view.set_draw_data(draw_data)        │
//   │  └── cx.redraw_all()   ← drives next frame      │
//   └─────────────────────────────────────────────────┘
//
// Makepad DSL Reminder:
//   Use raw brackets ONLY — never type names in live_design!
//
//   live_design! {
//       CosmosView = {{CosmosView}} {
//           width: Fill,
//           height: Fill,
//           margin: {top: 0.0, right: 0.0, bottom: 0.0, left: 0.0},
//           padding: {top: 5.0},
//           flow: Overlay,
//       }
//   }
//
//   NEVER:  margin: Inset { top: 5.0 }
//   ALWAYS: margin: {top: 5.0}
// ─────────────────────────────────────────────────────────────────────

use makepad_widgets::*;
use onyx_core::core_state::VoidNode;
use onyx_core::stellar_physics::PhysicsEngine;

/// Bridge between the stateless PhysicsEngine and the Makepad event loop.
///
/// Owns the `PhysicsEngine` and the canonical `Vec<VoidNode>` for
/// Phase 4 Stellar Dynamics.  The 144Hz `Timer` fires, ticks physics,
/// and requests a redraw so CosmosView picks up the new positions.
pub struct PhysicsBridge {
    pub engine: PhysicsEngine,
    pub nodes: Vec<VoidNode>,
    timer: Timer,
}

impl PhysicsBridge {
    pub fn new() -> Self {
        Self {
            engine: PhysicsEngine::new(),
            nodes: Vec::new(),
            timer: Timer::default(),
        }
    }

    /// Start the 144Hz physics loop.  Call once during `App::run()`.
    pub fn start(&mut self, cx: &mut Cx) {
        self.timer = cx.start_interval(1.0 / 144.0);
    }

    /// Process Makepad events.  Call from `App::handle_event()`.
    ///
    /// Returns `true` if a physics tick occurred and CosmosView
    /// should rebuild its draw data from `self.nodes`.
    pub fn handle_event(&mut self, cx: &mut Cx, event: &Event) -> bool {
        if self.timer.is_event(event).is_some() {
            let dt: f32 = 1.0 / 144.0;
            self.engine.tick(&mut self.nodes, dt, 1280.0, 800.0);

            // Drive the next frame — Makepad redraws all widgets,
            // CosmosView picks up updated positions from draw_data.
            cx.redraw_all();
            return true;
        }
        false
    }
}
