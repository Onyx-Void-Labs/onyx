// ─── Onyx Void — Application Entry Point ──────────────────────────
// Boots tracing → SurrealDB → Makepad event loop.
// ────────────────────────────────────────────────────────────────────

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod app;

fn main() {
    // ── Tracing ──
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "onyx=debug,makepad=info".into()),
        )
        .init();

    tracing::info!("🌌 Onyx Void — ignition sequence started");

    // ── Launch Makepad ──
    app::app_main();
}
