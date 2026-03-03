// ─── Onyx Net ──────────────────────────────────────────────────────
// Phase 2: The Telepathic Link
//
// This crate wires Iroh (P2P QUIC) + Iroh-Gossip (broadcast trees)
// to Loro (CRDTs) over ZSTD-compressed deltas. The result is
// sub-20ms sync between any two devices that share a secret room key.
//
// Architecture:
//   ShadowMesh    — gossip-based device discovery via SHA256(room_key)
//   SyncEngine    — bridges Loro deltas ↔ Iroh QUIC streams
//   OnyxNode      — top-level coordinator (Endpoint + Gossip + Router)
//
// Optimistic UI: local edits render immediately at 144Hz.
// Network sync is fire-and-forget — no blocking the render loop.
// ────────────────────────────────────────────────────────────────────

pub mod mesh;
pub mod node;
pub mod sync;

pub use mesh::{MeshEvent, ShadowMesh};
pub use node::OnyxNode;
pub use sync::SyncEngine;
