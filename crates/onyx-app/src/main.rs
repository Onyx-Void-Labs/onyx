// ─── Onyx Void — Application Entry Point ──────────────────────────
// Boots tracing → SurrealDB → Makepad event loop.
// ────────────────────────────────────────────────────────────────────

#[cfg(not(target_os = "android"))]
use mimalloc::MiMalloc;
#[cfg(not(target_os = "android"))]
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod app;
mod aero_hud;
mod media_engine;
mod net_bridge;
mod remote_cursor;

/// Global profile name parsed from `--profile <NAME>` CLI arg.
/// Each profile gets its own identity key so multiple instances
/// can run on the same machine with different Iroh NodeIDs.
pub static PROFILE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();

fn main() {
    // ── Parse --profile flag ──
    let args: Vec<String> = std::env::args().collect();
    let profile = args
        .windows(2)
        .find(|w| w[0] == "--profile")
        .map(|w| w[1].clone());

    if let Some(ref p) = profile {
        eprintln!("[onyx] using profile: {p}");
    }
    PROFILE.set(profile).ok();

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
