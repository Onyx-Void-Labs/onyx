// ─── Onyx Void — Application Orchestrator ──────────────────────────
// Initialises Parley fonts, wgpu, Vello renderer, and the custom
// borderless window.  Drives the render loop:
//
//   Background → Spine → Document rows → GlassHUD → WindowControls → Dock
// ────────────────────────────────────────────────────────────────────

use std::num::NonZeroUsize;
use std::sync::Arc;

use parley::{FontContext, LayoutContext};
use vello::kurbo::{Affine, Rect, RoundedRect, Size, Stroke};
use vello::peniko::{self, Brush, Fill};
use vello::wgpu;
use vello::{AaConfig, Renderer, RendererOptions, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::ui::chrome::{FormattingRibbon, HoveredButton, PathBar, RibbonHit, WindowControls};
use crate::widgets::editor::LaneEditor;
use crate::widgets::text::TextWidget;
use crate::widgets::{Action, LayoutContext as OnyxLayoutCtx, Widget};
use crate::window::WindowContext;

// ─── Palette ───────────────────────────────────────────────────────

/// Background — #09090b.
const ONYX_BLACK: peniko::Color = peniko::Color::from_rgba8(0x09, 0x09, 0x0b, 0xff);
/// Spine panel — slightly lighter.
const SPINE_BG: peniko::Color = peniko::Color::from_rgba8(0x0f, 0x0f, 0x12, 0x60);
/// Spine border — zinc-800.
const SPINE_BORDER: peniko::Color = peniko::Color::from_rgba8(0x27, 0x27, 0x2a, 0x40);
/// Primary text — zinc-200.
const ZINC_200: peniko::Color = peniko::Color::from_rgba8(228, 228, 231, 255);
/// Accent — blue-600.
const BLUE_600: peniko::Color = peniko::Color::from_rgba8(0x25, 0x63, 0xeb, 0xff);

// ─── Render State ──────────────────────────────────────────────────

/// Live GPU state, created once the surface is ready.
struct RenderState {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    target_texture: wgpu::Texture,
    target_view: wgpu::TextureView,
    blitter: wgpu::util::TextureBlitter,
}

// ─── OnyxApp ───────────────────────────────────────────────────────

/// Top-level application driven by `winit`.
pub struct OnyxApp {
    render: Option<RenderState>,
    wctx: Option<WindowContext>,
    scene: Scene,

    // Typography
    font_cx: FontContext,
    layout_cx: LayoutContext<Brush>,

    // Chrome
    ribbon: FormattingRibbon,
    path_bar: PathBar,
    hover_button: Option<HoveredButton>,

    // Document content
    title_text: TextWidget,
    editor: LaneEditor,

    // DPI
    scale_factor: f64,

    // Flags
    layouts_dirty: bool,
    needs_redraw: bool,
}

impl Default for OnyxApp {
    fn default() -> Self {
        Self {
            render: None,
            wctx: None,
            scene: Scene::new(),
            font_cx: FontContext::new(),
            layout_cx: LayoutContext::new(),
            ribbon: FormattingRibbon::new(15.0),
            path_bar: PathBar::new(&["Root", "Workspace"]),
            hover_button: None,
            title_text: TextWidget::new("Onyx Void", 28.0, ZINC_200),
            editor: LaneEditor::new(
                "Welcome to the void. Start typing to create.",
                15.0,
                peniko::Color::from_rgba8(0xa1, 0xa1, 0xaa, 0xff),
            ),
            scale_factor: 1.0,
            layouts_dirty: true,
            needs_redraw: true,
        }
    }
}

impl OnyxApp {
    /// Run all text layout passes when dirty.
    fn build_layouts(&mut self) {
        if !self.layouts_dirty {
            return;
        }
        let scale = self.scale_factor;
        let mut cx = OnyxLayoutCtx {
            font_cx: &mut self.font_cx,
            layout_cx: &mut self.layout_cx,
            scale_factor: scale,
        };
        let spine_w = 850.0 * scale;
        let wide = Size::new(spine_w, 2000.0);
        self.title_text.layout(&mut cx, wide);
        self.editor.layout(&mut cx, wide);
        self.ribbon.layout_labels(&mut cx);
        self.path_bar.layout_all(&mut cx);
        self.layouts_dirty = false;
    }

    /// Build the complete frame scene.
    fn render_scene(
        scene: &mut Scene,
        phys_w: f64,
        phys_h: f64,
        scale: f64,
        title: &TextWidget,
        editor: &LaneEditor,
        ribbon: &FormattingRibbon,
        path_bar: &PathBar,
        hover_button: Option<HoveredButton>,
        is_maximized: bool,
    ) {
        scene.reset();

        // 1. Background
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            ONYX_BLACK,
            None,
            &Rect::new(0.0, 0.0, phys_w, phys_h),
        );

        // 2. Window Controls (top)
        WindowControls::draw(scene, phys_w, hover_button, is_maximized);

        // 3. PathBar (top-left, below title bar)
        path_bar.draw(scene, 54.0);

        // 4. Spine — 850pt centered rect, scaled to physical pixels
        let spine_w = (850.0 * scale).min(phys_w - 60.0);
        let spine_x = (phys_w - spine_w) / 2.0;
        let spine_y = 110.0;
        let spine_h = phys_h - 200.0;
        let spine_rect =
            RoundedRect::new(spine_x, spine_y, spine_x + spine_w, spine_y + spine_h, 8.0);
        scene.fill(Fill::NonZero, Affine::IDENTITY, SPINE_BG, None, &spine_rect);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            SPINE_BORDER,
            None,
            &spine_rect,
        );

        // 5. Document title
        let title_x = spine_x + 40.0;
        let title_y = spine_y + 40.0;
        let title_sz = title.cached_size();
        title.draw(
            scene,
            Rect::new(
                title_x,
                title_y,
                title_x + title_sz.width,
                title_y + title_sz.height,
            ),
        );

        // 6. Editor (replaces static body rows)
        let editor_y = title_y + title_sz.height + 24.0;
        let editor_sz = editor.cached_size();
        editor.draw(
            scene,
            Rect::new(
                title_x,
                editor_y,
                title_x + editor_sz.width,
                editor_y + editor_sz.height,
            ),
        );

        // 7. FormattingRibbon (bottom)
        ribbon.draw(scene, phys_w, phys_h);
    }
}

