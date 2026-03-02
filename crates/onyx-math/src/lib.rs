// ─── Onyx Math ─────────────────────────────────────────────────────
// Typst-powered math rendering pipeline.
//
// Flow:  LaTeX/Typst source  →  Typst compile  →  Pixmap (RGBA)
//
// The Makepad widget in onyx-app will upload the resulting pixmap
// to a GPU texture for compositing at 144Hz.
// ────────────────────────────────────────────────────────────────────

pub mod render;

pub use render::MathRenderer;
