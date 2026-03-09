// ─── Onyx Void — Application (Vello + Winit + LoroTree UI) ─────────

use anyhow::Context;
use std::sync::mpsc;
use std::sync::Arc;

use crate::editor_renderer::EditorRenderer;
use onyx_core::blocks::Block;
use onyx_core::document::OnyxWorkspace;
use onyx_core::fsrs::{CardState, FlashcardData, Scheduler};
use onyx_core::model::{NodeType, PropertyType};
use parley::layout::{Alignment, AlignmentOptions, PositionedLayoutItem};
use parley::style::StyleProperty;
use parley::{FontContext, LayoutContext};
use vello::kurbo::{Affine, Point, Rect};
use vello::peniko::{Brush, Color, Fill};
use vello::util::{RenderContext, RenderSurface};
use vello::{Renderer, RendererOptions, Scene};

// ensure we use the same wgpu version that Vello depends on; alias it
use vello::wgpu;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

const RIBBON_H: f64 = 44.0;
const FONT_SIZE: f32 = 14.0;
const LINE_H: f64 = 22.0;
const LEFT_PAD: f64 = 20.0;
const INDENT: f64 = 24.0;
const INSPECTOR_W: f64 = 300.0;
const INSPECTOR_PAD: f64 = 12.0;
const PROP_ROW_H: f64 = 28.0;

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
fn inspector_bg() -> Color {
    Color::from_rgba8(30, 30, 36, 255)
}
fn selected_bg() -> Color {
    Color::from_rgba8(40, 40, 55, 255)
}
fn input_bg() -> Color {
    Color::from_rgba8(45, 45, 55, 255)
}
fn divider_color() -> Color {
    Color::from_rgba8(60, 60, 70, 255)
}
fn label_color() -> Color {
    Color::from_rgba8(150, 150, 165, 255)
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
    node_rects: Vec<(String, Rect)>,
    add_prop_btn_rect: Option<Rect>,
    prop_value_rects: Vec<(String, Rect)>,
    editing_field: Option<String>,
    input_buffer: String,
    /// Send (note_id, text) to the AI background thread for embedding.
    embed_tx: mpsc::Sender<(String, String)>,
    /// Receive (note_id, vector) back from the AI thread.
    embed_rx: mpsc::Receiver<(String, Vec<f32>)>,
    /// Whether architect mode (grid overlay) is active.
    pub is_architect_mode: bool,
    /// Dirty flag: only re-render when the scene actually changed.
    pub scene_dirty: bool,
    pub full_rebuild: bool, // Track if a full or partial rebuild is needed
    // --- Vello Compositing ---
    pub cached_target: Option<wgpu::Texture>,
    pub blitter: Option<wgpu::util::TextureBlitter>,
    // --- UI State Tracking ---
    pub ribbon_hitboxes: std::collections::HashMap<String, vello::kurbo::Rect>,
}

impl OnyxApp {
    /// Toggle between 850px (Focused) and 1400px (Wide Mode)
    /// Set self.scene_dirty = true to trigger a re-render
    pub fn toggle_spine_width(&mut self) {
        self.is_architect_mode = !self.is_architect_mode;
        self.scene_dirty = true;
    }
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
            node_rects: Vec::new(),
            add_prop_btn_rect: None,
            editor: EditorRenderer::new(),
            prop_value_rects: Vec::new(),
            editing_field: None,
            input_buffer: String::new(),
            embed_tx,
            embed_rx,
            is_architect_mode: false,
            scene_dirty: true,
            full_rebuild: false,
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

