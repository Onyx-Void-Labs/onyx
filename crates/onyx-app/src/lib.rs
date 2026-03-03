// ─── Onyx Void — Library Entry Point ───────────────────────────────
// Required for Android builds: the Makepad `app_main!()` macro
// generates JNI entry points when compiled as a cdylib (.so).
//
// On desktop, `main.rs` calls `app::app_main()` directly.
// On Android, the JVM loads the .so and invokes the JNI symbols
// that `app_main!()` auto-generates in `app.rs`.
// ────────────────────────────────────────────────────────────────────

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod app;
mod net_bridge;
mod remote_cursor;

/// Global profile name parsed from `--profile <NAME>` CLI arg.
/// On Android this stays uninitialized and defaults to the
/// standard identity path (no multi-profile needed on mobile).
pub static PROFILE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
