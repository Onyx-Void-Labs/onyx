// ─── Onyx Store ────────────────────────────────────────────────────
// Local-first persistence layer.
//
//   SurrealDB (embedded, kv-mem)  ← durable document storage
//   Loro (CRDTs)                  ← conflict-free collaborative state
//   ZSTD                          ← compression for CRDT deltas
//   Neural Index (candle)         ← semantic search via MiniLM embeddings
//   Vault (XChaCha20-Poly1305)    ← encryption at rest
//
// All storage is local. No cloud. No telemetry.
// ────────────────────────────────────────────────────────────────────

pub mod crdt;

#[cfg(feature = "surrealdb-backend")]
pub mod db;

#[cfg(feature = "neural")]
pub mod neural_index;

#[cfg(feature = "vault")]
pub mod vault;

pub use crdt::CrdtDoc;

#[cfg(feature = "surrealdb-backend")]
pub use db::Store;

#[cfg(feature = "neural")]
pub use neural_index::NeuralIndex;

#[cfg(feature = "vault")]
pub use vault::Vault;
