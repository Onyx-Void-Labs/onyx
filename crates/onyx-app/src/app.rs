// ─── Onyx Void — Application ─────────

use anyhow::Context;
use std::sync::mpsc;
use std::sync::Arc;

use crate::editor_renderer::EditorRenderer;
use onyx_core::document::OnyxWorkspace;
use parley::layout::{Alignment, AlignmentOptions};
use parley::style::StyleProperty;
use parley::{FontContext, LayoutContext};
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Brush, Color};
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu;
use vello::{Renderer, RendererOptions, Scene};

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

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
    parley_vello::render_text(scene, transform, &layout);
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
    pub editor: EditorRenderer,
    cursor_pos: (f64, f64),
    selected_node_id: Option<String>,

    // --- The Live Typing Engine ---
    pub live_text_buffer: String,

    #[allow(dead_code)]
    embed_tx: mpsc::Sender<(String, String)>,
    embed_rx: mpsc::Receiver<(String, Vec<f32>)>,
    pub is_architect_mode: bool,
    pub scene_dirty: bool,
    pub cached_target: Option<wgpu::Texture>,
    pub blitter: Option<wgpu::util::TextureBlitter>,
    pub ribbon_hitboxes: std::collections::HashMap<String, Rect>,
}

impl OnyxApp {
    pub fn new() -> anyhow::Result<Self> {
        // Channels: main → AI thread (embed requests), AI thread → main (results)
        let (embed_tx, work_rx) = mpsc::channel::<(String, String)>();
        let (result_tx, embed_rx) = mpsc::channel::<(String, Vec<f32>)>();

        // Spawn AI background thread; propagate failure so caller can decide.
        let _ = std::thread::Builder::new()
            .name("onyx-ai".into())
            .spawn(move || {
                tracing::info!("🧠 AI thread: loading SemanticEngine…");
                let engine = match onyx_core::neural::SemanticEngine::load() {
                    Ok(e) => {
                        tracing::info!("🧠 AI thread: engine ready");
                        e
                    }
                    Err(err) => {
                        tracing::error!("🧠 AI thread: failed to load engine: {err:#}");
                        return;
                    }
                };

                while let Ok((id, text)) = work_rx.recv() {
                    match engine.embed_text(&text) {
                        Ok(vec) => {
                            println!(
                                "🧠 AI: Embedded note '{}' -> [Vector Size: {}]",
                                id,
                                vec.len()
                            );
                            let _ = result_tx.send((id, vec));
                        }
                        Err(err) => {
                            tracing::warn!("🧠 AI: embed failed for {id}: {err:#}");
                        }
                    }
                }
            })
            .context("spawn AI thread")?;

        // Scoped font context: load exactly one font instead of all OS fonts.
        let mut font_cx = FontContext::default();
        let font_data = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
        font_cx
            .collection
            .register_fonts(font_data.to_vec().into(), None);

        let workspace = OnyxWorkspace::new();
        let all_notes = workspace.all_note_ids();
        tracing::info!("[DEBUG] all_note_ids: {:?}", all_notes);
        let initial_note_id = all_notes.first().cloned();
        tracing::info!("[DEBUG] initial_note_id: {:?}", initial_note_id);

        Ok(Self {
            render_cx: RenderContext::new(),
            renderer: None,
            surface: None,
            window: None,
            scene: Scene::new(),
            font_cx,
            layout_cx: LayoutContext::new(),
            workspace,
            cursor_pos: (0.0, 0.0),
            selected_node_id: initial_note_id,
            live_text_buffer: String::new(),
            editor: EditorRenderer::new(),
            embed_tx,
            embed_rx,
            is_architect_mode: false,
            scene_dirty: true,
            cached_target: None,
            blitter: None,
            ribbon_hitboxes: std::collections::HashMap::new(),
        })
    }

    fn handle_click(&mut self) {
        let pt = vello::kurbo::Point::new(self.cursor_pos.0, self.cursor_pos.1);
        
        if let Some(rect) = self.ribbon_hitboxes.get("btn_write") {
            if rect.contains(pt) && self.is_architect_mode {
                self.is_architect_mode = false;
                self.scene_dirty = true;
            }
        }
        if let Some(rect) = self.ribbon_hitboxes.get("btn_canvas") {
            if rect.contains(pt) && !self.is_architect_mode {
                self.is_architect_mode = true;
                self.scene_dirty = true;
            }
        }
        if let Some(w) = &self.window { w.request_redraw(); }
    }

