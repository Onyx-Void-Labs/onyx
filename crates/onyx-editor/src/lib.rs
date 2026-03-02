// ─── Onyx Editor ───────────────────────────────────────────────────
// High-performance text buffer and cursor management.
//
// Architecture:
//   • Buffer   — Rope-backed text storage (O(log n) edits)
//   • Cursor   — Multi-cursor state (position, selection, etc.)
//
// This crate is renderer-agnostic. The Makepad widget in onyx-app
// consumes `EditorBuffer` and drives Cosmic-Text layout from it.
// ────────────────────────────────────────────────────────────────────

pub mod buffer;
pub mod cursor;

pub use buffer::EditorBuffer;
pub use cursor::Cursor;
