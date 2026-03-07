// --- Onyx Void — Library Entry Point (Vello Stack) ---
#![allow(dead_code, unused_imports)]

pub mod widgets;

use std::num::NonZeroUsize;
use std::sync::Arc;

use parley::{FontContext, LayoutContext};
use vello::kurbo::{Affine, Circle};
use vello::peniko::{self, color::palette, Brush, Fill};
use vello::wgpu;
use vello::{AaConfig, Renderer, RendererOptions, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{CursorIcon, ResizeDirection, Window, WindowId};

use widgets::text::SimpleText;
use widgets::titlebar;
use widgets::Widget;

/// Onyx Black — #09090b
const ONYX_BLACK: vello::peniko::Color = vello::peniko::Color::from_rgba8(0x09, 0x09, 0x0b, 0xff);

const EDGE: f32 = 6.0;
const TITLE_H: f32 = 40.0;
const BTN_W: f32 = 46.0;

/// Hit-test regions for the custom window chrome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitRegion {
    TitleBar,
    Close,
    Minimise,
    Maximise,
    ResizeN,
    ResizeS,
    ResizeE,
    ResizeW,
    ResizeNE,
    ResizeNW,
    ResizeSE,
    ResizeSW,
    Content,
}

/// The live render state, created once the window surface is ready.
struct RenderState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    // Intermediate Rgba8Unorm texture — vello's compute shaders always write
    // this format. We blit from here to the surface (which may be Bgra8Unorm).
    target_texture: wgpu::Texture,
    target_view: wgpu::TextureView,
    blitter: wgpu::util::TextureBlitter,
}

/// Top-level application driven by `winit`.
pub struct OnyxApp {
    state: Option<RenderState>,
    scene: Scene,
    font_cx: FontContext,
    layout_cx: LayoutContext<Brush>,
    title_text: SimpleText,
    dock: widgets::dock::CommandDock,
    /// When true, `RedrawRequested` will continuously schedule redraws.
    is_animating: bool,
    /// Last known cursor position (logical pixels).
    cursor_pos: (f32, f32),
    /// Stored window size in logical pixels for hit testing.
    window_size: (f32, f32),
    /// Which titlebar button is currently hovered.
    hover_button: Option<titlebar::HoveredButton>,
    /// For double-click detection on the title bar.
    last_click_time: Option<std::time::Instant>,
}

impl Default for OnyxApp {
    fn default() -> Self {
        Self {
            state: None,
            scene: Scene::new(),
            font_cx: FontContext::new(),
            layout_cx: LayoutContext::new(),
            title_text: SimpleText::new(
                "Onyx Void",
                32.0,
                peniko::Color::from_rgba8(228, 228, 231, 255),
            ),
            dock: widgets::dock::CommandDock::new(),
            is_animating: false,
            cursor_pos: (0.0, 0.0),
            window_size: (1280.0, 800.0),
            hover_button: None,
            last_click_time: None,
        }
    }
}

/// Determine the hit-test region for the given cursor coordinates.
fn hit_test_region(x: f32, y: f32, w: f32, h: f32) -> HitRegion {
    // --- Corners first (EDGE x EDGE squares) ---
    if x < EDGE && y < EDGE {
        return HitRegion::ResizeNW;
    }
    if x >= w - EDGE && y < EDGE {
        return HitRegion::ResizeNE;
    }
    if x < EDGE && y >= h - EDGE {
        return HitRegion::ResizeSW;
    }
    if x >= w - EDGE && y >= h - EDGE {
        return HitRegion::ResizeSE;
    }

    // --- Edges ---
    if y < EDGE {
        return HitRegion::ResizeN;
    }
    if y >= h - EDGE {
        return HitRegion::ResizeS;
    }
    if x < EDGE {
        return HitRegion::ResizeW;
    }
    if x >= w - EDGE {
        return HitRegion::ResizeE;
    }

    // --- Title bar buttons (rightmost first) ---
    if x >= w - 46.0 && y < TITLE_H {
        return HitRegion::Close;
    }
    if x >= w - 92.0 && x < w - 46.0 && y < TITLE_H {
        return HitRegion::Maximise;
    }
    if x >= w - 138.0 && x < w - 92.0 && y < TITLE_H {
        return HitRegion::Minimise;
    }
    if y < TITLE_H {
        return HitRegion::TitleBar;
    }

    HitRegion::Content
}