    fn draw_header(&mut self, width: f64) {
        use vello::kurbo::{Rect, RoundedRect, Affine};
        use vello::peniko::{Brush, Color, Fill};

        // 1% OVERKILL: The Omnipresent Command Header
        let header_bg = Brush::Solid(Color::from_rgba8(18, 18, 22, 250));
        let border = Brush::Solid(Color::from_rgba8(40, 40, 45, 255));
        
        self.scene.fill(Fill::NonZero, Affine::IDENTITY, &header_bg, None, &Rect::new(0.0, 0.0, width, 52.0));
        self.scene.fill(Fill::NonZero, Affine::IDENTITY, &border, None, &Rect::new(0.0, 51.0, width, 52.0));

        // Breadcrumbs
        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "🌌 Onyx Void   /   Untitled Note", 24.0, 16.0, 14.0, Color::from_rgba8(150, 150, 160, 255));

        // Affine-Style Mode Toggle (Center)
        let toggle_w = 220.0;
        let tx = (width / 2.0) - (toggle_w / 2.0);
        let pill_bg = Brush::Solid(Color::from_rgba8(10, 10, 12, 255));
        let active_bg = Brush::Solid(Color::from_rgba8(60, 60, 70, 255));
        let text_active = Color::from_rgba8(255, 255, 255, 255);
        let text_dim = Color::from_rgba8(120, 120, 130, 255);

        self.scene.fill(Fill::NonZero, Affine::IDENTITY, &pill_bg, None, &RoundedRect::new(tx, 8.0, tx + toggle_w, 44.0, 8.0));
        self.ribbon_hitboxes.clear();

