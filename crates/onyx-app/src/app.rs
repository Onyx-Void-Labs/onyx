// ─── Onyx Void — Application ─────────

use anyhow::Context;
use std::sync::mpsc;
use std::sync::Arc;

use crate::editor_renderer::EditorRenderer;
use crate::cursor::CursorState;
use crate::renderer::canvas::CanvasRenderer;
use onyx_core::document::OnyxWorkspace;
use parley::layout::{Alignment, AlignmentOptions};
use parley::style::StyleProperty;
use parley::{FontContext, LayoutContext};
use vello::kurbo::{Affine, Rect};
use vello::peniko::{Brush, Color};
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu;
use vello::{Renderer, RendererOptions, Scene};
use onyx_core::blocks::Attribute;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};


pub fn draw_text(
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

pub struct NavigationHistory {
    pub stack: Vec<String>,
    pub index: usize,
}

impl NavigationHistory {
    pub fn new(initial: String) -> Self {
        Self {
            stack: vec![initial],
            index: 0,
        }
    }

    pub fn push(&mut self, id: String) {
        if self.stack.get(self.index) == Some(&id) {
            return;
        }
        self.stack.truncate(self.index + 1);
        self.stack.push(id);
        self.index = self.stack.len() - 1;
    }

    pub fn back(&mut self) -> Option<String> {
        if self.index > 0 {
            self.index -= 1;
            Some(self.stack[self.index].clone())
        } else {
            None
        }
    }

    pub fn forward(&mut self) -> Option<String> {
        if self.index + 1 < self.stack.len() {
            self.index += 1;
            Some(self.stack[self.index].clone())
        } else {
            None
        }
    }
    
    #[allow(dead_code)]
    pub fn current(&self) -> String {
        self.stack[self.index].clone()
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum DropdownType {
    None,
    FontFamily,
    FontSize,
    TextColor,
    HighlightColor,
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
    pub cursor: CursorState,
    pub nav_history: NavigationHistory,
    pub canvas_renderer: CanvasRenderer,
    cursor_pos: (f64, f64),
    last_mouse_pos: (f64, f64),
    is_panning: bool,
    selected_node_id: Option<String>,

    // --- The Live Typing Engine ---
    pub start_time: std::time::Instant,
    pub live_text_buffer: String,

    #[allow(dead_code)]
    embed_tx: mpsc::Sender<(String, String)>,
    embed_rx: mpsc::Receiver<(String, Vec<f32>)>,
    pub is_architect_mode: bool,
    pub scene_dirty: bool,
    pub cached_target: Option<wgpu::Texture>,
    pub blitter: Option<wgpu::util::TextureBlitter>,
    pub ribbon_hitboxes: std::collections::HashMap<String, Rect>,
    pub is_shelf_open: bool,
    pub scale_factor: f64,
    pub modifiers: winit::keyboard::ModifiersState,
    pub ribbon_hovered: Option<String>,
    pub font_family_index: usize,
    pub font_size_index: usize,
    pub active_dropdown: DropdownType,
    pub dropdown_search: String,
    pub is_dragging_text: bool,
    pub active_formatting: Vec<Attribute>,
    pub hovered_block: Option<usize>,
    pub is_editing_title: bool,
    pub last_text_color: vello::peniko::Color,
    pub last_highlight_color: vello::peniko::Color,
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
            selected_node_id: initial_note_id.clone(),
            live_text_buffer: String::new(),
            editor: EditorRenderer::new(),
            cursor: CursorState::new(),
            nav_history: NavigationHistory::new(initial_note_id.clone().unwrap_or_default()),
            canvas_renderer: CanvasRenderer::new(),
            cursor_pos: (0.0, 0.0),
            last_mouse_pos: (0.0, 0.0),
            is_panning: false,
            embed_tx,
            embed_rx,
            is_architect_mode: false,
            scene_dirty: true,
            cached_target: None,
            blitter: None,
            ribbon_hitboxes: std::collections::HashMap::new(),
            start_time: std::time::Instant::now(),
            is_shelf_open: false,
            scale_factor: 1.0,
            modifiers: winit::keyboard::ModifiersState::default(),
            ribbon_hovered: None,
            font_family_index: 0,
            font_size_index: 1, // 14.0 is index 1
            active_dropdown: DropdownType::None,
            dropdown_search: String::new(),
            is_dragging_text: false,
            active_formatting: Vec::new(),
            hovered_block: None,
            is_editing_title: false,
            last_text_color: vello::peniko::Color::from_rgba8(96, 165, 250, 255), // Default Accent Blue
            last_highlight_color: vello::peniko::Color::from_rgba8(250, 210, 40, 255), // Default Yellow
        })
    }

    pub fn handle_drag(&mut self) {
        if self.is_architect_mode || self.selected_node_id.is_none() {
            return;
        }

        let pt = vello::kurbo::Point::new(
            self.cursor_pos.0 / self.scale_factor,
            self.cursor_pos.1 / self.scale_factor,
        );

        if pt.y > 164.0 {
            // Find which block we are dragging over
            let hit_y = pt.y;
            let mut current_y = 52.0 + 82.0 + 30.0 + 35.0 + 60.0 + 40.0; // 299.0
            let mut hit_idx = None;

            for (i, layout) in self.editor.layouts.iter().enumerate() {
                let h = layout.height() as f64 + 24.0;
                if hit_y >= current_y && hit_y < current_y + h {
                    hit_idx = Some(i);
                    break;
                }
                current_y += h;
            }

            if let Some(idx) = hit_idx {
                let block_x = pt.x - 60.0;
                let block_y = hit_y - current_y;
                let byte_offset = self.editor.hit_test_x(idx, block_x, block_y);

                // Initialize selection anchor if not dragging yet
                if self.cursor.selection_anchor.is_none() {
                    self.cursor.selection_anchor = Some((self.cursor.block_index, self.cursor.byte_offset));
                }

                // Move cursor to dragged position, expanding selection
                self.cursor.move_to(idx, byte_offset, true);
                self.scene_dirty = true;
            }
        }
    }

    pub fn handle_click(&mut self) {
        self.active_formatting.clear();
        let pt = vello::kurbo::Point::new(
            self.cursor_pos.0 / self.scale_factor,
            self.cursor_pos.1 / self.scale_factor,
        );

        // 1. Dropdown item selection (highest priority)
        if self.active_dropdown != DropdownType::None {
            let mut selected_item = None;
            for (id, rect) in &self.ribbon_hitboxes {
                if id.starts_with("dropdown_item:") && rect.contains(pt) {
                    selected_item = Some(id.strip_prefix("dropdown_item:").unwrap().to_string());
                    break;
                }
            }
            
            if let Some(item) = selected_item {
                if let Some(note_id) = self.selected_node_id.as_ref() {
                    match self.active_dropdown {
                        DropdownType::FontFamily => {
                            let fonts = ["Inter", "Georgia", "Comic Sans MS", "Roboto", "Times New Roman", "Arial"];
                            if let Some(idx) = fonts.iter().position(|&f| f == item) {
                                self.font_family_index = idx;
                            }
                            let blocks = self.workspace.get_note_blocks(note_id);
                            if let Some(block) = blocks.get(self.cursor.block_index) {
                                let _ = onyx_core::editing::toggle_attribute(
                                    &mut self.workspace, note_id,
                                    self.cursor.block_index, 0..block.content.len(),
                                    Attribute::FontFamily(item)
                                );
                            }
                        }
                        DropdownType::FontSize => {
                            if let Ok(size) = item.parse::<f32>() {
                                let sizes = [8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0, 48.0, 72.0];
                                if let Some(idx) = sizes.iter().position(|&s| s == size) {
                                    self.font_size_index = idx;
                                }
                                let blocks = self.workspace.get_note_blocks(note_id);
                                if let Some(block) = blocks.get(self.cursor.block_index) {
                                    let _ = onyx_core::editing::toggle_attribute(
                                        &mut self.workspace, note_id,
                                        self.cursor.block_index, 0..block.content.len(),
                                        Attribute::FontSize(size)
                                    );
                                }
                            }
                        }
                        DropdownType::TextColor | DropdownType::HighlightColor => {
                            let is_text = self.active_dropdown == DropdownType::TextColor;
                            
                            if item == "Clear" {
                                if let Some((start, end)) = self.cursor.get_selection_range() {
                                    if start.0 == end.0 {
                                        let dummy_attr = if is_text {
                                            Attribute::Color([0.0; 4])
                                        } else {
                                            Attribute::Highlight([0.0; 4])
                                        };
                                        let _ = onyx_core::editing::clear_attribute_type(
                                            &mut self.workspace, note_id,
                                            start.0, start.1..end.1, dummy_attr
                                        );
                                    }
                                } else {
                                    self.active_formatting.retain(|a| {
                                        match (is_text, a) {
                                            (true, Attribute::Color(_)) => false,
                                            (false, Attribute::Highlight(_)) => false,
                                            _ => true
                                        }
                                    });
                                }
                            } else {
                                let c = match item.as_str() {
                                    "Red" => [1.0, 0.2, 0.2, 1.0],
                                    "Blue" => [0.0, 0.4, 1.0, 1.0],
                                    "Green" => [0.2, 0.8, 0.2, 1.0],
                                    "Yellow" => [1.0, 0.8, 0.0, 1.0],
                                    "Orange" => [1.0, 0.5, 0.0, 1.0],
                                    "Purple" => [0.6, 0.3, 0.9, 1.0],
                                    "Gray" => [0.6, 0.6, 0.6, 1.0],
                                    "Black" => [0.1, 0.1, 0.1, 1.0],
                                    "White" => [1.0, 1.0, 1.0, 1.0],
                                    _ => [1.0, 1.0, 1.0, 1.0],
                                };
                                let attr = if is_text {
                                    self.last_text_color = vello::peniko::Color::from_rgba8(
                                        (c[0]*255.0) as u8, (c[1]*255.0) as u8, (c[2]*255.0) as u8, (c[3]*255.0) as u8
                                    );
                                    Attribute::Color(c)
                                } else {
                                    self.last_highlight_color = vello::peniko::Color::from_rgba8(
                                        (c[0]*255.0) as u8, (c[1]*255.0) as u8, (c[2]*255.0) as u8, (c[3]*255.0) as u8
                                    );
                                    Attribute::Highlight(c)
                                };

                                if let Some((start, end)) = self.cursor.get_selection_range() {
                                    if start.0 == end.0 {
                                        let dummy_attr = if is_text {
                                            Attribute::Color([0.0; 4])
                                        } else {
                                            Attribute::Highlight([0.0; 4])
                                        };
                                        // clear overlapping first before applying new color
                                        let _ = onyx_core::editing::clear_attribute_type(
                                            &mut self.workspace, note_id,
                                            start.0, start.1..end.1, dummy_attr
                                        );

                                        let _ = onyx_core::editing::apply_attribute(
                                            &mut self.workspace, note_id,
                                            start.0, start.1..end.1, attr.clone()
                                        );
                                    }
                                } else {
                                    if let Some(pos) = self.active_formatting.iter().position(|a| match (is_text, a) {
                                        (true, Attribute::Color(_)) => true,
                                        (false, Attribute::Highlight(_)) => true,
                                        _ => false,
                                    }) {
                                        self.active_formatting[pos] = attr;
                                    } else {
                                        self.active_formatting.push(attr);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                self.active_dropdown = DropdownType::None;
                self.scene_dirty = true;
                return;
            } else {
                // If clicked outside the dropdown (but check if clicked the button again which is handled below)
                // Actually, let's just close it if we didn't hit an item, unless we hit the toggle buttons.
                let mut hit_toggle = false;
                if let Some(rect) = self.ribbon_hitboxes.get("ribbon:font_family") { if rect.contains(pt) { hit_toggle = true; } }
                if let Some(rect) = self.ribbon_hitboxes.get("ribbon:font_size") { if rect.contains(pt) { hit_toggle = true; } }
                if let Some(rect) = self.ribbon_hitboxes.get("ribbon:color") { if rect.contains(pt) { hit_toggle = true; } }
                if let Some(rect) = self.ribbon_hitboxes.get("ribbon:highlight") { if rect.contains(pt) { hit_toggle = true; } }
                
                if !hit_toggle {
                    self.active_dropdown = DropdownType::None;
                    self.scene_dirty = true;
                }
            }
        }
        
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
        if let Some(rect) = self.ribbon_hitboxes.get("btn_back") {
            if rect.contains(pt) {
                if let Some(id) = self.nav_history.back() {
                    self.selected_node_id = Some(id);
                    self.scene_dirty = true;
                }
            }
        }
        if let Some(rect) = self.ribbon_hitboxes.get("btn_forward") {
            if rect.contains(pt) {
                if let Some(id) = self.nav_history.forward() {
                    self.selected_node_id = Some(id);
                    self.scene_dirty = true;
                }
            }
        }
        if let Some(rect) = self.ribbon_hitboxes.get("nav:back") {
            if rect.contains(pt) {
                if let Some(id) = self.nav_history.back() {
                    self.selected_node_id = Some(id);
                    self.scene_dirty = true;
                }
            }
        }
        if let Some(rect) = self.ribbon_hitboxes.get("nav:forward") {
            if rect.contains(pt) {
                if let Some(id) = self.nav_history.forward() {
                    self.selected_node_id = Some(id);
                    self.scene_dirty = true;
                }
            }
        }
        if let Some(rect) = self.ribbon_hitboxes.get("nav:home") {
            if rect.contains(pt) {
                self.selected_node_id = None;
                self.scene_dirty = true;
            }
        }
        if let Some(rect) = self.ribbon_hitboxes.get("nav:title") {
            if rect.contains(pt) {
                if let Some(id) = &self.selected_node_id {
                    self.is_editing_title = true;
                    self.live_text_buffer = self.workspace.node_title(id).unwrap_or_default();
                    self.scene_dirty = true;
                    return;
                }
            }
        }
        
        if let Some(rect) = self.ribbon_hitboxes.get("btn_shelf") {
            if rect.contains(pt) {
                self.is_shelf_open = !self.is_shelf_open;
                self.scene_dirty = true;
            }
        }

        // --- Ribbon formatting button clicks ---
        if let Some(note_id) = self.selected_node_id.clone() {
            let format_buttons: &[(&str, Attribute)] = &[
                ("ribbon:bold", Attribute::Bold),
                ("ribbon:italic", Attribute::Italic),
                ("ribbon:underline", Attribute::Underline),
                ("ribbon:strikethrough", Attribute::Strikethrough),
                ("ribbon:superscript", Attribute::Superscript),
                ("ribbon:subscript", Attribute::Subscript),
                ("ribbon:highlight", Attribute::Highlight([1.0, 1.0, 0.0, 1.0])),
                ("ribbon:color", Attribute::Color([1.0, 0.2, 0.2, 1.0])),
            ];
            for (btn_id, attr) in format_buttons {
                if let Some(rect) = self.ribbon_hitboxes.get(*btn_id) {
                    if rect.contains(pt) {
                        if let Some((start, end)) = self.cursor.get_selection_range() {
                            if start.0 == end.0 {
                                // For colors/highlights, clear existing ones of the same type first
                                let is_color = matches!(attr, Attribute::Color(_));
                                let is_highlight = matches!(attr, Attribute::Highlight(_));
                                
                                if is_color || is_highlight {
                                    let dummy = if is_color { Attribute::Color([0.0; 4]) } else { Attribute::Highlight([0.0; 4]) };
                                    let _ = onyx_core::editing::clear_attribute_type(
                                        &mut self.workspace, &note_id,
                                        start.0, start.1..end.1, dummy
                                    );
                                    let _ = onyx_core::editing::apply_attribute(
                                        &mut self.workspace, &note_id,
                                        start.0, start.1..end.1, attr.clone()
                                    );
                                } else {
                                    let _ = onyx_core::editing::toggle_attribute(
                                        &mut self.workspace, &note_id,
                                        start.0, start.1..end.1, attr.clone(),
                                    );
                                }
                                self.scene_dirty = true;
                            }
                        } else {
                            if let Some(pos) = self.active_formatting.iter().position(|a| a == attr) {
                                self.active_formatting.remove(pos);
                            } else {
                                // For colors/highlights, replace existing one in active_formatting
                                let is_color = matches!(attr, Attribute::Color(_));
                                let is_highlight = matches!(attr, Attribute::Highlight(_));
                                if is_color || is_highlight {
                                    self.active_formatting.retain(|a| {
                                        match (is_color, a) {
                                            (true, Attribute::Color(_)) => false,
                                            (false, Attribute::Highlight(_)) => false,
                                            _ => true
                                        }
                                    });
                                }
                                self.active_formatting.push(attr.clone());
                            }
                            self.scene_dirty = true;
                        }
                    }
                }
            }

            for (align_id, align_val) in &[("ribbon:align_left", "left"), ("ribbon:align_center", "center"), ("ribbon:align_right", "right"), ("ribbon:align_justify", "justify")] {
                if let Some(rect) = self.ribbon_hitboxes.get(*align_id) {
                    if rect.contains(pt) {
                        let _ = onyx_core::editing::set_block_align(&mut self.workspace, &note_id, self.cursor.block_index, (*align_val).to_string());
                        self.scene_dirty = true;
                    }
                }
            }

            // Font cycling
            if let Some(rect) = self.ribbon_hitboxes.get("ribbon:font_family") {
                if rect.contains(pt) {
                    if matches!(self.active_dropdown, DropdownType::FontFamily) {
                        self.active_dropdown = DropdownType::None;
                    } else {
                        self.active_dropdown = DropdownType::FontFamily;
                        self.dropdown_search.clear();
                    }
                    self.scene_dirty = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                    return;
                }
            }

            // Font Size button
            if let Some(rect) = self.ribbon_hitboxes.get("ribbon:font_size") {
                if rect.contains(pt) {
                    if self.active_dropdown == DropdownType::FontSize {
                        self.active_dropdown = DropdownType::None;
                    } else {
                        self.active_dropdown = DropdownType::FontSize;
                        self.dropdown_search.clear();
                    }
                    self.scene_dirty = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                    return;
                }
            }

            // Color buttons toggles
            if let Some(rect) = self.ribbon_hitboxes.get("ribbon:color") {
                if rect.contains(pt) {
                    // Check if it's hitting the caret right side (width 12px padding) vs the left quick-apply button
                    if pt.x > rect.x1 - 16.0 {
                        if self.active_dropdown == DropdownType::TextColor {
                            self.active_dropdown = DropdownType::None;
                        } else {
                            self.active_dropdown = DropdownType::TextColor;
                            self.dropdown_search.clear();
                        }
                    }
                    let c = self.last_text_color.to_rgba8();
                    let attr = Attribute::Color([
                        c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, c.a as f32 / 255.0
                    ]);
                    if let Some((start, end)) = self.cursor.get_selection_range() {
                        if start.0 == end.0 {
                            let dummy = Attribute::Color([0.0; 4]);
                            let _ = onyx_core::editing::clear_attribute_type(&mut self.workspace, &note_id, start.0, start.1..end.1, dummy);
                            let _ = onyx_core::editing::apply_attribute(&mut self.workspace, &note_id, start.0, start.1..end.1, attr);
                        }
                    }
                    self.scene_dirty = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                    return;
                }
            }

            if let Some(rect) = self.ribbon_hitboxes.get("ribbon:highlight") {
                if rect.contains(pt) {
                    if pt.x > rect.x1 - 16.0 {
                        if self.active_dropdown == DropdownType::HighlightColor {
                            self.active_dropdown = DropdownType::None;
                        } else {
                            self.active_dropdown = DropdownType::HighlightColor;
                            self.dropdown_search.clear();
                        }
                    } else {
                        // Quick Apply
                        let c = self.last_highlight_color.to_rgba8();
                        let attr = Attribute::Highlight([
                            c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, c.a as f32 / 255.0
                        ]);
                        if let Some((start, end)) = self.cursor.get_selection_range() {
                            if start.0 == end.0 {
                                let dummy = Attribute::Highlight([0.0; 4]);
                                let _ = onyx_core::editing::clear_attribute_type(&mut self.workspace, &note_id, start.0, start.1..end.1, dummy);
                                let _ = onyx_core::editing::apply_attribute(&mut self.workspace, &note_id, start.0, start.1..end.1, attr);
                            }
                        }
                    }
                    self.scene_dirty = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                    return;
                }
            }

            // Heading buttons H1-H6
            for level in 1..=6u8 {
                let btn_id = format!("ribbon:h{}", level);
                if let Some(rect) = self.ribbon_hitboxes.get(&btn_id) {
                    if rect.contains(pt) {
                        let _ = onyx_core::editing::set_block_type(
                            &mut self.workspace, &note_id,
                            self.cursor.block_index,
                            onyx_core::blocks::BlockType::Heading(level),
                        );
                        self.scene_dirty = true;
                    }
                }
            }

            // Bullet list button
            if let Some(rect) = self.ribbon_hitboxes.get("ribbon:bullet") {
                if rect.contains(pt) {
                    let blocks = self.workspace.get_note_blocks(&note_id);
                    let new_type = if matches!(blocks.get(self.cursor.block_index).map(|b| &b.kind), Some(onyx_core::blocks::BlockType::BulletList)) {
                        onyx_core::blocks::BlockType::Paragraph
                    } else {
                        onyx_core::blocks::BlockType::BulletList
                    };
                    let _ = onyx_core::editing::set_block_type(
                        &mut self.workspace, &note_id,
                        self.cursor.block_index, new_type,
                    );
                    self.scene_dirty = true;
                }
            }

            // Numbered list button
            if let Some(rect) = self.ribbon_hitboxes.get("ribbon:number") {
                if rect.contains(pt) {
                    let blocks = self.workspace.get_note_blocks(&note_id);
                    let new_type = if matches!(blocks.get(self.cursor.block_index).map(|b| &b.kind), Some(onyx_core::blocks::BlockType::NumberedList)) {
                        onyx_core::blocks::BlockType::Paragraph
                    } else {
                        onyx_core::blocks::BlockType::NumberedList
                    };
                    let _ = onyx_core::editing::set_block_type(
                        &mut self.workspace, &note_id,
                        self.cursor.block_index, new_type,
                    );
                    self.scene_dirty = true;
                }
                // Code block button
            if let Some(rect) = self.ribbon_hitboxes.get("ribbon:code") {
                if rect.contains(pt) {
                    let blocks = self.workspace.get_note_blocks(&note_id);
                    let new_type = if matches!(blocks.get(self.cursor.block_index).map(|b| &b.kind), Some(onyx_core::blocks::BlockType::CodeBlock { .. })) {
                        onyx_core::blocks::BlockType::Paragraph
                    } else {
                        onyx_core::blocks::BlockType::CodeBlock { language: String::new() }
                    };
                    let _ = onyx_core::editing::set_block_type(
                        &mut self.workspace, &note_id,
                        self.cursor.block_index, new_type,
                    );
                    self.scene_dirty = true;
                }
            }

            // Math block button
            if let Some(rect) = self.ribbon_hitboxes.get("ribbon:math") {
                if rect.contains(pt) {
                    let blocks = self.workspace.get_note_blocks(&note_id);
                    let new_type = if matches!(blocks.get(self.cursor.block_index).map(|b| &b.kind), Some(onyx_core::blocks::BlockType::MathBlock)) {
                        onyx_core::blocks::BlockType::Paragraph
                    } else {
                        onyx_core::blocks::BlockType::MathBlock
                    };
                    let _ = onyx_core::editing::set_block_type(
                        &mut self.workspace, &note_id,
                        self.cursor.block_index, new_type,
                    );
                    self.scene_dirty = true;
                }
            }
        }
        }

        // Dynamic hitboxes (Voids/Notes)
        let mut target_id = None;
        for (key, rect) in &self.ribbon_hitboxes {
            if rect.contains(pt) {
                if key.starts_with("void:") || key.starts_with("node:") {
                    target_id = Some(key.split(':').nth(1).unwrap_or_default().to_string());
                }
            }
        }

        if let Some(id) = target_id {
            self.selected_node_id = Some(id.clone());
            self.nav_history.push(id);
            self.cursor.block_index = 0;
            self.cursor.byte_offset = 0;
            self.scene_dirty = true;
        }

        if !self.is_architect_mode && pt.y > 164.0 && self.selected_node_id.is_some() {
            let note_id = self.selected_node_id.as_ref().unwrap();

            // Commit title editing if clicking away
            if self.is_editing_title {
                if let Some(rect) = self.ribbon_hitboxes.get("nav:title") {
                    if !rect.contains(pt) {
                        self.is_editing_title = false;
                        let _ = self.workspace.set_node_title(note_id, &self.live_text_buffer);
                    }
                } else {
                    self.is_editing_title = false;
                    let _ = self.workspace.set_node_title(note_id, &self.live_text_buffer);
                }
            }

            // Check if clicking + icon on hovered block (now on the right side)
            if let Some(hover_idx) = self.hovered_block {
                let blocks = self.workspace.get_note_blocks(&note_id);
                let indent_x = if let Some(b) = blocks.get(hover_idx) { (b.indent_level as f64) * 30.0 } else { 0.0 };
                
                // Compute max_width the same way the renderer does
                let win_width = self.window.as_ref().map(|w| w.inner_size().width as f64 / self.scale_factor).unwrap_or(1200.0);
                let max_width = (win_width - 120.0f64).max(300.0);
                let right_edge = 60.0 + indent_x + max_width + 16.0;

                let mut current_y = 52.0 + 82.0 + 30.0 + 35.0 + 60.0 + 40.0; // 299.0
                for i in 0..hover_idx {
                    if let Some(l) = self.editor.layouts.get(i) {
                        current_y += l.height() as f64 + 24.0;
                    }
                }
                
                let plus_rect = vello::kurbo::Rect::new(
                    right_edge, current_y + 5.0, right_edge + 18.0, current_y + 23.0
                );
                
                if plus_rect.contains(pt) {
                    let _ = onyx_core::editing::insert_block(&mut self.workspace, &note_id, hover_idx + 1);
                    self.cursor.block_index = hover_idx + 1;
                    self.cursor.byte_offset = 0;
                    self.cursor.clear_selection();
                    self.scene_dirty = true;
                    if let Some(w) = &self.window { w.request_redraw(); }
                    return;
                }
            }

            let blocks = self.workspace.get_note_blocks(&note_id);
            let hit_y = pt.y;
            let mut current_y = 52.0 + 82.0 + 30.0 + 35.0 + 60.0 + 40.0; // 299.0
            let mut hit_idx = blocks.len().saturating_sub(1);
            
            for (i, layout) in self.editor.layouts.iter().enumerate() {
                let h = layout.height() as f64 + 24.0;
                if hit_y >= current_y && hit_y < current_y + h {
                    hit_idx = i;
                    break;
                }
                current_y += h;
            }
            
            if hit_idx < blocks.len() {
                self.cursor.block_index = hit_idx;
                
                // Get exact byte offset from our hit test layout helper
                let block_x = pt.x - 60.0; // Subtract total indent 
                // We'll calculate a relative `y` within the block
                let block_y = hit_y - current_y;
                let byte_offset = self.editor.hit_test_x(hit_idx, block_x, block_y);
                
                self.cursor.byte_offset = byte_offset;
                self.cursor.clear_selection();
                self.scene_dirty = true;
            }
        }

        if let Some(w) = &self.window { w.request_redraw(); }
    }

    fn draw_header(&mut self, width: f64) {
        use vello::kurbo::{Rect, Affine};
        use vello::peniko::{Brush, Color, Fill};

        // Static header background
        let header_bg = Brush::Solid(Color::from_rgba8(18, 18, 22, 250));
        let border = Brush::Solid(Color::from_rgba8(40, 40, 45, 255));
        
        self.scene.fill(Fill::NonZero, Affine::IDENTITY, &header_bg, None, &Rect::new(0.0, 0.0, width, 52.0));
        self.scene.fill(Fill::NonZero, Affine::IDENTITY, &border, None, &Rect::new(0.0, 51.0, width, 52.0));

        let text_active = Color::from_rgba8(255, 255, 255, 255);

        // Onyx Logo/Name
        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "Onyx", 20.0, 16.0, 18.0f32, text_active);

        // Shelf Toggle (Right)
        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "≡", width - 40.0, 14.0, 20.0f32, text_active);
        self.ribbon_hitboxes.insert("btn_shelf".into(), Rect::new(width - 50.0, 8.0, width - 10.0, 44.0));
    }

    fn draw_navigation_strip(&mut self, x_pos: f64, y_pos: f64) {
        let text_active = Color::from_rgba8(210, 210, 220, 255);
        let text_dim = Color::from_rgba8(130, 130, 140, 255);

        // Nav Arrows
        let can_back = self.nav_history.index > 0;
        let can_forward = self.nav_history.index + 1 < self.nav_history.stack.len();
        
        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "←", x_pos, y_pos, 16.0f32, if can_back { text_active } else { text_dim });
        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "→", x_pos + 30.0, y_pos, 16.0f32, if can_forward { text_active } else { text_dim });
        
        self.ribbon_hitboxes.insert("nav:back".into(), Rect::new(x_pos - 5.0, y_pos - 5.0, x_pos + 20.0, y_pos + 20.0));
        self.ribbon_hitboxes.insert("nav:forward".into(), Rect::new(x_pos + 25.0, y_pos - 5.0, x_pos + 50.0, y_pos + 20.0));

        // Breadcrumbs
        let mut cur_x = x_pos + 70.0;
        if let Some(note_id) = &self.selected_node_id {
            let path = self.workspace.get_path_to_root(note_id);
            for (_pid, title) in path {
                let segment = format!("{} / ", title);
                draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, &segment, cur_x, y_pos + 2.0, 13.0f32, text_dim);
                cur_x += (segment.len() as f64) * 7.5;
            }
        } else {
            draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "Home", cur_x, y_pos + 2.0, 13.0f32, text_active);
            self.ribbon_hitboxes.insert("nav:home".into(), Rect::new(cur_x, y_pos - 5.0, cur_x + 50.0, y_pos + 20.0));
        }
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

        let width_f = width as f64 / self.scale_factor;
        let height_f = height as f64 / self.scale_factor;

        // The infinite Canvas drawing logic (handled by canvas_renderer in draw())

        // 1. App Header (Shelf toggle)
        self.ribbon_hitboxes.clear();
        self.draw_header(width_f);

        // 2. Ribbon (Contextual tools) — build active_ids from cursor state
        let mut active_ids_owned: Vec<String> = Vec::new();
        if let Some(ref nid) = self.selected_node_id {
            let blocks = self.workspace.get_note_blocks(nid);
            if let Some(block) = blocks.get(self.cursor.block_index) {
                // Check block alignment for active state
                match block.align.as_str() {
                    "left" => active_ids_owned.push("align_left".into()),
                    "center" => active_ids_owned.push("align_center".into()),
                    "right" => active_ids_owned.push("align_right".into()),
                    _ => {}
                }

                // Check which attributes are active at cursor position
                for span in &block.attributes {
                    if span.start <= self.cursor.byte_offset && span.end >= self.cursor.byte_offset {
                        match &span.attr {
                            Attribute::Bold => active_ids_owned.push("bold".into()),
                            Attribute::Italic => active_ids_owned.push("italic".into()),
                            Attribute::Underline => active_ids_owned.push("underline".into()),
                            Attribute::Strikethrough => active_ids_owned.push("strikethrough".into()),
                            _ => {}
                        }
                    }
                }
                // Check block type for heading/list active state
                match &block.kind {
                    onyx_core::blocks::BlockType::Heading(level) => {
                        active_ids_owned.push(format!("h{}", level));
                    }
                    onyx_core::blocks::BlockType::BulletList => active_ids_owned.push("bullet".into()),
                    onyx_core::blocks::BlockType::NumberedList => active_ids_owned.push("number".into()),
                    _ => {}
                }
            }
        }
        let active_ids: Vec<&str> = active_ids_owned.iter().map(|s| s.as_str()).collect();
        let fonts = ["Inter", "Georgia", "Comic Sans MS", "Roboto", "Times New Roman", "Arial"];
        let sizes = ["8", "10", "12", "14", "16", "18", "20", "24", "28", "32", "36", "48", "72"];
        let font_label = fonts[self.font_family_index];
        let size_label = sizes[self.font_size_index];
        crate::ribbon::draw_ribbon(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            width_f,
            52.0,
            &active_ids,
            self.ribbon_hovered.as_deref(),
            font_label,
            size_label,
            self.active_dropdown,
            &self.dropdown_search,
            self.last_text_color,
            self.last_highlight_color,
            &mut self.ribbon_hitboxes,
        );
        
        let mut content_y = 52.0 + 82.0 + 30.0;

        let selected_id = self.selected_node_id.clone();

        match &selected_id {
            Some(id) if !self.is_architect_mode => {
                // 3. Navigation Strip (Arrows + Breadcrumbs)
                self.draw_navigation_strip(80.0, content_y);
                content_y += 35.0;

                // 4. Title Area
                let title = self.workspace.node_title(id).unwrap_or("Untitled".to_string());
                
                if self.is_editing_title {
                    // Draw title prefix and blinking cursor
                    let mut builder = self.layout_cx.ranged_builder(
                        &mut self.font_cx, &self.live_text_buffer, 1.0, false,
                    );
                    builder.push_default(parley::style::StyleProperty::FontSize(42.0));
                    builder.push_default(parley::style::StyleProperty::Brush(vello::peniko::Brush::Solid(Color::WHITE)));
                    let mut layout = builder.build(&self.live_text_buffer);
                    layout.break_all_lines(None);
                    layout.align(None, parley::layout::Alignment::Start, parley::layout::AlignmentOptions::default());
                    
                    parley_vello::render_text(&mut self.scene, vello::kurbo::Affine::translate((80.0, content_y)), &layout);
                    
                    if self.cursor.is_visible {
                        let cursor_x = 80.0 + layout.width() as f64;
                        let cursor_rect = vello::kurbo::Rect::new(cursor_x + 2.0, content_y + 8.0, cursor_x + 4.0, content_y + 48.0);
                        self.scene.fill(vello::peniko::Fill::NonZero, vello::kurbo::Affine::IDENTITY, &vello::peniko::Brush::Solid(Color::WHITE), None, &cursor_rect);
                    }
                    
                    self.ribbon_hitboxes.insert("nav:title".into(), vello::kurbo::Rect::new(80.0, content_y, 80.0 + layout.width() as f64 + 50.0, content_y + 50.0));
                } else {
                    draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, &title, 80.0, content_y, 42.0f32, Color::WHITE);
                    let mut builder = self.layout_cx.ranged_builder(
                        &mut self.font_cx, &title, 1.0, false,
                    );
                    builder.push_default(parley::style::StyleProperty::FontSize(42.0));
                    let mut layout = builder.build(&title);
                    layout.break_all_lines(None);
                    self.ribbon_hitboxes.insert("nav:title".into(), vello::kurbo::Rect::new(80.0, content_y, 80.0 + layout.width() as f64 + 50.0, content_y + 50.0));
                }
                
                content_y += 60.0;

                // 5. Editor Content
                self.editor.draw(
                    &mut self.scene,
                    &self.workspace,
                    id,
                    width_f,
                    content_y,
                    &self.cursor,
                    self.hovered_block,
                );
            }
            Some(id) if self.is_architect_mode => {
                self.canvas_renderer.draw(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, &self.workspace, id, width_f, height_f);
            }
            _ => {
                self.draw_navigation_strip(80.0, content_y);
                content_y += 35.0;
                self.draw_home_view(width_f, content_y);
            }
        }

        self.draw_shelf(width_f, height_f);
        
        // ── DROPDOWNS (Z-Index Fix) ──────────────────────────────────────────
        // Render dropdowns last so they always appear on top of other content
        crate::ribbon::draw_dropdowns(
            &mut self.scene,
            &mut self.font_cx,
            &mut self.layout_cx,
            width_f,
            52.0,
            self.ribbon_hovered.as_deref(),
            self.active_dropdown,
            &self.dropdown_search,
            &mut self.ribbon_hitboxes,
        );

        // Ensure no leftover logic outside the match

        if let Some(renderer) = self.renderer.as_mut() {
            let device = &self.render_cx.devices[device_id];
            if let Some(tex) = &self.cached_target {
                let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
                
                // Wrap everything in a scale transform for High DPI
                let mut root_scene = Scene::new();
                root_scene.append(&self.scene, Some(Affine::scale(self.scale_factor)));

                renderer
                    .render_to_texture(
                        &device.device,
                        &device.queue,
                        &root_scene,
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

    fn draw_home_view(&mut self, width: f64, top_offset: f64) {
        let mut y_pos = top_offset + 60.0;
        let x_pos = 80.0;
        let text_active = Color::from_rgba8(255, 255, 255, 255);
        let text_dim = Color::from_rgba8(150, 150, 160, 255);

        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "Your Voids", x_pos, y_pos, 32.0, text_active);
        y_pos += 60.0;

        let roots = self.workspace.tree.roots();
        let mut grid_x = x_pos;
        for root in roots {
            let id = root.to_string();
            if let Some(node_type) = self.workspace.node_type_of(&id) {
                if node_type == onyx_core::model::NodeType::Void {
                    let title = self.workspace.node_title(&id).unwrap_or("Void".to_string());
                    draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, &format!("🌌 {}", title), grid_x, y_pos, 18.0, text_dim);
                    
                    // Hitbox for navigation
                    self.ribbon_hitboxes.insert(format!("void:{}", id), vello::kurbo::Rect::new(grid_x, y_pos, grid_x + 150.0, y_pos + 30.0));
                    
                    grid_x += 200.0;
                    if grid_x > width - 200.0 {
                        grid_x = x_pos;
                        y_pos += 50.0;
                    }
                }
            }
        }
        
        y_pos += 100.0;
        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "Schedule", x_pos, y_pos, 28.0, text_active);
        y_pos += 50.0;
        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "Due Today", x_pos, y_pos, 18.0, text_dim);
        y_pos += 30.0;
        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "• Calculus III Assignment - 11:59 PM", x_pos + 20.0, y_pos, 14.0, text_dim);

        // 3D Deck (Right Side)
        let deck = crate::renderer::deck::CardDeck::new();
        deck.draw(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, width - 280.0, 150.0);
    }


    fn draw_shelf(&mut self, width: f64, height: f64) {
        if !self.is_shelf_open { return; }
        
        use vello::kurbo::{Rect, Affine};
        use vello::peniko::{Brush, Color, Fill};

        let shelf_w = 300.0;
        let x_pos = width - shelf_w;
        let shelf_bg = Brush::Solid(Color::from_rgba8(15, 15, 18, 255));
        let border = Brush::Solid(Color::from_rgba8(40, 40, 45, 255));
        
        self.scene.fill(Fill::NonZero, Affine::IDENTITY, &shelf_bg, None, &Rect::new(x_pos, 0.0, width, height));
        self.scene.fill(Fill::NonZero, Affine::IDENTITY, &border, None, &Rect::new(x_pos, 0.0, x_pos + 1.0, height));
        
        let text_active = Color::from_rgba8(255, 255, 255, 255);
        let text_dim = Color::from_rgba8(140, 140, 150, 255);

        // Properties section
        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "Properties", x_pos + 20.0, 80.0, 20.0, text_active);
        
        // Mock properties
        let props = [("Status", "Doing"), ("Priority", "High"), ("Due", "Today")];
        let mut py = 120.0;
        for (k, v) in props {
            draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, k, x_pos + 25.0, py, 14.0, text_dim);
            draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, v, x_pos + 120.0, py, 14.0, text_active);
            py += 30.0;
        }

        // Utilities
        py += 40.0;
        draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, "Utilities", x_pos + 20.0, py, 20.0, text_active);
        py += 40.0;
        let utils = ["Flashcards", "Question Bank", "Password Mgr", "Calendar"];
        for u in utils {
            draw_text(&mut self.scene, &mut self.font_cx, &mut self.layout_cx, u, x_pos + 25.0, py, 15.0, text_dim);
            py += 30.0;
        }
    }
}

