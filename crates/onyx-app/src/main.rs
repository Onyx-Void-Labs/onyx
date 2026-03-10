// --- Onyx Void — Application Entry Point (Vello + Winit) ---

mod app;
mod cursor;
mod editor_renderer;
mod ribbon;
#[allow(dead_code)]
mod renderer;
#[allow(dead_code)]
mod widgets;

use anyhow::Context;
use winit::event_loop::EventLoop;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "onyx=debug,wgpu=warn".into()),
        )
        .init();

    tracing::info!("Onyx Void — Genesis ignition (Vello stack)");

    let event_loop = EventLoop::new().context("failed to create event loop")?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    let mut app = app::OnyxApp::new()?;
    event_loop.run_app(&mut app).context("event loop error")?;
    Ok(())
}