        // Write Mode Button
        let write_active = !self.is_architect_mode;
        if write_active {
            self.scene.fill(Fill::NonZero, Affine::IDENTITY, &active_bg, None, &RoundedRect::new(tx + 4.0, 12.0, tx + (toggle_w/2.0) - 2.0, 40.0, 6.0));
        }
        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "📄 Write", tx + 24.0, 18.0, 14.0, if write_active { text_active } else { text_dim });
        self.ribbon_hitboxes.insert("btn_write".into(), Rect::new(tx, 8.0, tx + (toggle_w/2.0), 44.0));

        // Canvas Mode Button
        let canvas_active = self.is_architect_mode;
        if canvas_active {
            self.scene.fill(Fill::NonZero, Affine::IDENTITY, &active_bg, None, &RoundedRect::new(tx + (toggle_w/2.0) + 2.0, 12.0, tx + toggle_w - 4.0, 40.0, 6.0));
        }
        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "🗺️ Canvas", tx + 124.0, 18.0, 14.0, if canvas_active { text_active } else { text_dim });
        self.ribbon_hitboxes.insert("btn_canvas".into(), Rect::new(tx + (toggle_w/2.0), 8.0, tx + toggle_w, 44.0));
    }

    pub fn draw(&mut self) {
        // 1% OVERKILL: Zero-CPU-idle enforcement
        if !self.scene_dirty {
            return;
        }

        // drain embedding results first
        while let Ok((note_id, vec)) = self.embed_rx.try_recv() {
            let _ = self.workspace.set_vector(&note_id, &vec);
        }

        let surface_ref = match self.surface.as_mut() {
            Some(s) => s,
            None => return,
        };
        let device_id = surface_ref.dev_id;

        let output = match surface_ref.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let surface_format = surface_ref.config.format;
        let width = surface_ref.config.width;
        let height = surface_ref.config.height;

        // 1% OVERKILL: Manage intermediate Rgba8Unorm target for Vello 0.7.0 compositing
        let needs_rebuild = self
            .cached_target
            .as_ref()
            .map_or(true, |t| t.width() != width || t.height() != height);
        if needs_rebuild {
            // borrow device only inside scope
            let device = &self.render_cx.devices[device_id];
            self.cached_target = Some(device.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Vello Intermediate Target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }));
            // recreate blitter for the current swapchain format
            self.blitter = Some(wgpu::util::TextureBlitter::new(
                &device.device,
                surface_format,
            ));
        }

        // 1% OVERKILL: Purge previous frame geometry
        self.scene.reset();

        let width_f = width as f64;
        let height_f = height as f64;

        // 1% OVERKILL: The Blueprint Grid
        if self.is_architect_mode {
            let grid_size = 40.0;
            let stroke = vello::kurbo::Stroke::new(1.0);
            let brush =
                vello::peniko::Brush::Solid(vello::peniko::Color::from_rgba8(255, 255, 255, 8)); // 3% white blueprint

            let mut x = 0.0;
            while x < width_f {
                self.scene.stroke(
                    &stroke,
                    vello::kurbo::Affine::IDENTITY,
                    &brush,
                    None,
                    &vello::kurbo::Line::new((x, 0.0), (x, height_f)),
                );
                x += grid_size;
            }
            let mut y = 0.0;
            while y < height_f {
                self.scene.stroke(
                    &stroke,
                    vello::kurbo::Affine::IDENTITY,
                    &brush,
                    None,
                    &vello::kurbo::Line::new((0.0, y), (width_f, y)),
                );
                y += grid_size;
            }
        }

        // Rebuild the CRDT/UI layout mapping
        self.draw_header(width_f);
        if let Some(note_id) = &self.selected_node_id {
            // Pass the live typing buffer down to the GPU renderer
            self.editor.build_scene(
                &mut self.scene,
                &self.workspace,
                note_id,
                width_f,
                80.0,
                &self.live_text_buffer,
            );
        }

        if let Some(renderer) = self.renderer.as_mut() {
            let device = &self.render_cx.devices[device_id];
            if let Some(tex) = &self.cached_target {
                let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
                renderer
                    .render_to_texture(
                        &device.device,
                        &device.queue,
                        &self.scene,
                        &view,
                        &vello::RenderParams {
                            base_color: vello::peniko::Color::from_rgba8(9, 9, 11, 255),
                            width,
                            height,
                            antialiasing_method: vello::AaConfig::Area,
                        },
                    )
                    .expect("CRITICAL: Failed to rasterize Vello scene");
            }

            // now blit the RGBA result to the swapchain's BGRA texture
            if let (Some(blitter), Some(src_tex)) =
                (self.blitter.as_ref(), self.cached_target.as_ref())
            {
                let mut encoder =
                    device
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("blit encoder"),
                        });
                let src_view = src_tex.create_view(&wgpu::TextureViewDescriptor::default());
                let dst_view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                blitter.copy(&device.device, &mut encoder, &src_view, &dst_view);
                device.queue.submit(Some(encoder.finish()));
            }
        }

        output.present();
        self.scene_dirty = false;
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

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(err) => {
                eprintln!("CRITICAL: failed to create window: {err}");
                event_loop.exit();
                return;
            }
        };

        let surface = match pollster::block_on(self.render_cx.create_surface(
            window.clone(),
            window.inner_size().width,
            window.inner_size().height,
            wgpu::PresentMode::AutoVsync,
        )) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("CRITICAL: create surface failed: {err}");
                event_loop.exit();
                return;
            }
        };

        let device = &self.render_cx.devices[surface.dev_id]; // Immutable borrow only

        // NOTE: the vello version in this workspace (0.7.x) does not expose a
        // `surface_format` option yet, so we can’t perform the explicit format
        // binding described in the original patch.  We keep the CPU renderer and
        // include the pipeline cache field to satisfy the struct requirements.
        match Renderer::new(
            &device.device,
            RendererOptions {
                use_cpu: true,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: None,
                pipeline_cache: None,
            },
        ) {
            Ok(r) => self.renderer = Some(r),
            Err(err) => {
                eprintln!("CRITICAL: create renderer failed: {err}");
                event_loop.exit();
                return;
            }
        }

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
                    self.scene_dirty = true;
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = (position.x, position.y);
                self.scene_dirty = true;
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
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    let mut text_changed = false;

                    match &event.logical_key {
                        Key::Character(s) => {
                            if s == "/" {
                                self.is_architect_mode = !self.is_architect_mode;
                                self.scene_dirty = true;
                            } else {
                                self.live_text_buffer.push_str(s);
                                text_changed = true;
                            }
                        }
                        Key::Named(NamedKey::Space) => {
                            self.live_text_buffer.push(' ');
                            text_changed = true;
                        }
                        Key::Named(NamedKey::Backspace) => {
                            self.live_text_buffer.pop();
                            text_changed = true;
                        }
                        Key::Named(NamedKey::Enter) => {
                            self.live_text_buffer.push('\n');
                            text_changed = true;
                        }
                        _ => {}
                    }

                    if text_changed {
                        self.scene_dirty = true;
                        if let Some(w) = &self.window {
                            w.request_redraw();
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.draw();
            }
            _ => {}
        }
    }
}