impl ApplicationHandler for OnyxApp {
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

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
        self.window = Some(window.clone());
        self.scale_factor = window.scale_factor();
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
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
                self.scene_dirty = true;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                use winit::event::MouseScrollDelta;
                let zoom_factor = match delta {
                    MouseScrollDelta::LineDelta(_, y) => (y as f64 * 0.1).exp(),
                    MouseScrollDelta::PixelDelta(pos) => (pos.y * 0.001).exp(),
                };
                self.canvas_renderer.zoom *= zoom_factor;
                self.scene_dirty = true;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let dx = position.x - self.last_mouse_pos.0;
                let dy = position.y - self.last_mouse_pos.1;
                self.last_mouse_pos = (position.x, position.y);
                self.cursor_pos = (position.x, position.y);

                let pt = vello::kurbo::Point::new(position.x / self.scale_factor, position.y / self.scale_factor);
                let mut hovered = None;
                for (id, rect) in &self.ribbon_hitboxes {
                    if rect.contains(pt) {
                        hovered = Some(id.clone());
                        break;
                    }
                }
                if self.ribbon_hovered != hovered {
                    self.ribbon_hovered = hovered;
                    self.scene_dirty = true;
                }

                if !self.is_architect_mode && pt.y > 164.0 && self.selected_node_id.is_some() {
                    let hit_y = pt.y;
                    // match the rendering offset: ribbon(52+82+30) + nav(35) + title(60) + editor_margin(40) = 299.0
                    let mut current_y = 52.0 + 82.0 + 30.0 + 35.0 + 60.0 + 40.0; 
                    let mut hit_idx = None;
                    
                    for (i, layout) in self.editor.layouts.iter().enumerate() {
                        let h = layout.height() as f64 + 24.0;
                        if hit_y >= current_y && hit_y < current_y + h {
                            hit_idx = Some(i);
                            break;
                        }
                        current_y += h;
                    }

                    // To fix the "plus button disappears" bug, if the mouse is to the right
                    // of the layout, we still want it to count as hovering the block if it's over the + button area.
                    if hit_idx.is_some() {
                        if self.hovered_block != hit_idx {
                            self.hovered_block = hit_idx;
                            self.scene_dirty = true;
                        }
                    } else if let Some(hover_idx) = self.hovered_block {
                        // Check if the mouse is still within the hovered_block's row height (even if far to the right)
                        let mut still_hovered = false;
                        let mut block_y = 52.0 + 82.0 + 30.0 + 35.0 + 60.0 + 40.0;
                        for (i, layout) in self.editor.layouts.iter().enumerate() {
                            let h = layout.height() as f64 + 24.0;
                            if i == hover_idx {
                                if hit_y >= block_y && hit_y < block_y + h {
                                    still_hovered = true;
                                }
                                break;
                            }
                            block_y += h;
                        }
                        
                        if !still_hovered {
                            self.hovered_block = None;
                            self.scene_dirty = true;
                        }
                    }
                } else if self.hovered_block.is_some() {
                    self.hovered_block = None;
                    self.scene_dirty = true;
                }

