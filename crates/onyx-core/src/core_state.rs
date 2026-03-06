// ─── Core State: Phase 4 (Stellar Dynamics) & Phase 5 (Semantic Embeddings) ──
// Backend foundation types for the evolved Singularity Engine.
//
// All structs are rkyv-serializable (zero-copy) and serde-compatible
// for Loro CRDT container mapping.  IDs are String UUIDs for direct
// Loro key interop.
//
// No C-library dependencies.  Pure Rust.  Android-safe.
// ─────────────────────────────────────────────────────────────────────────────

use rkyv::{Archive, Deserialize, Serialize};
use serde;

// ── Node Classification (Ignition Protocol) ─────────────────────

/// Stellar classification of a VoidNode — emergent via the Ignition Protocol.
///
/// - **Asteroid**: mass < 2.0 — unsorted capture.
/// - **RockyPlanet**: mass >= 2.0 && < 10.0 — structured data.
/// - **GasGiant**: mass >= 10.0 && < 50.0 — topic-level aggregator.
/// - **Sun**: mass >= 50.0 — ignited system anchor.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum NodeType {
    Asteroid,
    RockyPlanet,
    GasGiant,
    Sun,
    BlackHole,
    WhiteHole,
}

impl Default for NodeType {
    fn default() -> Self {
        NodeType::Asteroid
    }
}

// ── VoidNode (Phase 4 / Phase 5) ───────────────────────────────

/// The fundamental spatial data atom for Stellar Dynamics.
///
/// Each VoidNode is a body in the physics simulation.  Its `id` is a
/// String UUID that maps 1:1 to a Loro CRDT container key.
///
/// `embedding` is prepared for Phase 5 (HuggingFace Candle) where
/// semantic vectors will drive gravity modulation — similar documents
/// cluster together via embedding-space distance.
#[derive(Debug, Clone, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize)]
pub struct VoidNode {
    /// UUID string — maps directly to a Loro CRDT container key.
    pub id: String,

    /// Position in 3D space [x, y, z].
    pub position: [f32; 3],

    /// Velocity vector [vx, vy, vz].
    pub velocity: [f32; 3],

    /// Mass — determines gravitational pull.  Derived from content size.
    pub mass: f32,

    /// Stellar classification — governs physics behavior and rendering.
    pub node_type: NodeType,

    /// Semantic embedding vector (Phase 5: HuggingFace Candle).
    /// When populated, embedding-space distance modulates gravity
    /// so semantically similar nodes cluster together.
    pub embedding: Option<Vec<f32>>,

    /// Expansion factor for the Phase 4 "Dive" animation.
    /// 1.0 = normal size.  Animated toward target during transitions.
    pub expansion_factor: f32,

    /// Tombstone flag — set when absorbed by a BlackHole.
    pub tombstone: bool,
}

impl VoidNode {
    /// Create a new VoidNode with the given UUID and type.
    /// Starts at the origin with default spatial state.
    pub fn new(id: String, node_type: NodeType) -> Self {
        Self {
            id,
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            mass: 1.0,
            node_type,
            embedding: None,
            expansion_factor: 1.0,
            tombstone: false,
        }
    }

    /// Create an Asteroid (unsorted inbox item).
    pub fn asteroid(id: String) -> Self {
        Self::new(id, NodeType::Asteroid)
    }

    /// Create a RockyPlanet (structured data node).
    pub fn rocky_planet(id: String) -> Self {
        Self::new(id, NodeType::RockyPlanet)
    }

    /// Create a GasGiant (topic-level aggregator, high mass).
    pub fn gas_giant(id: String) -> Self {
        Self {
            mass: 10.0,
            ..Self::new(id, NodeType::GasGiant)
        }
    }

    /// Create a Sun (ignited system anchor).
    pub fn sun(id: String) -> Self {
        Self::new(id, NodeType::Sun)
    }

    /// Ignition Protocol: Calculate mass and emergent node type from
    /// content length and incoming link count.
    /// Taxonomy is emergent — no manual type selection.
    pub fn calculate_mass_and_type(&mut self, content_length: usize, incoming_links: usize) {
        if matches!(self.node_type, NodeType::BlackHole | NodeType::WhiteHole) {
            return;
        }
        let mass = 1.0 + (content_length as f32) / 100.0 + (incoming_links as f32) * 5.0;
        self.mass = mass;
        self.node_type = if mass >= 50.0 {
            NodeType::Sun
        } else if mass >= 10.0 {
            NodeType::GasGiant
        } else if mass >= 2.0 {
            NodeType::RockyPlanet
        } else {
            NodeType::Asteroid
        };
    }
}
