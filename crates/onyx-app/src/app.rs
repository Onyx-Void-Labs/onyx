// ─── Onyx Void — Application (Vello + Winit + LoroTree UI) ─────────

use std::sync::Arc;

use onyx_core::document::OnyxWorkspace;
use onyx_core::model::NodeType;
use parley::layout::{Alignment, AlignmentOptions, PositionedLayoutItem};
use parley::style::StyleProperty;
use parley::{FontContext, LayoutContext};
use vello::kurbo::{Affine, Point, Rect, RoundedRect};
use vello::peniko::{Brush, Color, Fill};
use vello::util::{RenderContext, RenderSurface};
use vello::{AaConfig, Renderer, RendererOptions, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

const RIBBON_H: f64 = 44.0;
const FONT_SIZE: f32 = 14.0;
const LINE_H: f64 = 22.0;
const LEFT_PAD: f64 = 20.0;
const INDENT: f64 = 24.0;

fn void_btn() -> Rect {
    Rect::new(10.0, 6.0, 140.0, 38.0)
}
fn note_btn() -> Rect {
    Rect::new(150.0, 6.0, 280.0, 38.0)
}

fn bg_color() -> Color {
    Color::from_rgba8(24, 24, 28, 255)
}
fn ribbon_bg() -> Color {
    Color::from_rgba8(32, 32, 38, 255)
}
fn btn_color() -> Color {
    Color::from_rgba8(50, 50, 60, 255)
}
fn btn_hover() -> Color {
    Color::from_rgba8(70, 70, 85, 255)
}
fn text_color() -> Color {
    Color::from_rgba8(220, 220, 230, 255)
}
fn void_color() -> Color {
    Color::from_rgba8(130, 170, 255, 255)
}
fn note_color() -> Color {
    Color::from_rgba8(180, 220, 140, 255)
}
fn muted_color() -> Color {
    Color::from_rgba8(100, 100, 110, 255)
}

pub struct OnyxApp {
    render_cx: RenderContext,
    renderer: Option<Renderer>,
    surface: Option<RenderSurface<'static>>,
    window: Option<Arc<Window>>,
    scene: Scene,
    font_cx: FontContext,
    layout_cx: LayoutContext<Brush>,
    pub workspace: OnyxWorkspace,
    cursor_pos: (f64, f64),
    void_counter: u32,
    note_counter: u32,
}

impl OnyxApp {
    pub fn new() -> Self {
        Self {
            render_cx: RenderContext::new(),
            renderer: None,
            surface: None,
            window: None,
            scene: Scene::new(),
            font_cx: FontContext::default(),
            layout_cx: LayoutContext::new(),
            workspace: OnyxWorkspace::new(),
            cursor_pos: (0.0, 0.0),
            void_counter: 0,
            note_counter: 0,
        }
    }

    fn handle_click(&mut self) {
        let pt = Point::new(self.cursor_pos.0, self.cursor_pos.1);

        if void_btn().contains(pt) {
            self.void_counter += 1;
            let title = format!("Void {}", self.void_counter);
            self.workspace.create_void(None, &title);
            tracing::info!("Created: {}", title);
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        } else if note_btn().contains(pt) {
            if let Some(parent_id) = self.workspace.first_void_id() {
                self.note_counter += 1;
                let title = format!("Note {}", self.note_counter);
                self.workspace.create_note(&parent_id, &title);
                tracing::info!("Created: {}", title);
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            } else {
                tracing::warn!("No void exists — create a Void first");
            }
        }
    }

    pub fn draw(&mut self) {
        // Get dimensions (early return if no surface)
        let (width, height) = match self.surface.as_ref() {
            Some(s) => (s.config.width, s.config.height),
            None => return,
        };
        if self.renderer.is_none() {
            return;
        }

        // ── Phase 1: Build scene ──────────────────────────────────
        self.scene.reset();

        // Ribbon background
        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            ribbon_bg(),
            None,
            &Rect::new(0.0, 0.0, width as f64, RIBBON_H),
        );

        // Buttons
        let pt = Point::new(self.cursor_pos.0, self.cursor_pos.1);
        let vb = void_btn();
        let nb = note_btn();

        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            if vb.contains(pt) {
                btn_hover()
            } else {
                btn_color()
            },
            None,
            &RoundedRect::from_rect(vb, 4.0),
        );
        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            if nb.contains(pt) {
                btn_hover()
            } else {
                btn_color()
            },
            None,
            &RoundedRect::from_rect(nb, 4.0),
        );

        // Button labels
        draw_text(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            "+ New Void",
            vb.x0 + 10.0,
            vb.y0 + 6.0,
            FONT_SIZE,
            text_color(),
        );
        draw_text(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            "+ New Note",
            nb.x0 + 10.0,
            nb.y0 + 6.0,
            FONT_SIZE,
            text_color(),
        );

        // Tree hierarchy
        let nodes = self.workspace.get_tree_nodes();
        let mut y = RIBBON_H + 20.0;
        for (node, depth) in &nodes {
            let x = LEFT_PAD + (*depth as f64) * INDENT;
            let color = match node.node_type {
                NodeType::Void => void_color(),
                NodeType::Note => note_color(),
            };
            let prefix = match node.node_type {
                NodeType::Void => "\u{25B8} ",
                NodeType::Note => "  \u{25AA} ",
            };
            let label = format!("{}{}", prefix, node.title);
            draw_text(
                &mut self.scene,
                &mut self.font_cx,
                &mut self.layout_cx,
                &label,
                x,
                y,
                FONT_SIZE,
                color,
            );
            y += LINE_H;
        }

        if nodes.is_empty() {
            draw_text(
                &mut self.scene,
                &mut self.font_cx,
                &mut self.layout_cx,
                "Click [+ New Void] to create your first void",
                LEFT_PAD,
                RIBBON_H + 40.0,
                FONT_SIZE,
                muted_color(),
            );
        }

        // ── Phase 2: Render to surface ────────────────────────────
        let surface = self.surface.as_mut().unwrap();
        let renderer = self.renderer.as_mut().unwrap();
        let device = &self.render_cx.devices[surface.dev_id];

        let render_params = vello::RenderParams {
            base_color: bg_color(),
            width,
            height,
            antialiasing_method: AaConfig::Area,
        };

        renderer
            .render_to_texture(
                &device.device,
                &device.queue,
                &self.scene,
                &surface.target_view,
                &render_params,
            )
            .expect("render to texture");

        let surface_texture = surface
            .surface
            .get_current_texture()
            .expect("get surface texture");

        let target_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("blit"),
            });

        surface.blitter.copy(
            &device.device,
            &mut encoder,
            &surface.target_view,
            &target_view,
        );

        device.queue.submit([encoder.finish()]);
        surface_texture.present();
    }
}

