// --- Onyx Void — Application Entry Point (Vello + Winit) ---

use std::sync::Arc;

use vello::peniko::Color;
use vello::util::{RenderContext, RenderSurface};
use vello::{AaConfig, Renderer, RendererOptions, Scene};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

struct OnyxApp {
    render_cx: RenderContext,
    renderer: Option<Renderer>,
    surface: Option<RenderSurface<'static>>,
    window: Option<Arc<Window>>,
    scene: Scene,
}

impl Default for OnyxApp {
    fn default() -> Self {
        Self {
            render_cx: RenderContext::new(),
            renderer: None,
            surface: None,
            window: None,
            scene: Scene::new(),
        }
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
            WindowEvent::RedrawRequested => {
                self.draw();
            }
            _ => {}
        }
    }
}

impl OnyxApp {
    fn draw(&mut self) {
        let surface = match &mut self.surface {
            Some(s) => s,
            None => return,
        };
        let renderer = match &mut self.renderer {
            Some(r) => r,
            None => return,
        };
        let device = &self.render_cx.devices[surface.dev_id];

        self.scene.reset();

        let width = surface.config.width;
        let height = surface.config.height;

        let render_params = vello::RenderParams {
            base_color: Color::from_rgba8(24, 24, 28, 255),
            width,
            height,
            antialiasing_method: AaConfig::Area,
        };

        // Render scene to intermediate texture
        renderer
            .render_to_texture(
                &device.device,
                &device.queue,
                &self.scene,
                &surface.target_view,
                &render_params,
            )
            .expect("render to texture");

        // Blit the intermediate texture to the surface
        let surface_texture = surface
            .surface
            .get_current_texture()
            .expect("get surface texture");

        let target_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            device
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

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "onyx=debug,wgpu=warn".into()),
        )
        .init();

    tracing::info!("Onyx Void — Genesis ignition (Vello stack)");

    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = OnyxApp::default();
    event_loop.run_app(&mut app).expect("event loop error");
}
