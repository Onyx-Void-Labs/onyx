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

    // 1. The Glass Dock
    CommandDock = <View> {
        width: Fit, height: Fit
        flow: Right, spacing: 10.0, padding: {top: 10.0, bottom: 10.0, left: 20.0, right: 20.0}
        align: {x: 0.5, y: 0.5}
        show_bg: true
        draw_bg: {
            color: #18181bC0
            radius: 20.0
            border_width: 1.0
            border_color: #27272a
        }

        <Button> { text: "✦ WRITE", draw_text: { text_style: {font_size: 11.0} } }
        <Button> { text: "▨ PAINT", draw_text: { text_style: {font_size: 11.0} } }
        <Button> { text: "✉ MAIL", draw_text: { text_style: {font_size: 11.0} } }
        <Button> { text: "⚙ SETTINGS", draw_text: { text_style: {font_size: 11.0} } }
    }

    // 2. The Path Bar (Breadcrumbs)
    PathBar = <View> {
        width: Fill, height: Fit
        flow: Right, spacing: 5.0, padding: {left: 30.0, top: 20.0, bottom: 20.0}

        <Label> { text: "Root", draw_text: { color: #a1a1aa, text_style: {font_size: 12.0} } }
        <Label> { text: "/", draw_text: { color: #3f3f46, text_style: {font_size: 12.0} } }
        <Label> { text: "Workspace", draw_text: { color: #f4f4f5, text_style: {font_size: 12.0} } }
    }

    App = {{App}} {
        ui: <Window> {
            window: { inner_size: vec2(1280, 800) },
            pass: { clear_color: #09090b },

            body = <View> {
                width: Fill, height: Fill
                flow: Overlay

                // MAIN CONTENT LAYER
                <View> {
                    width: Fill, height: Fill
                    flow: Down

                    <PathBar> {}

                    // The Canvas / Editor Void
                    <View> {
                        width: Fill, height: Fill
                        align: {x: 0.5, y: 0.5}
                        <Label> {
                            text: "[ Empty Slot ]"
                            draw_text: { color: #27272a, text_style: {font_size: 16.0} }
                        }
                    }
                }

                // HUD LAYER (Floating at bottom)
                <View> {
                    width: Fill, height: Fill
                    align: {x: 0.5, y: 1.0}
                    padding: {bottom: 30.0}
                    <CommandDock> {}
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
