// --- Makepad Application Shell ---
// Minimal live_design! skeleton - proves upstream Makepad compiles.
// Backend modules (Cosmos, Physics, CRDT, Net) are preserved but
// not wired to the UI until the rendering layer is rebuilt.

use makepad_widgets::*;

// -- Live Design --

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    App = {{App}} {
        ui: <Window> {
            window: { inner_size: vec2(1280, 800) },
            pass: { clear_color: #09090b },
            body = <View> {
                width: Fill, height: Fill,
                flow: Down,
                align: { x: 0.5, y: 0.5 },

                <Label> {
                    text: "ONYX VOID: CORE RESTORED"
                    draw_text: {
                        color: #88C0D0,
                        text_style: { font_size: 24.0 }
                    }
                }

                <Label> {
                    text: "live_design! migration complete"
                    draw_text: {
                        color: #4C566A,
                        text_style: { font_size: 12.0 }
                    }
                }
            }
        }
    }
}

// -- App Struct --

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

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        self.ui.handle_event(cx, event, &mut Scope::empty());
    }
}
