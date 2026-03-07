// ─── Onyx Void — Application (Vello + Winit + LoroTree UI) ─────────

use std::sync::mpsc;
use std::sync::Arc;

use onyx_core::blocks::Block;
use onyx_core::document::OnyxWorkspace;
use onyx_core::fsrs::{self, CardState, FlashcardData};
use onyx_core::model::{NodeType, PropertyType};
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
}

impl OnyxApp {
    pub fn new() -> Self {
        // Channels: main → AI thread (embed requests), AI thread → main (results)
        let (embed_tx, work_rx) = mpsc::channel::<(String, String)>();
        let (result_tx, embed_rx) = mpsc::channel::<(String, Vec<f32>)>();

        // Spawn AI background thread
        std::thread::Builder::new()
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
            .expect("spawn AI thread");

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
            selected_node_id: None,
            node_rects: Vec::new(),
            add_prop_btn_rect: None,
            prop_value_rects: Vec::new(),
            editing_field: None,
            input_buffer: String::new(),
            embed_tx,
            embed_rx,
        }
    }

    fn handle_click(&mut self) {
        let pt = Point::new(self.cursor_pos.0, self.cursor_pos.1);

        if void_btn().contains(pt) {
            self.void_counter += 1;
            let title = format!("Void {}", self.void_counter);
            self.workspace.create_void(None, &title);
            tracing::info!("Created: {}", title);
        } else if note_btn().contains(pt) {
            if let Some(parent_id) = self.workspace.first_void_id() {
                self.note_counter += 1;
                let title = format!("Note {}", self.note_counter);
                let note_id = self.workspace.create_note(&parent_id, &title);
                tracing::info!("Created: {}", title);

                // ── Block Engine: initialize note with one empty Paragraph ──
                let initial_block = Block::empty_paragraph();
                self.workspace.set_note_blocks(&note_id, &[initial_block]);
                println!("🧱 Block Engine: Initialized note with 1 block.");

                // ── FSRS: create a dummy flashcard and schedule it ──
                let card_state = CardState::new();
                let (scheduled_state, days) = fsrs::next_interval(&card_state, 3); // Good
                let card_id = uuid::Uuid::new_v4().to_string();
                let flashcard = FlashcardData {
                    front: format!("What is {}?", title),
                    back: format!("A note in Onyx Void."),
                    note_id: note_id.clone(),
                    state: scheduled_state,
                };
                self.workspace.set_flashcard(&card_id, &flashcard);
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
                            self.workspace.add_property_schema(
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
        // Drain completed embeddings from AI thread
        while let Ok((note_id, vec)) = self.embed_rx.try_recv() {
            self.workspace.set_vector(&note_id, &vec);
        }

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
        self.node_rects.clear();
        let nodes = self.workspace.get_tree_nodes();
        let tree_width = width as f64 - INSPECTOR_W;
        let mut y = RIBBON_H + 20.0;
        for (node, depth) in &nodes {
            let x = LEFT_PAD + (*depth as f64) * INDENT;
            let row_rect = Rect::new(0.0, y, tree_width, y + LINE_H);

            if self.selected_node_id.as_ref() == Some(&node.id) {
                self.scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    selected_bg(),
                    None,
                    &row_rect,
                );
            }

            self.node_rects.push((node.id.clone(), row_rect));

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

        // ── Inspector Panel ──────────────────────────────────────
        let inspector_x = width as f64 - INSPECTOR_W;
        self.add_prop_btn_rect = None;
        self.prop_value_rects.clear();

        // Inspector background
        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            inspector_bg(),
            None,
            &Rect::new(inspector_x, 0.0, width as f64, height as f64),
        );
        // Divider
        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            divider_color(),
            None,
            &Rect::new(inspector_x, 0.0, inspector_x + 1.0, height as f64),
        );

        let ix = inspector_x + INSPECTOR_PAD;
        let mut iy = RIBBON_H + INSPECTOR_PAD;

        if let Some(ref selected_id) = self.selected_node_id {
            let node_type = self.workspace.node_type_of(selected_id);
            let title = self.workspace.node_title(selected_id).unwrap_or_default();

            draw_text(
                &mut self.scene,
                &mut self.font_cx,
                &mut self.layout_cx,
                &format!("Selected: {}", title),
                ix,
                iy,
                FONT_SIZE,
                text_color(),
            );
            iy += LINE_H + 8.0;

            match node_type {
                Some(NodeType::Void) => {
                    let schema = self.workspace.get_active_schema(selected_id);

                    draw_text(
                        &mut self.scene,
                        &mut self.font_cx,
                        &mut self.layout_cx,
                        "Schema Properties:",
                        ix,
                        iy,
                        FONT_SIZE - 1.0,
                        label_color(),
                    );
                    iy += LINE_H;

                    if schema.is_empty() {
                        draw_text(
                            &mut self.scene,
                            &mut self.font_cx,
                            &mut self.layout_cx,
                            "  (none)",
                            ix,
                            iy,
                            FONT_SIZE - 1.0,
                            muted_color(),
                        );
                        iy += LINE_H;
                    } else {
                        for prop in &schema {
                            let kind_str = match &prop.kind {
                                PropertyType::Text => "Text",
                                PropertyType::Select(_) => "Select",
                                PropertyType::Date => "Date",
                                PropertyType::Checkbox => "Checkbox",
                            };
                            draw_text(
                                &mut self.scene,
                                &mut self.font_cx,
                                &mut self.layout_cx,
                                &format!("  {} ({})", prop.name, kind_str),
                                ix,
                                iy,
                                FONT_SIZE,
                                text_color(),
                            );
                            iy += LINE_H;
                        }
                    }

                    iy += 4.0;
                    let add_btn = Rect::new(ix, iy, ix + 120.0, iy + 28.0);
                    self.scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        if add_btn.contains(pt) {
                            btn_hover()
                        } else {
                            btn_color()
                        },
                        None,
                        &RoundedRect::from_rect(add_btn, 4.0),
                    );
                    draw_text(
                        &mut self.scene,
                        &mut self.font_cx,
                        &mut self.layout_cx,
                        "+ Add Prop",
                        add_btn.x0 + 8.0,
                        add_btn.y0 + 5.0,
                        FONT_SIZE,
                        text_color(),
                    );
                    self.add_prop_btn_rect = Some(add_btn);
                }
                Some(NodeType::Note) => {
                    if let Some(void_id) = self.workspace.parent_void_of(selected_id) {
                        let schema = self.workspace.get_active_schema(&void_id);
                        let values = self.workspace.get_note_values(selected_id, &void_id);

                        draw_text(
                            &mut self.scene,
                            &mut self.font_cx,
                            &mut self.layout_cx,
                            "Properties:",
                            ix,
                            iy,
                            FONT_SIZE - 1.0,
                            label_color(),
                        );
                        iy += LINE_H;

                        if schema.is_empty() {
                            draw_text(
                                &mut self.scene,
                                &mut self.font_cx,
                                &mut self.layout_cx,
                                "  (no schema defined)",
                                ix,
                                iy,
                                FONT_SIZE - 1.0,
                                muted_color(),
                            );
                        } else {
                            for prop in &schema {
                                draw_text(
                                    &mut self.scene,
                                    &mut self.font_cx,
                                    &mut self.layout_cx,
                                    &prop.name,
                                    ix,
                                    iy,
                                    FONT_SIZE - 1.0,
                                    label_color(),
                                );
                                iy += LINE_H * 0.8;

                                let val = values.get(&prop.name).map(|s| s.as_str()).unwrap_or("");
                                let is_editing = self.editing_field.as_ref() == Some(&prop.name);
                                let display_val = if is_editing { &self.input_buffer } else { val };

                                let field_rect = Rect::new(
                                    ix,
                                    iy,
                                    inspector_x + INSPECTOR_W - INSPECTOR_PAD,
                                    iy + PROP_ROW_H,
                                );
                                self.scene.fill(
                                    Fill::NonZero,
                                    Affine::IDENTITY,
                                    if is_editing {
                                        Color::from_rgba8(55, 55, 70, 255)
                                    } else {
                                        input_bg()
                                    },
                                    None,
                                    &RoundedRect::from_rect(field_rect, 3.0),
                                );

                                let display_text = if is_editing {
                                    format!("{}|", display_val)
                                } else if display_val.is_empty() {
                                    "\u{2014}".to_string()
                                } else {
                                    display_val.to_string()
                                };

                                draw_text(
                                    &mut self.scene,
                                    &mut self.font_cx,
                                    &mut self.layout_cx,
                                    &display_text,
                                    ix + 6.0,
                                    iy + 5.0,
                                    FONT_SIZE,
                                    text_color(),
                                );

                                self.prop_value_rects.push((prop.name.clone(), field_rect));
                                iy += PROP_ROW_H + 6.0;
                            }
                        }
                    }
                }
                _ => {}
            }
        } else {
            draw_text(
                &mut self.scene,
                &mut self.font_cx,
                &mut self.layout_cx,
                "No selection",
                ix,
                iy,
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
            WindowEvent::KeyboardInput { event, .. } => {
                if self.editing_field.is_some() && event.state == ElementState::Pressed {
                    match &event.logical_key {
                        Key::Named(NamedKey::Enter) => {
                            if let (Some(note_id), Some(field)) =
                                (self.selected_node_id.clone(), self.editing_field.clone())
                            {
                                if let Some(void_id) = self.workspace.parent_void_of(&note_id) {
                                    self.workspace.set_note_property(
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
