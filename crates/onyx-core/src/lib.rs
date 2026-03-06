// ─── Onyx Core ─────────────────────────────────────────────────────
// Domain types, error handling, and shared primitives for Onyx Void.
// Every other crate depends on this. Zero IO. Pure data & logic.
// ────────────────────────────────────────────────────────────────────

pub mod core_state;
pub mod document;
pub mod error;
pub mod id;
pub mod identity;
pub mod persistence;
pub mod protocol;
pub mod stellar_physics;
pub mod void_node;

pub use error::OnyxError;
pub use id::OnyxId;
pub use identity::VoidIdentity;
pub use void_node::{NodeType, SpatialState, VoidNode};
