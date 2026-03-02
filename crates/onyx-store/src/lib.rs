// ─── Onyx Store ────────────────────────────────────────────────────
// Local-first persistence layer.
//
//   SurrealDB (embedded, kv-mem)  ← durable document storage
//   Loro (CRDTs)                  ← conflict-free collaborative state
//   ZSTD                          ← compression for CRDT deltas
//
// All storage is local. No cloud. No telemetry.
// ────────────────────────────────────────────────────────────────────

pub mod crdt;

#[cfg(feature = "surrealdb-backend")]
pub mod db;

pub use crdt::CrdtDoc;

#[cfg(feature = "surrealdb-backend")]
pub use db::Store;