// ─── ApplicationHandler ────────────────────────────────────────────

impl ApplicationHandler for OnyxApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.render.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("Onyx Void")
            .with_decorations(false)
            .with_transparent(true)
            .with_inner_size(LogicalSize::new(1280.0_f64, 800.0));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("failed to create window"),
        );

        // Windows OS drop-shadow for borderless windows.
        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::WindowExtWindows;
            window.set_undecorated_shadow(true);
        }

        // ── wgpu bootstrap ──
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("failed to create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("no suitable GPU adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("onyx-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .expect("failed to create wgpu device");

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| {
                matches!(
                    f,
                    wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
                )
            })
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: if surface_caps
                .present_modes
                .contains(&wgpu::PresentMode::Mailbox)
            {
                wgpu::PresentMode::Mailbox
            } else {
                wgpu::PresentMode::Fifo
            },
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let (target_texture, target_view) =
            create_target_texture(&device, config.width, config.height);
        let blitter = wgpu::util::TextureBlitter::new(&device, surface_format);

        let renderer = Renderer::new(
            &device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: NonZeroUsize::new(1),
                ..Default::default()
            },
        )
        .expect("failed to create vello renderer");

        self.wctx = Some(WindowContext::new(Arc::clone(&window)));
        self.scale_factor = window.scale_factor();
        self.layouts_dirty = true;

        self.render = Some(RenderState {
            surface,
            config,
            device,
            queue,
            renderer,
            target_texture,
            target_view,
            blitter,
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.render.is_none() || self.wctx.is_none() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                let rs = self.render.as_mut().unwrap();
                let wctx = self.wctx.as_mut().unwrap();
                let w = new_size.width.max(1);
                let h = new_size.height.max(1);
                rs.config.width = w;
                rs.config.height = h;
                rs.surface.configure(&rs.device, &rs.config);
                let (tex, view) = create_target_texture(&rs.device, w, h);
                rs.target_texture = tex;
                rs.target_view = view;
                wctx.resize(w, h);
                wctx.window.request_redraw();
            }

            WindowEvent::CursorMoved { position, .. } => {
                let wctx = self.wctx.as_mut().unwrap();
                wctx.update_cursor(position.x as f32, position.y as f32);

                let new_hover = WindowControls::hovered_button(
                    wctx.cursor_pos.0,
                    wctx.cursor_pos.1,
                    wctx.window_size.0,
                );
                if new_hover != self.hover_button {
                    self.hover_button = new_hover;
                    wctx.window.request_redraw();
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let wctx = self.wctx.as_mut().unwrap();
                wctx.scale_factor = scale_factor;
                self.scale_factor = scale_factor;
                self.layouts_dirty = true;
                wctx.window.request_redraw();
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let wctx = self.wctx.as_mut().unwrap();

                // Ribbon hit-test (physical coords)
                let rs = self.render.as_ref().unwrap();
                let phys_w = rs.config.width as f64;
                let phys_h = rs.config.height as f64;
                let (cx, cy) = (wctx.cursor_pos.0 as f64, wctx.cursor_pos.1 as f64);
                if let Some(hit) = self.ribbon.hit_test(cx, cy, phys_w, phys_h) {
                    match hit {
                        RibbonHit::Bold => {
                            self.ribbon.is_bold = !self.ribbon.is_bold;
                        }
                        RibbonHit::Italic => {
                            self.ribbon.is_italic = !self.ribbon.is_italic;
                        }
                        RibbonHit::FontSizeMinus => {
                            if self.editor.font_size > 8.0 {
                                self.editor.font_size -= 1.0;
                                self.ribbon.font_size = self.editor.font_size;
                                self.ribbon.update_size_label();
                            }
                        }
                        RibbonHit::FontSizePlus => {
                            if self.editor.font_size < 72.0 {
                                self.editor.font_size += 1.0;
                                self.ribbon.font_size = self.editor.font_size;
                                self.ribbon.update_size_label();
                            }
                        }
                        RibbonHit::Settings => {}
                    }
                    self.layouts_dirty = true;
                    self.needs_redraw = true;
                    wctx.window.request_redraw();
                } else {
                    wctx.handle_click(event_loop);
                    wctx.window.request_redraw();
                }
            }

            // Route keyboard input to the editor
            ref evt @ WindowEvent::KeyboardInput { .. } => {
                if self.editor.handle_event(evt) == Action::Redraw {
                    self.layouts_dirty = true;
                    self.needs_redraw = true;
                    let wctx = self.wctx.as_ref().unwrap();
                    wctx.window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                // Build text layouts before borrowing render state.
                self.build_layouts();

                let rs = self.render.as_mut().unwrap();
                let wctx = self.wctx.as_ref().unwrap();
                let phys_w = rs.config.width;
                let phys_h = rs.config.height;

                Self::render_scene(
                    &mut self.scene,
                    phys_w as f64,
                    phys_h as f64,
                    self.scale_factor,
                    &self.title_text,
                    &self.editor,
                    &self.ribbon,
                    &self.path_bar,
                    self.hover_button,
                    wctx.window.is_maximized(),
                );

                let surface_texture = match rs.surface.get_current_texture() {
                    Ok(t) => t,
                    Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                        rs.surface.configure(&rs.device, &rs.config);
                        return;
                    }
                    Err(e) => {
                        tracing::error!("surface error: {e}");
                        return;
                    }
                };

                rs.renderer
                    .render_to_texture(
                        &rs.device,
                        &rs.queue,
                        &self.scene,
                        &rs.target_view,
                        &vello::RenderParams {
                            base_color: ONYX_BLACK,
                            width: phys_w,
                            height: phys_h,
                            antialiasing_method: AaConfig::Area,
                        },
                    )
                    .expect("vello render failed");

                let surface_view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    rs.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("blit"),
                        });
                rs.blitter
                    .copy(&rs.device, &mut encoder, &rs.target_view, &surface_view);
                rs.queue.submit(Some(encoder.finish()));

                surface_texture.present();

                self.needs_redraw = false;

                // Cursor blink drives the next redraw via about_to_wait.
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // PERF: ControlFlow::Poll is set once in main(). Always request redraw
        // so the cursor blink stays animated and input is never delayed.
        if let Some(wctx) = &self.wctx {
            wctx.window.request_redraw();
        }
    }
}

// ─── Helpers ───────────────────────────────────────────────────────

/// Create an intermediate Rgba8Unorm texture for vello's compute pipeline.
fn create_target_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("vello-target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        format: wgpu::TextureFormat::Rgba8Unorm,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}
