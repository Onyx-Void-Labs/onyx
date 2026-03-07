// --- Onyx Void — Application Entry Point (Vello + Winit) ---

mod app;
mod renderer;
mod widgets;

use winit::event_loop::EventLoop;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "onyx=debug,wgpu=warn".into()),
        )
        .init();

    tracing::info!("Onyx Void — Genesis ignition (Vello stack)");

    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    let mut app = app::OnyxApp::new();
    event_loop.run_app(&mut app).expect("event loop error");
}