                if self.is_panning && self.is_architect_mode {
                    self.canvas_renderer.offset.x += dx;
                    self.canvas_renderer.offset.y += dy;
                    self.scene_dirty = true;
                }

                if self.is_dragging_text {
                    self.handle_drag();
                }

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state,
                button,
                ..
            } => {
                if button == MouseButton::Right {
                    self.is_panning = state == ElementState::Pressed;
                }
                if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        self.handle_click();
                        self.is_dragging_text = true;
                    } else {
                        self.is_dragging_text = false;
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    let mut text_changed = false;
                    let mut nav_changed = false;
                    let ctrl = self.modifiers.control_key();
                    let alt = self.modifiers.alt_key();
                    let shift = self.modifiers.shift_key();

                    if let Some(note_id) = self.selected_node_id.clone() {
                        if !matches!(self.active_dropdown, DropdownType::None) {
                            match &event.logical_key {
                                Key::Character(s) => {
                                    self.dropdown_search.push_str(s);
                                    text_changed = true;
                                }
                                Key::Named(NamedKey::Backspace) => {
                                    self.dropdown_search.pop();
                                    text_changed = true;
                                }
                                Key::Named(NamedKey::Enter) => {
                                    self.active_dropdown = DropdownType::None;
                                    text_changed = true;
                                }
                                Key::Named(NamedKey::Escape) => {
                                    self.active_dropdown = DropdownType::None;
                                    text_changed = true;
                                }
                                Key::Named(NamedKey::ArrowDown) => {
                                    if self.active_dropdown == DropdownType::FontFamily {
                                        self.font_family_index = (self.font_family_index + 1).min(5);
                                    } else if self.active_dropdown == DropdownType::FontSize {
                                        self.font_size_index = (self.font_size_index + 1).min(12);
                                    }
                                    text_changed = true;
                                }
                                Key::Named(NamedKey::ArrowUp) => {
                                    if self.active_dropdown == DropdownType::FontFamily {
                                        self.font_family_index = self.font_family_index.saturating_sub(1);
                                    } else if self.active_dropdown == DropdownType::FontSize {
                                        self.font_size_index = self.font_size_index.saturating_sub(1);
                                    }
                                    text_changed = true;
                                }
                                _ => {}
                            }
                        } else if self.is_editing_title {
                            match &event.logical_key {
                                Key::Character(s) => {
                                    self.live_text_buffer.push_str(s);
                                    let _ = self.workspace.set_node_title(&note_id, &self.live_text_buffer);
                                    text_changed = true;
                                }
                                Key::Named(NamedKey::Backspace) => {
                                    self.live_text_buffer.pop();
                                    let _ = self.workspace.set_node_title(&note_id, &self.live_text_buffer);
                                    text_changed = true;
                                }
                                Key::Named(NamedKey::Enter) | Key::Named(NamedKey::Escape) => {
                                    self.is_editing_title = false;
                                    let _ = self.workspace.set_node_title(&note_id, &self.live_text_buffer);
                                    text_changed = true;
                                }
                                _ => {}
                            }
                        } else {
                            match &event.logical_key {
                                Key::Character(s) => {
                                    if ctrl && (s == "b" || s == "B") {
                                        if let Some((start, end)) = self.cursor.get_selection_range() {
                                            if start.0 == end.0 {
                                                let _ = onyx_core::editing::toggle_attribute(
                                                    &mut self.workspace, &note_id,
                                                    start.0, start.1..end.1,
                                                    Attribute::Bold,
                                                );
                                                text_changed = true;
                                            }
                                        } else {
                                            let attr = Attribute::Bold;
                                            if let Some(pos) = self.active_formatting.iter().position(|a| a == &attr) {
                                                self.active_formatting.remove(pos);
                                            } else {
                                                self.active_formatting.push(attr);
                                            }
                                        }
                                    } else if ctrl && (s == "i" || s == "I") {
                                        if let Some((start, end)) = self.cursor.get_selection_range() {
                                            if start.0 == end.0 {
                                                let _ = onyx_core::editing::toggle_attribute(
                                                    &mut self.workspace, &note_id,
                                                    start.0, start.1..end.1,
                                                    Attribute::Italic,
                                                );
                                                text_changed = true;
                                            }
                                        } else {
                                            let attr = Attribute::Italic;
                                            if let Some(pos) = self.active_formatting.iter().position(|a| a == &attr) {
                                                self.active_formatting.remove(pos);
                                            } else {
                                                self.active_formatting.push(attr);
                                            }
                                        }
                                    } else if ctrl && (s == "u" || s == "U") {
                                        if let Some((start, end)) = self.cursor.get_selection_range() {
                                            if start.0 == end.0 {
                                                let _ = onyx_core::editing::toggle_attribute(
                                                    &mut self.workspace, &note_id,
                                                    start.0, start.1..end.1,
                                                    Attribute::Underline,
                                                );
                                                text_changed = true;
                                            }
                                        } else {
                                            let attr = Attribute::Underline;
                                            if let Some(pos) = self.active_formatting.iter().position(|a| a == &attr) {
                                                self.active_formatting.remove(pos);
                                            } else {
                                                self.active_formatting.push(attr);
                                            }
                                        }
                                    } else if !ctrl && !alt {
                                        let _ = onyx_core::editing::insert_text(
                                            &mut self.workspace, &note_id,
                                            self.cursor.block_index, self.cursor.byte_offset, s,
                                            Some(&self.active_formatting),
                                        );
                                        self.cursor.byte_offset += s.len();
                                        self.cursor.clear_selection();
                                        text_changed = true;
                                    }
                                }
                                Key::Named(NamedKey::Tab) => {
                                    if shift {
                                        let _ = onyx_core::editing::decrease_indent(&mut self.workspace, &note_id, self.cursor.block_index);
                                    } else {
                                        let _ = onyx_core::editing::increase_indent(&mut self.workspace, &note_id, self.cursor.block_index);
                                    }
                                    text_changed = true;
                                }
                                Key::Named(NamedKey::Space) => {
                                    let _ = onyx_core::editing::insert_text(
                                        &mut self.workspace, &note_id,
                                        self.cursor.block_index, self.cursor.byte_offset, " ",
                                        Some(&self.active_formatting),
                                    );
                                    self.cursor.byte_offset += 1;
                                    self.cursor.clear_selection();
                                    text_changed = true;
                                }
                                Key::Named(NamedKey::Delete) => {
                                    let blocks = self.workspace.get_note_blocks(&note_id);
                                    if let Some(block) = blocks.get(self.cursor.block_index) {
                                        if self.cursor.byte_offset < block.content.len() {
                                            let _ = onyx_core::editing::delete_text(
                                                &mut self.workspace, &note_id,
                                                self.cursor.block_index, self.cursor.byte_offset, 1
                                            );
                                            text_changed = true;
                                        } else if self.cursor.block_index + 1 < blocks.len() {
                                            let _ = onyx_core::editing::merge_blocks(
                                                &mut self.workspace, &note_id, self.cursor.block_index + 1
                                            );
                                            text_changed = true;
                                        }
                                    }
                                }
                                Key::Named(NamedKey::Backspace) => {
                                    if let Some((start, end)) = self.cursor.get_selection_range() {
                                        if start.0 == end.0 {
                                            let _ = onyx_core::editing::delete_text(
                                                &mut self.workspace, &note_id,
                                                start.0, start.1, end.1 - start.1,
                                            );
                                            self.cursor.block_index = start.0;
                                            self.cursor.byte_offset = start.1;
                                            self.cursor.clear_selection();
                                            text_changed = true;
                                        }
                                    } else if self.cursor.byte_offset == 0 {
                                        let blocks = self.workspace.get_note_blocks(&note_id);
                                        if let Some(block) = blocks.get(self.cursor.block_index) {
                                            if block.indent_level > 0 {
                                                let _ = onyx_core::editing::decrease_indent(&mut self.workspace, &note_id, self.cursor.block_index);
                                                text_changed = true;
                                            } else if self.cursor.block_index > 0 {
                                                let prev_len = blocks[self.cursor.block_index - 1].content.len();
                                                let _ = onyx_core::editing::merge_blocks(&mut self.workspace, &note_id, self.cursor.block_index);
                                                self.cursor.block_index -= 1;
                                                self.cursor.byte_offset = prev_len;
                                                text_changed = true;
                                            }
                                        }
                                    } else {
                                        let blocks = self.workspace.get_note_blocks(&note_id);
                                        if let Some(block) = blocks.get(self.cursor.block_index) {
                                            let prev = block.content[..self.cursor.byte_offset]
                                                .char_indices().last().map(|(idx, _)| idx).unwrap_or(0);
                                            let del_len = self.cursor.byte_offset - prev;
                                            let _ = onyx_core::editing::delete_text(
                                                &mut self.workspace, &note_id,
                                                self.cursor.block_index, prev, del_len
                                            );
                                            self.cursor.byte_offset = prev;
                                            text_changed = true;
                                        }
                                    }
                                }
                                Key::Named(NamedKey::Enter) => {
                                    let _ = onyx_core::editing::split_block(
                                        &mut self.workspace, &note_id,
                                        self.cursor.block_index, self.cursor.byte_offset,
                                    );
                                    self.cursor.block_index += 1;
                                    self.cursor.byte_offset = 0;
                                    self.cursor.clear_selection();
                                    text_changed = true;
                                }
                                Key::Named(NamedKey::ArrowLeft) => {
                                    self.active_formatting.clear();
                                    if self.cursor.byte_offset > 0 {
                                        let blocks = self.workspace.get_note_blocks(&note_id);
                                        if let Some(block) = blocks.get(self.cursor.block_index) {
                                            let prev = block.content[..self.cursor.byte_offset]
                                                .char_indices().last().map(|(idx, _)| idx).unwrap_or(0);
                                            self.cursor.move_to(self.cursor.block_index, prev, shift);
                                        }
                                        nav_changed = true;
                                    } else if self.cursor.block_index > 0 {
                                        let new_block = self.cursor.block_index - 1;
                                        let blocks = self.workspace.get_note_blocks(&note_id);
                                        let new_off = blocks[new_block].content.len();
                                        self.cursor.move_to(new_block, new_off, shift);
                                        nav_changed = true;
                                    }
                                }
                                Key::Named(NamedKey::ArrowRight) => {
                                    self.active_formatting.clear();
                                    let blocks = self.workspace.get_note_blocks(&note_id);
                                    if let Some(block) = blocks.get(self.cursor.block_index) {
                                        if self.cursor.byte_offset < block.content.len() {
                                            let next = block.content[self.cursor.byte_offset..]
                                                .char_indices().nth(1).map(|(idx, _)| self.cursor.byte_offset + idx)
                                                .unwrap_or(block.content.len());
                                            self.cursor.move_to(self.cursor.block_index, next, shift);
                                            nav_changed = true;
                                        } else if self.cursor.block_index + 1 < blocks.len() {
                                            self.cursor.move_to(self.cursor.block_index + 1, 0, shift);
                                            nav_changed = true;
                                        }
                                    }
                                }
                                Key::Named(NamedKey::ArrowUp) => {
                                    if ctrl {
                                        let sizes = [8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0, 48.0, 72.0];
                                        if self.font_size_index + 1 < sizes.len() {
                                            self.font_size_index += 1;
                                            let size = sizes[self.font_size_index];
                                            self.active_formatting.retain(|a| !matches!(a, Attribute::FontSize(_)));
                                            self.active_formatting.push(Attribute::FontSize(size));
                                            if let Some((start, end)) = self.cursor.get_selection_range() {
                                                if start.0 == end.0 {
                                                    let dummy = Attribute::FontSize(14.0);
                                                    let _ = onyx_core::editing::clear_attribute_type(
                                                        &mut self.workspace, &note_id,
                                                        start.0, start.1..end.1, dummy
                                                    );
                                                    let _ = onyx_core::editing::apply_attribute(
                                                        &mut self.workspace, &note_id,
                                                        start.0, start.1..end.1,
                                                        Attribute::FontSize(size)
                                                    );
                                                    text_changed = true;
                                                }
                                            }
                                        }
                                    } else if alt {
                                        if self.cursor.block_index > 0 {
                                            let _ = onyx_core::editing::move_block_up(&mut self.workspace, &note_id, self.cursor.block_index);
                                            self.cursor.block_index -= 1;
                                            text_changed = true;
                                        }
                                    } else if self.cursor.block_index > 0 {
                                        let new_block = self.cursor.block_index - 1;
                                        let blocks = self.workspace.get_note_blocks(&note_id);
                                        let new_off = self.cursor.byte_offset.min(blocks[new_block].content.len());
                                        self.cursor.move_to(new_block, new_off, shift);
                                        nav_changed = true;
                                    }
                                }
                                Key::Named(NamedKey::ArrowDown) => {
                                    self.active_formatting.clear();
                                    if ctrl {
                                        let sizes = [8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0, 48.0, 72.0];
                                        if self.font_size_index > 0 {
                                            self.font_size_index -= 1;
                                            let size = sizes[self.font_size_index];
                                            self.active_formatting.retain(|a| !matches!(a, Attribute::FontSize(_)));
                                            self.active_formatting.push(Attribute::FontSize(size));
                                            if let Some((start, end)) = self.cursor.get_selection_range() {
                                                if start.0 == end.0 {
                                                    let dummy = Attribute::FontSize(14.0);
                                                    let _ = onyx_core::editing::clear_attribute_type(
                                                        &mut self.workspace, &note_id,
                                                        start.0, start.1..end.1, dummy
                                                    );
                                                    let _ = onyx_core::editing::apply_attribute(
                                                        &mut self.workspace, &note_id,
                                                        start.0, start.1..end.1,
                                                        Attribute::FontSize(size)
                                                    );
                                                    text_changed = true;
                                                }
                                            }
                                        }
                                    } else if alt {
                                        let blocks = self.workspace.get_note_blocks(&note_id);
                                        if self.cursor.block_index + 1 < blocks.len() {
                                            let _ = onyx_core::editing::move_block_down(&mut self.workspace, &note_id, self.cursor.block_index);
                                            self.cursor.block_index += 1;
                                            text_changed = true;
                                        }
                                    } else {
                                        let blocks = self.workspace.get_note_blocks(&note_id);
                                        if self.cursor.block_index + 1 < blocks.len() {
                                            let new_block = self.cursor.block_index + 1;
                                            let new_off = self.cursor.byte_offset.min(blocks[new_block].content.len());
                                            self.cursor.move_to(new_block, new_off, shift);
                                            nav_changed = true;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    if text_changed || nav_changed {
                        self.scene_dirty = true;
                        self.cursor.reset_blink();
                        if let Some(w) = &self.window { w.request_redraw(); }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let elapsed = self.start_time.elapsed().as_secs_f64();
                if elapsed - self.cursor.last_blink_time > 0.5 {
                    self.cursor.is_visible = !self.cursor.is_visible;
                    self.cursor.last_blink_time = elapsed;
                    self.scene_dirty = true;
                }
                self.draw();
            }
            _ => {}
        }
    }
}