impl ApplicationHandler for OnyxApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("Onyx Void")
            .with_inner_size(LogicalSize::new(1280.0, 800.0));

        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));

        let surface = pollster::block_on(self.render_cx.create_surface(
            window.clone(),
            window.inner_size().width,
            window.inner_size().height,
            wgpu::PresentMode::AutoVsync,
        ))
        .expect("create surface");

        let device = &self.render_cx.devices[surface.dev_id];
        self.renderer = Some(
            Renderer::new(
                &device.device,
                RendererOptions {
                    use_cpu: false,
                    antialiasing_support: vello::AaSupport::area_only(),
                    num_init_threads: None,
                    pipeline_cache: None,
                },
            )
            .expect("create renderer"),
        );

        self.surface = Some(surface);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    if let Some(surface) = &mut self.surface {
                        self.render_cx
                            .resize_surface(surface, size.width, size.height);
                    }
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = (position.x, position.y);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.handle_click();
            }
            WindowEvent::RedrawRequested => {
                self.draw();
            }
            _ => {}
        }
    }
}

// ─── Text rendering (free function to avoid borrow conflicts) ──────

fn draw_text(
    scene: &mut Scene,
    font_cx: &mut FontContext,
    layout_cx: &mut LayoutContext<Brush>,
    text: &str,
    x: f64,
    y: f64,
    size: f32,
    color: Color,
) {
    let brush = Brush::Solid(color);
    let mut builder = layout_cx.ranged_builder(font_cx, text, 1.0, false);
    builder.push_default(StyleProperty::FontSize(size));
    builder.push_default(StyleProperty::Brush(brush.clone()));
    let mut layout = builder.build(text);
    layout.break_all_lines(None);
    layout.align(None, Alignment::Start, AlignmentOptions::default());

    let transform = Affine::translate((x, y));

    for line in layout.lines() {
        for item in line.items() {
            if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                let run = glyph_run.run();
                let font = run.font();
                let font_size = run.font_size();
                let synthesis = run.synthesis();
                let glyph_xform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));

                let xform = match glyph_xform {
                    Some(gx) => transform * gx,
                    None => transform,
                };

                scene
                    .draw_glyphs(font)
                    .font_size(font_size)
                    .transform(xform)
                    .brush(&brush)
                    .draw(
                        Fill::NonZero,
                        glyph_run.glyphs().map(|g| vello::Glyph {
                            id: g.id as u32,
                            x: g.x,
                            y: g.y,
                        }),
                    );
            }
        }
    }
}
