// ─── Makepad Application Shell ─────────────────────────────────────
// Root Makepad app. Owns the widget tree and the Tokio runtime
// (for async DB operations).
//
// Architecture:
//   OnyxApp
//     └─ OnyxEditor (custom widget — defined inline for Phase 1)
//           ├── EditorBuffer   (onyx-editor crate, Rope-backed)
//           ├── Cursor          (onyx-editor crate)
//           ├── MathRenderer   (onyx-math crate, stub)
//           └── Store / CrdtDoc (onyx-store, spawned on Tokio)
// ────────────────────────────────────────────────────────────────────

use makepad_widgets::*;

// TODO: Font atlas memory optimization (1% win):
//  • switch cosmic-text to SDF or alpha-only glyph cache
//  • initial atlas 1024×1024, expand sparsely
//  • the actual implementation must be in makepad-widgets. the AI snippet
//    would look roughly like:
//
//      let atlas_desc = wgpu::TextureDescriptor {
//          format: wgpu::TextureFormat::R8Unorm, // one byte per texel
//          size: wgpu::Extent3d { width: 1024, height: 1024, depth_or_array_layers: 1 },
//          ..Default::default()
//      };
//      // shader sample: vec4(color.r,0,0,1)
//
//    Changing `draw_text` shader to read `.r` channel is sufficient to cut
//    font memory by ~75%.
//
// TODO: GPU power management for low-RAM mode:
//  • prefer integrated/low-power adapter
//  • set swap chain count to 2 (double buffering)
//  • present mode to Fifo (vs Immediate)
//  • apply downlevel_limits to device descriptor
//
//    Example wgpu configuration (to be placed inside Makepad's platform code):
//
//      let adapter_opts = wgpu::RequestAdapterOptions {
//          power_preference: wgpu::PowerPreference::LowPower,
//          ..Default::default()
//      };
//      let sc_desc = wgpu::SwapChainDescriptor {
//          present_mode: wgpu::PresentMode::Fifo,
//          usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
//          width: size.width,
//          height: size.height,
//          format: surface_format,
//          // note: wgpu doesn't expose `max_back_buffer_count` directly; makepad
//          // would need to call `device.create_swap_chain` with `1` or 2.
//      };
//      let limits = wgpu::Limits::downlevel_defaults();
//      let desc = wgpu::DeviceDescriptor { limits, ..Default::default() };
//
//  These changes should trim ~16 MB from the back buffer and eliminate the
//  enormous driver heap allocations on low-end systems.

// ─── DSL: The Void UI ────────────────────────────────────────────

live_design! {
    use link::theme::*;
    use link::widgets::*;

    // ── Color palette ──
    VOID_BG      = #0A0A0F
    VOID_SURFACE = #12121A
    VOID_TEXT    = #E0E0E8
    VOID_ACCENT  = #7B68EE  // Medium Slate Blue
    VOID_DIM     = #4A4A5A

    // ── Root application ──
    App = {{App}} {
        ui: <Window> {
            window: { inner_size: vec2(1280, 800) },
            show_bg: true,
            draw_bg: { color: (VOID_BG) }

            body = <View> {
                flow: Down
                padding: { left: 0, top: 0, right: 0, bottom: 0 }
                spacing: 0

                // ── Title bar ──
                <View> {
                    width: Fill
                    height: 48
                    show_bg: true
                    draw_bg: { color: (VOID_SURFACE) }
                    padding: { left: 20, top: 12 }

                    <Label> {
                        text: "ONYX VOID"
                        draw_text: {
                            color: (VOID_ACCENT)
                            text_style: { font_size: 14.0 }
                        }
                    }
                }

                // ── Editor area ──
                <View> {
                    width: Fill
                    height: Fill
                    padding: { left: 40, top: 30, right: 40, bottom: 30 }

                    editor_label = <Label> {
                        text: "Welcome to the Void.\n\nThis is Phase 1 — The Foundation.\nThe editor widget will render here.\n\nPress any key to begin."
                        draw_text: {
                            color: (VOID_TEXT)
                            text_style: { font_size: 13.0 }
                        }
                    }
                }

                // ── Status bar ──
                <View> {
                    width: Fill
                    height: 28
                    show_bg: true
                    draw_bg: { color: (VOID_SURFACE) }
                    padding: { left: 20, top: 6 }

                    <Label> {
                        text: "Phase 2 · Telepathic Link · 144Hz · P2P Sync"
                        draw_text: {
                            color: (VOID_DIM)
                            text_style: { font_size: 10.0 }
                        }
                    }
                }
            }
        }
    }
}

// ─── App struct ──────────────────────────────────────────────────

app_main!(App);

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        makepad_widgets::live_design(cx);
    }
}

impl MatchEvent for App {
    fn handle_actions(&mut self, _cx: &mut Cx, _actions: &Actions) {
        // Phase 1: action handling will be wired here
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
