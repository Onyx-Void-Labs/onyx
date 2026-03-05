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

    /// Spawn a new VoidNode of the given type at a semi-random position
    /// near the origin.  Returns the index of the new node.
    pub fn spawn_node(&mut self, node_type: NodeType) -> usize {
        let id = OnyxId::new();
        let mut node = VoidNode::new(id, node_type);

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

        // Planets start with more mass so they anchor constellations.
        match node_type {
            NodeType::Planet => node.spatial.mass = 50.0,
            NodeType::DysonSphere => node.spatial.mass = 40.0,
            NodeType::Satellite => node.spatial.mass = 5.0,
            NodeType::Asteroid => node.spatial.mass = 3.0,
        }

        self.nodes.push(node);
        self.nodes.len() - 1
    }

    /// Run one physics tick.
    pub fn tick(&mut self, dt: f32) {
        self.time += dt;

        // If a node is being dragged, pin its velocity to zero
        // so the physics engine doesn't fight the user.
        if let Some(idx) = self.dragged {
            if let Some(node) = self.nodes.get_mut(idx) {
                node.spatial.velocity = [0.0, 0.0, 0.0];
            }
        }

        physics::tick(&mut self.nodes, dt);
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
            Some(NodeType::Asteroid) => "☄",
            Some(NodeType::Planet) => "●",
            Some(NodeType::Satellite) => "◎",
            Some(NodeType::DysonSphere) => "◈",
            None => "?",
        }
    }

    /// Number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Delete a node by index (drag into Black Hole).
    pub fn remove_node(&mut self, idx: usize) {
        if idx < self.nodes.len() {
            self.nodes.remove(idx);
            // Fix up selected / dragged indices.
            self.selected = self.selected.and_then(|s| {
                if s == idx { None }
                else if s > idx { Some(s - 1) }
                else { Some(s) }
            });
            self.dragged = self.dragged.and_then(|d| {
                if d == idx { None }
                else if d > idx { Some(d - 1) }
                else { Some(d) }
            });
        }
    }
}