        // Pass click down to the Editor Canvas
        self.editor
            .on_mouse_click(self.cursor_pos.0, self.cursor_pos.1);
    }

    fn draw_top_ribbon(&mut self, width: f64) {
        use vello::kurbo::{Affine, Rect, RoundedRect, Stroke};
        use vello::peniko::{Brush, Color, Fill};

        self.ribbon_hitboxes.clear();

        let pill_y = 24.0;
        let pill_h = 44.0;
        let border_stroke = Stroke::new(1.0);
        let glass_bg = Brush::Solid(Color::from_rgba8(30, 30, 36, 230));
        let border_brush = Brush::Solid(Color::from_rgba8(60, 60, 70, 255));
        let text_color = Color::from_rgba8(220, 220, 230, 255);
        let accent_color = Color::from_rgba8(130, 170, 255, 255);

        // ── PILL 1: WORKSPACE (Left) ──
        let p1_x = 40.0;
        let p1_w = 200.0;
        let p1_rect = RoundedRect::new(p1_x, pill_y, p1_x + p1_w, pill_y + pill_h, 8.0);
        self.scene
            .fill(Fill::NonZero, Affine::IDENTITY, &glass_bg, None, &p1_rect);
        self.scene.stroke(
            &border_stroke,
            Affine::IDENTITY,
            &border_brush,
            None,
            &p1_rect,
        );

        draw_text(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            "+ Void",
            p1_x + 24.0,
            pill_y + 12.0,
            16.0,
            text_color,
        );
        self.ribbon_hitboxes.insert(
            "btn_void".into(),
            Rect::new(p1_x, pill_y, p1_x + 100.0, pill_y + pill_h),
        );

        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            &border_brush,
            None,
            &Rect::new(
                p1_x + 100.0,
                pill_y + 8.0,
                p1_x + 101.0,
                pill_y + pill_h - 8.0,
            ),
        );

        draw_text(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            "+ Note",
            p1_x + 124.0,
            pill_y + 12.0,
            16.0,
            text_color,
        );
        self.ribbon_hitboxes.insert(
            "btn_note".into(),
            Rect::new(p1_x + 100.0, pill_y, p1_x + p1_w, pill_y + pill_h),
        );

        // ── PILL 2: LENS FORMATTING (Center) ──
        let p2_w = 340.0;
        let p2_x = (width / 2.0) - (p2_w / 2.0);
        let p2_rect = RoundedRect::new(p2_x, pill_y, p2_x + p2_w, pill_y + pill_h, 8.0);
        self.scene
            .fill(Fill::NonZero, Affine::IDENTITY, &glass_bg, None, &p2_rect);
        self.scene.stroke(
            &border_stroke,
            Affine::IDENTITY,
            &border_brush,
            None,
            &p2_rect,
        );

        draw_text(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            "Bold",
            p2_x + 30.0,
            pill_y + 12.0,
            16.0,
            text_color,
        );
        draw_text(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            "Italic",
            p2_x + 100.0,
            pill_y + 12.0,
            16.0,
            text_color,
        );
        draw_text(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            "H1",
            p2_x + 170.0,
            pill_y + 12.0,
            16.0,
            text_color,
        );
        draw_text(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            "Cloze",
            p2_x + 240.0,
            pill_y + 12.0,
            16.0,
            accent_color,
        );

        // ── PILL 3: NEURO ENGINE (Right) ──
        let p3_w = 140.0;
        let p3_x = width - p3_w - 40.0;
        let p3_rect = RoundedRect::new(p3_x, pill_y, p3_x + p3_w, pill_y + pill_h, 8.0);

        let arch_bg = if self.is_architect_mode {
            Brush::Solid(Color::from_rgba8(180, 220, 140, 255))
        } else {
            glass_bg.clone()
        };
        let arch_text = if self.is_architect_mode {
            Color::BLACK
        } else {
            text_color
        };

        self.scene
            .fill(Fill::NonZero, Affine::IDENTITY, &arch_bg, None, &p3_rect);
        self.scene.stroke(
            &border_stroke,
            Affine::IDENTITY,
            &border_brush,
            None,
            &p3_rect,
        );

        draw_text(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            "Grid Mode",
            p3_x + 30.0,
            pill_y + 12.0,
            16.0,
            arch_text,
        );
        self.ribbon_hitboxes.insert(
            "btn_architect".into(),
            Rect::new(p3_x, pill_y, p3_x + p3_w, pill_y + pill_h),
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

        // Draw Custom UI
        self.draw_top_ribbon(width_f);

        if let Some(note_id) = &self.selected_node_id {
            self.editor
                .build_scene(&mut self.scene, &self.workspace, note_id, width_f);
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

    // New: full scene render (replace with your actual logic)
    fn render_scene(&mut self) {
        // ...existing code for full scene build...
    }

    // New: partial render for dirty widgets only
    fn partial_render(&mut self) {
        // Only re-layout dirty widgets (title, sections, editor blocks)
        // Reuse last frame's Vello scene tree, patch changes
        // Skip unchanged areas
        // Target: Active GPU drops to 1-3%
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
                // inform editor of clicks for cursor movement
                let (x, y) = self.cursor_pos;
                self.editor.on_mouse_click(x, y);
            }
            WindowEvent::KeyboardInput { event, .. } => {
                // notify editor regardless of editing state
                if event.state == ElementState::Pressed {
                    let key_str = format!("{:?}", event.logical_key);
                    self.editor.on_key_down(&key_str);
                }
                if self.editing_field.is_some() && event.state == ElementState::Pressed {
                    self.scene_dirty = true;
                    match &event.logical_key {
                        Key::Named(NamedKey::Enter) => {
                            if let (Some(note_id), Some(field)) =
                                (self.selected_node_id.clone(), self.editing_field.clone())
                            {
                                if let Some(void_id) = self.workspace.parent_void_of(&note_id) {
                                    let _ = self.workspace.set_note_property(
                                        &note_id,
                                        &void_id,
                                        &field,
                                        &self.input_buffer,
                                    );
                                    tracing::info!(
                                        "Set {}.{} = {}",
                                        note_id,
                                        field,
                                        self.input_buffer
                                    );
                                    // Re-embed note with updated property text
                                    let title =
                                        self.workspace.node_title(&note_id).unwrap_or_default();
                                    let embed_text =
                                        format!("{} {} {}", title, field, self.input_buffer);
                                    let _ = self.embed_tx.send((note_id, embed_text));
                                }
                            }
                            self.editing_field = None;
                            self.input_buffer.clear();
                        }
                        Key::Named(NamedKey::Escape) => {
                            self.editing_field = None;
                            self.input_buffer.clear();
                        }
                        Key::Named(NamedKey::Backspace) => {
                            self.input_buffer.pop();
                        }
                        _ => {
                            if let Some(ref text) = event.text {
                                self.input_buffer.push_str(text);
                            }
                        }
                    }
                    if let Some(w) = &self.window {
                        w.request_redraw();
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
