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
use widgets::titlebar::TitleBar;

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
    titlebar: TitleBar,
    /// When true, `RedrawRequested` will continuously schedule redraws.
    is_animating: bool,
    /// Last known cursor position (logical pixels).
    cursor_pos: (f32, f32),
    /// Stored window size in PHYSICAL pixels (from Resized).
    window_size: (f32, f32),
    /// Display scale factor (physical / logical).
    scale_factor: f64,
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
            titlebar: TitleBar::new(),
            is_animating: false,
            cursor_pos: (0.0, 0.0),
            window_size: (1280.0, 800.0),
            scale_factor: 1.0,
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
    /// Logical window size (physical / scale_factor).
    fn logical_window_size(&self) -> (f32, f32) {
        let s = self.scale_factor as f32;
        (self.window_size.0 / s, self.window_size.1 / s)
    }

    /// Build the vello Scene for the current frame.
    fn render_scene(
        scene: &mut Scene,
        width: f32,
        height: f32,
        title: &SimpleText,
        dock: &widgets::dock::CommandDock,
        titlebar: &TitleBar,
    ) {
        scene.reset();

        // --- Title bar chrome (logical coords) ---
        titlebar.paint(scene, width, height);

        // "Onyx Void" text centered horizontally, below the title bar.
        let center_x = width as f64 / 2.0;
        title.draw(scene, center_x - 80.0, height as f64 / 2.0 + 48.0);

        // Draw the command dock.
        dock.paint(scene, width, height);
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

        // Store scale factor and physical window size.
        self.scale_factor = window.scale_factor();
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
                // Store PHYSICAL size; derive logical via scale_factor.
                self.scale_factor = state.window.scale_factor();
                self.window_size = (w as f32, h as f32);
                state.window.request_redraw();
            }

            WindowEvent::CursorMoved { position, .. } => {
                // Convert physical → logical.
                let s = self.scale_factor as f32;
                self.cursor_pos = ((position.x as f32) / s, (position.y as f32) / s);

                // Logical window dimensions.
                let lw = self.window_size.0 / s;
                let lh = self.window_size.1 / s;

                // Set cursor icon and hover state based on hit-test region.
                let region = hit_test_region(self.cursor_pos.0, self.cursor_pos.1, lw, lh);
                let new_hover = match region {
                    HitRegion::Close | HitRegion::Maximise | HitRegion::Minimise => Some(region),
                    _ => None,
                };
                if new_hover != self.titlebar.hover {
                    self.titlebar.hover = new_hover;
                    state.window.request_redraw();
                }
                let icon = match region {
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
                state.window.request_redraw();
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = scale_factor;
                state.window.request_redraw();
            }

            WindowEvent::KeyboardInput { .. } => {
                state.window.request_redraw();
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let s = self.scale_factor as f32;
                let lw = self.window_size.0 / s;
                let lh = self.window_size.1 / s;
                let region = hit_test_region(self.cursor_pos.0, self.cursor_pos.1, lw, lh);
                let now = std::time::Instant::now();

                match region {
                    HitRegion::Close => {
                        event_loop.exit();
                    }
                    HitRegion::Minimise => {
                        state.window.set_minimized(true);
                    }
                    HitRegion::Maximise => {
                        state.window.set_maximized(!state.window.is_maximized());
                    }
                    HitRegion::TitleBar => {
                        // Double-click: toggle maximise.
                        let is_dbl = self
                            .last_click_time
                            .map(|t| now.duration_since(t).as_millis() < 400)
                            .unwrap_or(false);
                        if is_dbl {
                            state.window.set_maximized(!state.window.is_maximized());
                            self.last_click_time = None;
                            // No drag on double-click.
                        } else {
                            // Single click: start drag.
                            self.last_click_time = Some(now);
                            state.window.drag_window().ok();
                        }
                    }
                    HitRegion::ResizeN => {
                        state.window.drag_resize_window(ResizeDirection::North).ok();
                    }
                    HitRegion::ResizeS => {
                        state.window.drag_resize_window(ResizeDirection::South).ok();
                    }
                    HitRegion::ResizeE => {
                        state.window.drag_resize_window(ResizeDirection::East).ok();
                    }
                    HitRegion::ResizeW => {
                        state.window.drag_resize_window(ResizeDirection::West).ok();
                    }
                    HitRegion::ResizeNE => {
                        state
                            .window
                            .drag_resize_window(ResizeDirection::NorthEast)
                            .ok();
                    }
                    HitRegion::ResizeNW => {
                        state
                            .window
                            .drag_resize_window(ResizeDirection::NorthWest)
                            .ok();
                    }
                    HitRegion::ResizeSE => {
                        state
                            .window
                            .drag_resize_window(ResizeDirection::SouthEast)
                            .ok();
                    }
                    HitRegion::ResizeSW => {
                        state
                            .window
                            .drag_resize_window(ResizeDirection::SouthWest)
                            .ok();
                    }
                    HitRegion::Content => {}
                }
                state.window.request_redraw();
            }

            WindowEvent::MouseInput { .. } => {
                state.window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                let s = self.scale_factor as f32;
                let lw = self.window_size.0 / s;
                let lh = self.window_size.1 / s;

                // Build text layout if needed (first frame or after changes).
                if self.title_text.layout.is_none() {
                    self.title_text
                        .build(&mut self.font_cx, &mut self.layout_cx);
                }

                Self::render_scene(
                    &mut self.scene,
                    lw,
                    lh,
                    &self.title_text,
                    &self.dock,
                    &self.titlebar,
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

                // Render to intermediate Rgba8Unorm texture (logical viewport).
                state
                    .renderer
                    .render_to_texture(
                        &state.device,
                        &state.queue,
                        &self.scene,
                        &state.target_view,
                        &vello::RenderParams {
                            base_color: ONYX_BLACK,
                            width: lw as u32,
                            height: lh as u32,
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
