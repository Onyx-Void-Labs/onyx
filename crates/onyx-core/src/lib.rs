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
pub mod editing;
pub use document::OnyxWorkspace; // re-export core workspace type
pub mod fsrs;
pub use fsrs::{CardState, FlashcardData, Scheduler};
pub mod graph;
pub mod question_bank;
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
pub use persistence::{
    atomic_write, load_workspace, load_workspace_from_dir, load_workspace_with_recovery,
    save_workspace, save_workspace_to_dir, save_workspace_to_tmp, start_autosave,
};
pub mod query;
pub mod scheduler;
pub mod search;
pub mod settings;
pub mod templates;

// Canvas geometry and neuro navigation
pub mod canvas;

// Learning helpers such as the Feynman audio grader
pub mod learning;
