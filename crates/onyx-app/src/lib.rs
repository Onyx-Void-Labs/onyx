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
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use widgets::text::SimpleText;

/// Onyx Black — #09090b
const ONYX_BLACK: vello::peniko::Color = vello::peniko::Color::from_rgba8(0x09, 0x09, 0x0b, 0xff);

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
        }
    }
}

impl OnyxApp {
    /// Build the vello Scene for the current frame.
    fn render_scene(scene: &mut Scene, width: f64, height: f64, title: &SimpleText) {
        scene.reset();

        // Red circle in the center of the window.
        let center_x = width / 2.0;
        let center_y = height / 2.0;
        let radius = width.min(height) * 0.12;

        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            palette::css::RED,
            None,
            &Circle::new((center_x, center_y), radius),
        );

        // Draw "Onyx Void" text centered.
        title.draw(scene, center_x - 80.0, center_y + radius + 48.0);
    }
}

impl ApplicationHandler for OnyxApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title("Onyx Void")
            .with_inner_size(LogicalSize::new(1280, 800));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("failed to create window"),
        );

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

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("onyx-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            },
        ))
        .expect("failed to create wgpu device");

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| matches!(f, wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm))
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
                state.window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                let width = state.config.width;
                let height = state.config.height;

                // Build text layout if needed (first frame or after changes).
                if self.title_text.layout.is_none() {
                    self.title_text.build(&mut self.font_cx, &mut self.layout_cx);
                }

                Self::render_scene(&mut self.scene, width as f64, height as f64, &self.title_text);

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
                let mut encoder = state.device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor { label: Some("blit") },
                );
                state.blitter.copy(
                    &state.device,
                    &mut encoder,
                    &state.target_view,
                    &surface_view,
                );
                state.queue.submit(Some(encoder.finish()));

                surface_texture.present();

                // Request continuous redraws for smooth animation.
                state.window.request_redraw();
            }

            _ => {}
        }
    }
}

/// Create an intermediate Rgba8Unorm texture for vello's compute pipeline.
fn create_target_texture(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::Texture, wgpu::TextureView) {
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
