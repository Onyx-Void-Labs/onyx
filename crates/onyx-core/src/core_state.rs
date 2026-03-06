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

// ── Utility Anchor Classification ───────────────────────────────

/// Anchor types for utility nodes — fixed-position system nodes
/// that serve as gravitational anchors in the spatial universe.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Archive,
    Serialize,
    Deserialize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum UtilityAnchor {
    Settings,
    Calendar,
    Passwords,
    Email,
}

// ── Node Classification (Phase 4 Stellar Dynamics) ─────────────

/// Stellar classification of a VoidNode in the Phase 4 universe.
///
/// - **Atom**: Smallest data particle — a single capture or note.
/// - **RockyMoon**: Structured data node — tasks, events, small docs.
/// - **GasGiant**: Topic-level aggregator — high mass, strong gravity.
/// - **Sun**: System anchor — infinite effective mass, kinematic (fixed).
/// - **Asteroid**: Unsorted inbox items — low mass, drift-eligible.
/// - **Utility**: Fixed system anchors (Settings, Calendar, etc.) — kinematic.
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
    Atom,
    RockyMoon,
    GasGiant,
    Sun,
    Asteroid,
    Utility(UtilityAnchor),
}

impl Default for NodeType {
    fn default() -> Self {
        NodeType::Atom
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
        }
    }

    /// Create an Atom (smallest data particle).
    pub fn atom(id: String) -> Self {
        Self::new(id, NodeType::Atom)
    }

    /// Create a RockyMoon (structured data node).
    pub fn rocky_moon(id: String) -> Self {
        Self::new(id, NodeType::RockyMoon)
    }

    /// Create a GasGiant (topic-level aggregator, high mass).
    pub fn gas_giant(id: String) -> Self {
        Self {
            mass: 10.0,
            ..Self::new(id, NodeType::GasGiant)
        }
    }

    /// Create a Sun (kinematic system anchor).
    pub fn sun(id: String) -> Self {
        Self::new(id, NodeType::Sun)
    }

    /// Create an Asteroid (unsorted inbox item).
    pub fn asteroid(id: String) -> Self {
        Self::new(id, NodeType::Asteroid)
    }

    /// Create a Utility anchor (kinematic, fixed position).
    pub fn utility(id: String, anchor: UtilityAnchor) -> Self {
        Self::new(id, NodeType::Utility(anchor))
    }
}
