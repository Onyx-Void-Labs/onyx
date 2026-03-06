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

    // 1. Custom Button — flat matte SDF, no default Makepad chrome
    OnyxButton = <View> {
        width: Fit, height: Fit
        padding: {left: 12.0, right: 12.0, top: 8.0, bottom: 8.0}
        align: {x: 0.5, y: 0.5}
        show_bg: true

        draw_bg: {
            instance hover: 0.0
            instance down: 0.0
            instance radius: 6.0
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.radius);
                // Mix default -> hover -> down
                let bg_color = mix(#18181b, mix(#27272a, #3f3f46, self.down), self.hover);
                sdf.fill_keep(bg_color);
                return sdf.result;
            }
        }

        animator: {
            hover = {
                default: off
                off = { from: {all: Forward {duration: 0.1}} apply: {draw_bg: {hover: 0.0}} }
                on = { from: {all: Snap} apply: {draw_bg: {hover: 1.0}} }
            }
            down = {
                default: off
                off = { from: {all: Forward {duration: 0.1}} apply: {draw_bg: {down: 0.0}} }
                on = { from: {all: Snap} apply: {draw_bg: {down: 1.0}} }
            }
        }

        label = <Label> {
            draw_text: { color: #a1a1aa, text_style: {font_size: 11.0} }
        }
    }

    // 2. The Glass Dock — SDF pill shader with border stroke
    CommandDock = <View> {
        width: Fit, height: Fit
        flow: Right, spacing: 10.0, padding: 8.0
        align: {x: 0.5, y: 0.5}
        show_bg: true

        draw_bg: {
            instance radius: 20.0
            instance border_width: 1.0
            instance border_color: #27272a
            color: #09090bE6

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                // Offset by 1.0px to prevent anti-alias clipping on the outer stroke
                sdf.box(
                    1.0, 1.0,
                    self.rect_size.x - 2.0,
                    self.rect_size.y - 2.0,
                    self.radius
                );
                sdf.fill_keep(self.color);
                sdf.stroke(self.border_color, self.border_width);
                return sdf.result;
            }
        }

        <OnyxButton> { label = { text: "✦ WRITE" } }
        <OnyxButton> { label = { text: "▨ PAINT" } }
        <OnyxButton> { label = { text: "✉ MAIL" } }
        <OnyxButton> { label = { text: "⚙ SETTINGS" } }
    }

    // 3. The Path Bar (Breadcrumbs)
    PathBar = <View> {
        width: Fill, height: Fit
        flow: Right, spacing: 5.0, padding: {left: 30.0, top: 20.0, bottom: 20.0}

        <Label> { text: "Root", draw_text: { color: #a1a1aa, text_style: {font_size: 12.0} } }
        <Label> { text: "/", draw_text: { color: #3f3f46, text_style: {font_size: 12.0} } }
        <Label> { text: "Workspace", draw_text: { color: #f4f4f5, text_style: {font_size: 12.0} } }
    }

    // 4. A single document slot — transparent by default, SDF border in paint mode
    OnyxSlot = <View> {
        width: Fill, height: Fit
        padding: 15.0
        show_bg: true
        draw_bg: {
            color: #00000000
            instance border_color: #27272a
            instance border_width: 1.0
            instance is_painting: 0.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, 4.0);
                sdf.stroke(self.border_color, self.border_width * self.is_painting);
                return sdf.result;
            }
        }
        <Label> { text: "Type here...", draw_text: { color: #52525b, text_style: {font_size: 14.0} } }
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

                    // The Document Canvas
                    <View> {
                        width: Fill, height: Fill
                        align: {x: 0.5, y: 0.0}
                        padding: {top: 40.0, bottom: 100.0}

                        // The Document Column (max width for readability)
                        <View> {
                            width: 850.0, height: Fit
                            flow: Down, spacing: 10.0

                            // Row 1 (Title)
                            <View> {
                                width: Fill, height: Fit
                                padding: {bottom: 20.0}
                                <Label> { text: "MATH2411: Calculus Matrix", draw_text: { color: #f4f4f5, text_style: {font_size: 28.0} } }
                            }

                            // Row 2 (Content)
                            <View> {
                                width: Fill, height: Fit
                                flow: Right, spacing: 15.0
                                <OnyxSlot> {}
                            }
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
