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

/// The classification of a VoidNode within the spatial universe.
///
/// - **Asteroid**: Unsorted / inbox — orbits the camera until triaged.
/// - **Planet**: A topic-level knowledge node (large, has gravity).
/// - **Satellite**: A task — orbits its parent Planet.
/// - **DysonSphere**: Encrypted content — passkey-protected, rendered
///   as a procedural golden shell that unfolds on authentication.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash,
    Archive, Serialize, Deserialize,
    serde::Serialize, serde::Deserialize,
)]
pub enum NodeType {
    /// Unsorted capture — lives in the Asteroid Belt (inbox).
    Asteroid,
    /// A topic node — attracts related Asteroids via semantic gravity.
    Planet,
    /// A task — orbits its parent Planet with deadline-driven urgency.
    Satellite,
    /// Encrypted node — encased in a procedural shell, requires Passkey.
    DysonSphere,
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
    Debug, Clone, PartialEq,
    Archive, Serialize, Deserialize,
    serde::Serialize, serde::Deserialize,
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
}

impl Default for SpatialState {
    fn default() -> Self {
        Self {
            pos: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            mass: 1.0,
            heat: 1.0,
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
    Debug, Clone, PartialEq,
    Archive, Serialize, Deserialize,
    serde::Serialize, serde::Deserialize,
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

    /// Create a new Planet (topic node).
    pub fn planet(id: OnyxId) -> Self {
        Self::new(id, NodeType::Planet)
    }

    /// Create a new Satellite (task node).
    pub fn satellite(id: OnyxId) -> Self {
        Self::new(id, NodeType::Satellite)
    }

    /// Create a new DysonSphere (encrypted node).
    pub fn dyson_sphere(id: OnyxId) -> Self {
        Self::new(id, NodeType::DysonSphere)
    }

    /// Update mass from content size (e.g. character count).
    /// Clamps to a minimum of 1.0 to avoid zero-gravity nodes.
    pub fn update_mass_from_content(&mut self, char_count: usize) {
        self.spatial.mass = (char_count as f32).max(1.0);
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
