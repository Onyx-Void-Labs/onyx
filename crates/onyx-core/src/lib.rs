#![deny(unsafe_code)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![warn(clippy::panic)]
// ─── Onyx Core ─────────────────────────────────────────────────────
// Bedrock data model for Onyx Void.
// LoroTree-backed Void / Note types land here during Onyx Genesis.
// ────────────────────────────────────────────────────────────────────

pub mod blob;
pub mod blocks;
pub mod crypto;
pub mod diffing;
pub mod document;
pub use document::OnyxWorkspace; // re-export core workspace type
pub mod fsrs;
pub mod question_bank;
pub mod graph;
pub use graph::BacklinkIndex;
pub mod grid_layout;
pub mod history;
pub mod import;
pub mod layout_state;
pub mod manager;
pub use manager::WorkspaceManager;
pub mod math;
pub mod media;
pub mod model;
pub mod neural;
pub mod persistence;
pub use persistence::{load_workspace, save_workspace, start_autosave, atomic_write, save_workspace_to_tmp, load_workspace_with_recovery, save_workspace_to_dir, load_workspace_from_dir};
pub mod query;
pub mod scheduler;
pub mod search;
pub mod settings;
pub mod templates;
