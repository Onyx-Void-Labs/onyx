// ─── VoidNode: The Core Data Atom ──────────────────────────────────
// Every piece of data in the Singularity Engine is a VoidNode.
// Text, flashcards, tasks, passwords — all represented as spatial
// bodies governed by physics, backed by Loro CRDTs.
//
// This module defines the foundational data structures:
//   • NodeType     — classification (Asteroid, Planet, Satellite, DysonSphere)
//   • SpatialState — position, velocity, mass, heat in 3D space
//   • VoidNode     — the atom itself, combining identity + type + physics
// ────────────────────────────────────────────────────────────────────

use rkyv::{Archive, Deserialize, Serialize};
use serde;

use crate::id::OnyxId;

// ── Node Classification ─────────────────────────────────────────

/// Stellar classification — emergent from content mass via the Ignition Protocol.
///
/// - **Asteroid**: mass < 2.0 — unsorted / inbox.
/// - **RockyPlanet**: mass >= 2.0 && < 10.0 — structured data, small docs.
/// - **GasGiant**: mass >= 10.0 && < 50.0 — topic-level aggregator.
/// - **Sun**: mass >= 50.0 — ignited system anchor, dominant gravity.
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
pub enum NodeType {
    /// Unsorted capture — lives in the Asteroid Belt (inbox). mass < 2.0.
    Asteroid,
    /// Structured data node — tasks, events, small docs. mass >= 2.0 && < 10.0.
    RockyPlanet,
    /// Topic-level aggregator — high mass, strong gravity. mass >= 10.0 && < 50.0.
    GasGiant,
    /// System anchor — ignited! Massive gravitational pull. mass >= 50.0.
    Sun,
}

impl Default for NodeType {
    fn default() -> Self {
        NodeType::Asteroid
    }
}

// ── Spatial State ───────────────────────────────────────────────

/// The physics state of a VoidNode in 3D space.
///
/// All rendering, clustering (constellations), and interaction
/// (gravity, accretion, Oort Cloud drift) derive from this state.
#[derive(
    Debug, Clone, PartialEq, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize,
)]
pub struct SpatialState {
    /// Position in 3D space [x, y, z].
    pub pos: [f32; 3],

    /// Velocity vector [vx, vy, vz] — used by the physics integrator.
    pub velocity: [f32; 3],

    /// Mass — derived from content size (char count, attachment bytes).
    /// Determines gravitational pull on nearby Asteroids.
    pub mass: f32,

    /// Heat — recency of edits.  Hot nodes glow brighter and stay
    /// near the center; cold nodes drift to the Oort Cloud.
    pub heat: f32,

    /// Kinematic lock — when true, physics forces are skipped and
    /// velocity is zeroed (the user is dragging this node).
    #[serde(default)]
    pub is_dragged: bool,

    /// Hover state — when true, the user's cursor is over this node.
    /// Physics applies extra damping ("hover arrest") to make it
    /// easier to click moving targets.
    #[serde(default)]
    pub hovered: bool,
}

impl Default for SpatialState {
    fn default() -> Self {
        Self {
            pos: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            mass: 1.0,
            heat: 1.0,
            is_dragged: false,
            hovered: false,
        }
    }
}

// ── VoidNode ────────────────────────────────────────────────────

/// The fundamental data atom of the Singularity Engine.
///
/// Every piece of user data — notes, tasks, flashcards, passwords —
/// is a VoidNode.  Its content is backed by a Loro CRDT document
/// (referenced by `id`), while its spatial existence is governed
/// by Newtonian-ish physics via `spatial`.
#[derive(
    Debug, Clone, PartialEq, Archive, Serialize, Deserialize, serde::Serialize, serde::Deserialize,
)]
pub struct VoidNode {
    /// Unique identity — same ID used as the Loro document key.
    pub id: OnyxId,

    /// What kind of spatial body this node represents.
    pub node_type: NodeType,

    /// Physics state: position, velocity, mass, heat.
    pub spatial: SpatialState,

    /// Quantum Mirrors — cross-links to other VoidNodes.
    /// A node appearing in multiple constellations gets a Wormhole
    /// orbital for each mirror link.
    pub mirrors: Vec<OnyxId>,
}

impl VoidNode {
    /// Create a new VoidNode with the given ID and type.
    /// Starts at the origin with default spatial state and no mirrors.
    pub fn new(id: OnyxId, node_type: NodeType) -> Self {
        Self {
            id,
            node_type,
            spatial: SpatialState::default(),
            mirrors: Vec::new(),
        }
    }

    /// Create a new Asteroid (unsorted inbox item).
    pub fn asteroid(id: OnyxId) -> Self {
        Self::new(id, NodeType::Asteroid)
    }

    /// Create a new RockyPlanet (structured data node).
    pub fn rocky_planet(id: OnyxId) -> Self {
        Self::new(id, NodeType::RockyPlanet)
    }

    /// Create a new GasGiant (topic aggregator).
    pub fn gas_giant(id: OnyxId) -> Self {
        Self::new(id, NodeType::GasGiant)
    }

    /// Create a new Sun (system anchor — ignited).
    pub fn sun(id: OnyxId) -> Self {
        Self::new(id, NodeType::Sun)
    }

    /// Ignition Protocol: Calculate mass and emergent node type from
    /// content length and incoming link count.
    /// Taxonomy is emergent — no manual type selection.
    pub fn calculate_mass_and_type(&mut self, content_length: usize, incoming_links: usize) {
        let mass = 1.0 + (content_length as f32) * 0.001 + (incoming_links as f32) * 5.0;
        self.spatial.mass = mass;
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

    /// Apply a heat pulse (edit recency bump).
    /// Heat decays over time via the physics integrator; this sets
    /// it to maximum (1.0) on any edit.
    pub fn heat_pulse(&mut self) {
        self.spatial.heat = 1.0;
    }

    /// Add a mirror (cross-link) to another VoidNode.
    pub fn add_mirror(&mut self, target: OnyxId) {
        if !self.mirrors.contains(&target) {
            self.mirrors.push(target);
        }
    }

    /// Remove a mirror link.
    pub fn remove_mirror(&mut self, target: &OnyxId) {
        self.mirrors.retain(|m| m != target);
    }
}
