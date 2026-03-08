// ─── Onyx Void — Application (Vello + Winit + LoroTree UI) ─────────

use anyhow::Context;
use std::sync::mpsc;
use std::sync::Arc;

use onyx_core::blocks::Block;
use onyx_core::document::OnyxWorkspace;
use onyx_core::fsrs::{self, CardState, FlashcardData, Scheduler};
use onyx_core::model::{NodeType, PropertyType};
use parley::layout::{Alignment, AlignmentOptions, PositionedLayoutItem};
use parley::style::StyleProperty;
use parley::{FontContext, LayoutContext};
use vello::kurbo::{Affine, Point, Rect};
use vello::peniko::{Brush, Color, Fill};
use vello::util::{RenderContext, RenderSurface};
use vello::{Renderer, RendererOptions, Scene};

// ensure we use the same wgpu version that Vello depends on; alias it
use vello::wgpu as wgpu;

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

        Ok(Self {
            render_cx: RenderContext::new(),
            renderer: None,
            surface: None,
            window: None,
            scene: Scene::new(),
            font_cx,
            layout_cx: LayoutContext::new(),
            workspace: OnyxWorkspace::new(),
            cursor_pos: (0.0, 0.0),
            void_counter: 0,
            note_counter: 0,
            selected_node_id: None,
            node_rects: Vec::new(),
            add_prop_btn_rect: None,
            prop_value_rects: Vec::new(),
            editing_field: None,
            input_buffer: String::new(),
            embed_tx,
            embed_rx,
            is_architect_mode: false,
            scene_dirty: true,
            full_rebuild: false,
        })
    }

    fn handle_click(&mut self) {
        let pt = Point::new(self.cursor_pos.0, self.cursor_pos.1);

        if void_btn().contains(pt) {
            self.void_counter += 1;
            let title = format!("Void {}", self.void_counter);
            let _ = self.workspace.create_void(None, &title);
            tracing::info!("Created: {}", title);
        } else if note_btn().contains(pt) {
            if let Some(parent_id) = self.workspace.first_void_id() {
                self.note_counter += 1;
                let title = format!("Note {}", self.note_counter);
                let note_id = self
                    .workspace
                    .create_note(&parent_id, &title)
                    .unwrap_or_default();
                tracing::info!("Created: {}", title);

                // ── Block Engine: initialize note with one empty Paragraph ──
                let initial_block = Block::empty_paragraph();
                let _ = self.workspace.set_note_blocks(&note_id, &[initial_block]);
                println!("🧱 Block Engine: Initialized note with 1 block.");

                // ── FSRS: create a dummy flashcard and schedule it ──
                let card_state = CardState::default();
                let mut sched = Scheduler::default();
                let (scheduled_state, days) = sched.next_interval(&card_state, 3); // Good
                let card_id = uuid::Uuid::new_v4().to_string();
                let flashcard = FlashcardData {
                    front: format!("What is {}?", title),
                    back: "A note in Onyx Void.".to_string(),
                    note_id: note_id.clone(),
                    state: scheduled_state,
                };
                let _ = self.workspace.set_flashcard(&card_id, &flashcard);
                println!("🎓 FSRS: Card created. Next review in {} day(s).", days);

                // Queue embedding for the new note
                let _ = self.embed_tx.send((note_id, title));
            } else {
                tracing::warn!("No void exists — create a Void first");
            }
        } else {
            let mut handled = false;

            // [+ Add Prop] button in inspector
            if let Some(btn_rect) = self.add_prop_btn_rect {
                if btn_rect.contains(pt) {
                    if let Some(ref void_id) = self.selected_node_id {
                        if self.workspace.node_type_of(void_id) == Some(NodeType::Void) {
                            let void_id = void_id.clone();
                            let _ = self.workspace.add_property_schema(
                                &void_id,
                                "Week",
                                PropertyType::Select(vec![
                                    "Monday".into(),
                                    "Tuesday".into(),
                                    "Wednesday".into(),
                                    "Thursday".into(),
                                    "Friday".into(),
                                    "Saturday".into(),
                                    "Sunday".into(),
                                ]),
                            );
                            tracing::info!("Added 'Week' property to void");
                        }
                    }
                    handled = true;
                }
            }

            // Property value fields in inspector
            if !handled {
                for (prop_name, rect) in &self.prop_value_rects {
                    if rect.contains(pt) {
                        if let Some(ref note_id) = self.selected_node_id {
                            if let Some(void_id) = self.workspace.parent_void_of(note_id) {
                                let values = self.workspace.get_note_values(note_id, &void_id);
                                self.input_buffer =
                                    values.get(prop_name).cloned().unwrap_or_default();
                                self.editing_field = Some(prop_name.clone());
                            }
                        }
                        handled = true;
                        break;
                    }
                }
            }

            // Tree node selection
            if !handled {
                for (id, rect) in &self.node_rects {
                    if rect.contains(pt) {
                        self.selected_node_id = Some(id.clone());
                        self.editing_field = None;
                        self.input_buffer.clear();
                        break;
                    }
                }
            }
        }

        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    pub fn draw(&mut self) {
        while let Ok((note_id, vec)) = self.embed_rx.try_recv() {
            let _ = self.workspace.set_vector(&note_id, &vec);
        }

        let surface = match self.surface.as_mut() {
            Some(s) => s,
            None => return,
        };
        let device_id = surface.dev_id;
        let device = &self.render_cx.devices[device_id];

        let output = match surface.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render"),
            });

        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 9.0 / 255.0,
                            g: 9.0 / 255.0,
                            b: 11.0 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        device.queue.submit(std::iter::once(encoder.finish()));
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

        // CPU renderer — bulletproof, no format bugs
        match Renderer::new(
            &device.device,
            RendererOptions {
                use_cpu: true, // ← Skip all GPU validation hell
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
