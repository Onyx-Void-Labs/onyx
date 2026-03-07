// ─── Onyx Void — Library Root ──────────────────────────────────────
// Module declarations and re-exports only. All logic lives in
// app.rs (orchestrator), window.rs (chrome), widgets/, and ui/.
// ────────────────────────────────────────────────────────────────────
#![allow(dead_code, unused_imports)]

pub mod app;
pub mod ui;
pub mod widgets;
pub mod window;

pub use app::OnyxApp;
