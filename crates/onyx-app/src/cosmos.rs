// ─── Cosmos: The Spatial Universe State ─────────────────────────────
// Owns all VoidNodes and drives the physics simulation.
//
// The Cosmos is the single source of truth for all spatial state.
// The App holds one Cosmos instance and ticks it every frame.
//
// Responsibilities:
//   • CRUD operations on VoidNodes
//   • Running the physics integrator
//   • Spatial queries (hit-testing, nearest node, etc.)
//   • Generating unique spawn positions for new nodes
// ────────────────────────────────────────────────────────────────────

use onyx_core::id::OnyxId;
use onyx_core::void_node::{NodeType, VoidNode};

use crate::physics;

/// The spatial universe — holds all VoidNodes and drives physics.
pub struct Cosmos {
    /// All nodes in the universe.
    pub nodes: Vec<VoidNode>,
    /// Accumulated time (seconds) — used for spawn position generation.
    time: f32,
    /// Index of the currently selected node (if any).
    pub selected: Option<usize>,
    /// Index of the node being dragged (if any).
    pub dragged: Option<usize>,
}

impl Default for Cosmos {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            time: 0.0,
            selected: None,
            dragged: None,
        }
    }
}

impl Cosmos {
    /// Create a new empty cosmos.
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a new VoidNode.  All nodes start as Asteroids —
    /// taxonomy is emergent via `calculate_mass_and_type()`.
    /// Returns the index of the new node.
    pub fn spawn_node(&mut self) -> usize {
        let id = OnyxId::new();
        let mut node = VoidNode::new(id, NodeType::Asteroid);

        // Generate a position on a golden-angle spiral so new nodes
        // don't stack on top of each other.
        let i = self.nodes.len() as f32;
        let golden_angle = 2.399_963; // radians ≈ 137.508°
        let angle = i * golden_angle + self.time * 0.1;
        let radius = 80.0 + i.sqrt() * 60.0;
        node.spatial.pos[0] = angle.cos() * radius;
        node.spatial.pos[1] = angle.sin() * radius;

        // New nodes are hot (just created).
        node.spatial.heat = 1.0;

        // Base mass for a fresh node (will evolve via Ignition Protocol).
        node.spatial.mass = 1.0;

        self.nodes.push(node);
        self.nodes.len() - 1
    }

    /// Run one physics tick.
    pub fn tick(&mut self, dt: f32, view_center_x: f32, view_center_y: f32, zoom: f32) {
        self.time += dt;

        // Sync the is_dragged flag on the actual node
        // so the physics engine respects kinematic lock.
        for (i, node) in self.nodes.iter_mut().enumerate() {
            node.spatial.is_dragged = self.dragged == Some(i);
        }

        physics::tick(&mut self.nodes, dt, view_center_x, view_center_y, zoom);
    }

    /// Move the dragged node to the given world-space coordinates.
    pub fn drag_to(&mut self, wx: f32, wy: f32) {
        if let Some(idx) = self.dragged {
            if let Some(node) = self.nodes.get_mut(idx) {
                node.spatial.pos[0] = wx;
                node.spatial.pos[1] = wy;
                node.spatial.velocity = [0.0, 0.0, 0.0];
                // Heat pulse on drag — the user is interacting.
                node.heat_pulse();
            }
        }
    }

    /// Get the content label for a node (for rendering text previews).
    pub fn node_label(&self, idx: usize) -> &'static str {
        match self.nodes.get(idx).map(|n| n.node_type) {
            Some(NodeType::Asteroid) => "●",
            Some(NodeType::RockyPlanet) => "●",
            Some(NodeType::GasGiant) => "◎",
            Some(NodeType::Sun) => "☀",
            Some(NodeType::BlackHole) => "⊗",
            Some(NodeType::WhiteHole) => "⊙",
            None => "?",
        }
    }

    /// Number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Set hover state on a specific node (for hover arrest).
    pub fn set_hovered(&mut self, idx: usize, hovered: bool) {
        if let Some(node) = self.nodes.get_mut(idx) {
            node.spatial.hovered = hovered;
        }
    }

    /// Clear all hover states (e.g. when mouse leaves the cosmos view).
    pub fn clear_all_hovers(&mut self) {
        for node in &mut self.nodes {
            node.spatial.hovered = false;
        }
    }

    /// Release a dragged node with throw velocity (inertia).
    /// Scales per-frame drag delta into physics velocity.
    pub fn release_throw(&mut self, vx: f32, vy: f32) {
        const THROW_MULTIPLIER: f32 = 300.0;
        if let Some(idx) = self.dragged {
            if let Some(node) = self.nodes.get_mut(idx) {
                node.spatial.velocity[0] = vx * THROW_MULTIPLIER;
                node.spatial.velocity[1] = vy * THROW_MULTIPLIER;
            }
        }
        self.dragged = None;
    }

    /// Delete a node by index (drag into Black Hole).
    #[allow(dead_code)]
    pub fn remove_node(&mut self, idx: usize) {
        if idx < self.nodes.len() {
            self.nodes.remove(idx);
            // Fix up selected / dragged indices.
            self.selected = self.selected.and_then(|s| {
                if s == idx {
                    None
                } else if s > idx {
                    Some(s - 1)
                } else {
                    Some(s)
                }
            });
            self.dragged = self.dragged.and_then(|d| {
                if d == idx {
                    None
                } else if d > idx {
                    Some(d - 1)
                } else {
                    Some(d)
                }
            });
        }
    }

    /// Purge all tombstoned nodes from the cosmos.
    #[allow(dead_code)]
    pub fn purge_tombstones(&mut self) {
        self.nodes.retain(|n| !n.tombstone);
        self.selected = None;
        self.dragged = None;
    }

    /// Spawn a BlackHole singularity at the given position.
    pub fn spawn_black_hole(&mut self, x: f32, y: f32) -> usize {
        let id = OnyxId::new();
        let mut node = VoidNode::new(id, NodeType::BlackHole);
        node.spatial.pos[0] = x;
        node.spatial.pos[1] = y;
        node.spatial.mass = 6.25; // visual radius = sqrt(6.25) * 10 = 25
        node.spatial.heat = 0.8;
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    /// Spawn a WhiteHole singularity at the given position.
    pub fn spawn_white_hole(&mut self, x: f32, y: f32) -> usize {
        let id = OnyxId::new();
        let mut node = VoidNode::new(id, NodeType::WhiteHole);
        node.spatial.pos[0] = x;
        node.spatial.pos[1] = y;
        node.spatial.mass = 6.25;
        node.spatial.heat = 0.8;
        self.nodes.push(node);
        self.nodes.len() - 1
    }
}