impl OnyxApp {
    /// Build the vello Scene for the current frame.
    fn render_scene(
        scene: &mut Scene,
        width: f64,
        height: f64,
        title: &SimpleText,
        dock: &widgets::dock::CommandDock,
        hover_button: Option<titlebar::HoveredButton>,
    ) {
        scene.reset();

        // --- Title bar chrome ---
        let mut paint_cx = widgets::PaintCtx { scene };
        titlebar::paint(&mut paint_cx, width, hover_button);
        let scene = paint_cx.scene;

        // "Onyx Void" text centered horizontally, below the title bar.
        let center_x = width / 2.0;
        title.draw(scene, center_x - 80.0, height / 2.0 + 48.0);

        // Draw the command dock.
        let mut paint_cx = widgets::PaintCtx { scene };
        dock.paint(&mut paint_cx, 0.0, 0.0);
    }
}

impl ApplicationHandler for OnyxApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("Onyx Void")
            .with_decorations(false)
            .with_transparent(true)
            .with_inner_size(LogicalSize::new(1280.0_f64, 800.0_f64));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("failed to create window"),
        );

        // Enable OS drop-shadow on Windows for borderless windows.
        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::WindowExtWindows;
            window.set_undecorated_shadow(true);
        }

        // --- wgpu bootstrap ---
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("failed to create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("failed to find a suitable GPU adapter");

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
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // --- intermediate Rgba8Unorm target for vello compute shaders ---
        let (target_texture, target_view) =
            create_target_texture(&device, config.width, config.height);
        let blitter = wgpu::util::TextureBlitter::new(&device, surface_format);

        // --- vello renderer ---
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

        // Initialise dock dimensions from the first window size.
        self.dock.window_width = size.width.max(1) as f64;
        self.dock.window_height = size.height.max(1) as f64;
        self.window_size = (size.width.max(1) as f32, size.height.max(1) as f32);

        self.state = Some(RenderState {
            window,
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
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                let w = new_size.width.max(1);
                let h = new_size.height.max(1);
                state.config.width = w;
                state.config.height = h;
                state.surface.configure(&state.device, &state.config);
                let (tex, view) = create_target_texture(&state.device, w, h);
                state.target_texture = tex;
                state.target_view = view;
                self.dock.window_width = w as f64;
                self.dock.window_height = h as f64;
                let scale = state.window.scale_factor() as f32;
                self.window_size = (w as f32 / scale, h as f32 / scale);
                state.window.request_redraw();
            }

            WindowEvent::CursorMoved { position, .. } => {
                let scale = state.window.scale_factor() as f32;
                let x = position.x as f32 / scale;
                let y = position.y as f32 / scale;
                self.cursor_pos = (x, y);

                // Update hover state for title-bar buttons.
                let new_hover = titlebar::hovered_button(x, y, self.window_size.0);
                if new_hover != self.hover_button {
                    self.hover_button = new_hover;
                    state.window.request_redraw();
                }

                // Set cursor icon based on hit-test region.
                let icon = match hit_test_region(x, y, self.window_size.0, self.window_size.1) {
                    HitRegion::ResizeN => CursorIcon::NResize,
                    HitRegion::ResizeS => CursorIcon::SResize,
                    HitRegion::ResizeE => CursorIcon::EResize,
                    HitRegion::ResizeW => CursorIcon::WResize,
                    HitRegion::ResizeNE => CursorIcon::NeResize,
                    HitRegion::ResizeNW => CursorIcon::NwResize,
                    HitRegion::ResizeSE => CursorIcon::SeResize,
                    HitRegion::ResizeSW => CursorIcon::SwResize,
                    _ => CursorIcon::Default,
                };
                state.window.set_cursor(icon);
            }

            WindowEvent::KeyboardInput { .. } => {
                state.window.request_redraw();
            }

            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                // Double-click detection for title bar maximize/restore.
                let now = std::time::Instant::now();
                let is_double_click = self
                    .last_click_time
                    .map(|t| now.duration_since(t).as_millis() < 400)
                    .unwrap_or(false);
                let region = hit_test_region(
                    self.cursor_pos.0,
                    self.cursor_pos.1,
                    self.window_size.0,
                    self.window_size.1,
                );
                if is_double_click && matches!(region, HitRegion::TitleBar) {
                    state.window.set_maximized(!state.window.is_maximized());
                    self.last_click_time = None;
                    return;
                }
                self.last_click_time = Some(now);

                let (cx, cy) = self.cursor_pos;
                match hit_test_region(cx, cy, self.window_size.0, self.window_size.1) {
                    HitRegion::TitleBar => {
                        let _ = state.window.drag_window();
                    }
                    HitRegion::ResizeN => {
                        let _ = state.window.drag_resize_window(ResizeDirection::North);
                    }
                    HitRegion::ResizeS => {
                        let _ = state.window.drag_resize_window(ResizeDirection::South);
                    }
                    HitRegion::ResizeE => {
                        let _ = state.window.drag_resize_window(ResizeDirection::East);
                    }
                    HitRegion::ResizeW => {
                        let _ = state.window.drag_resize_window(ResizeDirection::West);
                    }
                    HitRegion::ResizeNE => {
                        let _ = state.window.drag_resize_window(ResizeDirection::NorthEast);
                    }
                    HitRegion::ResizeNW => {
                        let _ = state.window.drag_resize_window(ResizeDirection::NorthWest);
                    }
                    HitRegion::ResizeSE => {
                        let _ = state.window.drag_resize_window(ResizeDirection::SouthEast);
                    }
                    HitRegion::ResizeSW => {
                        let _ = state.window.drag_resize_window(ResizeDirection::SouthWest);
                    }
                    HitRegion::Close => {
                        event_loop.exit();
                    }
                    HitRegion::Minimise => {
                        state.window.set_minimized(true);
                    }
                    HitRegion::Maximise => {
                        state.window.set_maximized(!state.window.is_maximized());
                    }
                    HitRegion::Content => {
                        state.window.request_redraw();
                    }
                }
            }

            WindowEvent::MouseInput { .. } => {
                state.window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                let width = state.config.width;
                let height = state.config.height;

                // Build text layout if needed (first frame or after changes).
                if self.title_text.layout.is_none() {
                    self.title_text
                        .build(&mut self.font_cx, &mut self.layout_cx);
                }

                Self::render_scene(
                    &mut self.scene,
                    width as f64,
                    height as f64,
                    &self.title_text,
                    &self.dock,
                    self.hover_button,
                );

                let surface_texture = match state.surface.get_current_texture() {
                    Ok(t) => t,
                    Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                        state.surface.configure(&state.device, &state.config);
                        return;
                    }
                    Err(e) => {
                        tracing::error!("surface error: {e}");
                        return;
                    }
                };

                // Render to intermediate Rgba8Unorm texture.
                state
                    .renderer
                    .render_to_texture(
                        &state.device,
                        &state.queue,
                        &self.scene,
                        &state.target_view,
                        &vello::RenderParams {
                            base_color: ONYX_BLACK,
                            width,
                            height,
                            antialiasing_method: AaConfig::Msaa16,
                        },
                    )
                    .expect("vello render failed");

                // Blit from Rgba8Unorm target → surface (which may be Bgra8Unorm).
                let surface_view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    state
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("blit"),
                        });
                state.blitter.copy(
                    &state.device,
                    &mut encoder,
                    &state.target_view,
                    &surface_view,
                );
                state.queue.submit(Some(encoder.finish()));

                surface_texture.present();

                // Only keep requesting redraws when actively animating.
                if self.is_animating {
                    state.window.request_redraw();
                }
            }

            _ => {}
        }
    }
}

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
