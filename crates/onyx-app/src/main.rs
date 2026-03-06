// --- Onyx Void - Application Entry Point ---

#[cfg(not(target_os = "android"))]
use mimalloc::MiMalloc;
#[cfg(not(target_os = "android"))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod app;

fn main() {
    // -- Tracing --
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "onyx=debug,makepad=info".into()),
        )
        .init();

    tracing::info!("Onyx Void - ignition sequence started");

    // -- Launch Makepad --
    app::app_main();
}
