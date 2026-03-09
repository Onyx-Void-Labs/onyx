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
    void_counter: u32,
    note_counter: u32,
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
            void_counter: 0,
            note_counter: 0,
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

        // UI Engine Hit-Testing
        if let Some(rect) = self.ribbon_hitboxes.get("btn_void") {
            if rect.contains(pt) {
                self.void_counter += 1;
                let title = format!("Void {}", self.void_counter);
                let _ = self.workspace.create_void(None, &title);
                tracing::info!("Created Void: {}", title);
                self.scene_dirty = true;
                return;
            }
        }

        if let Some(rect) = self.ribbon_hitboxes.get("btn_note") {
            if rect.contains(pt) {
                if let Some(parent_id) = self.workspace.first_void_id() {
                    self.note_counter += 1;
                    let title = format!("Note {}", self.note_counter);
                    let note_id = self
                        .workspace
                        .create_note(&parent_id, &title)
                        .unwrap_or_default();
                    tracing::info!("Created Note: {}", title);

                    let initial_block = onyx_core::blocks::Block::empty_paragraph();
                    let _ = self.workspace.set_note_blocks(&note_id, &[initial_block]);

                    self.selected_node_id = Some(note_id);
                    self.scene_dirty = true;
                }
                return;
            }
        }

        if let Some(rect) = self.ribbon_hitboxes.get("btn_architect") {
            if rect.contains(pt) {
                self.is_architect_mode = !self.is_architect_mode;
                tracing::info!("Architect Mode: {}", self.is_architect_mode);
                self.scene_dirty = true;
                return;
            }
        }
    }

    fn draw_system_shelf(&mut self, width: f64) {
        use vello::kurbo::{Affine, Rect};
        use vello::peniko::{Brush, Color, Fill};

        // 1% OVERKILL: A unified structural shelf, not floating pills.
        let shelf_rect = Rect::new(0.0, 0.0, width, 60.0);
        let shelf_bg = Brush::Solid(Color::from_rgba8(20, 20, 24, 255));
        let divider_line = Brush::Solid(Color::from_rgba8(40, 40, 50, 255));

        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &shelf_bg,
            None,
            &shelf_rect,
        );
        // Bottom border line for "shelf" feel
        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &divider_line,
            None,
            &Rect::new(0.0, 59.0, width, 60.0),
        );

        // Logic: Change tools based on is_architect_mode
        let label = if self.is_architect_mode {
            "ARCHITECT: GRID MODE (Widgets active)"
        } else {
            "DOCUMENT: FOCUS MODE (Standard)"
        };
        draw_text(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            label,
            40.0,
            20.0,
            18.0,
            Color::from_rgba8(150, 150, 160, 255),
        );

        // Add a "Help" hint
        draw_text(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            "Press '/' to toggle tools",
            width - 200.0,
            20.0,
            14.0,
            Color::from_rgba8(80, 80, 90, 255),
        );
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

        // Draw the new System Shelf
        self.draw_system_shelf(width_f);

        // Shift the canvas down so text doesn't overlap the shelf
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
                self.scene_dirty = true;
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
