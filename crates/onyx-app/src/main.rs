// --- Onyx Void — Application Entry Point (Vello Stack) ---

#[cfg(not(target_os = "android"))]
use mimalloc::MiMalloc;
#[cfg(not(target_os = "android"))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use onyx_app::OnyxApp;
use winit::event_loop::EventLoop;

fn main() {
    // -- Tracing --
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "onyx=debug,wgpu=warn".into()),
        )
        .init();

    tracing::info!("Onyx Void — ignition sequence started (Vello stack)");

    let event_loop = EventLoop::new().expect("failed to create event loop");
    // PERF: Always poll — maximum responsiveness, zero input latency.
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = OnyxApp::default();
    event_loop.run_app(&mut app).expect("event loop error");
}
