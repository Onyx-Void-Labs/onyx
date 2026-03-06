// ─── Stellar Physics Engine ────────────────────────────────────────
// Semi-implicit Euler integrator for VoidNode spatial dynamics.
//
// The physics simulation governs:
//   • Gravity    — Planets attract Asteroids; mass-proportional.
//   • Heat Decay — Nodes cool over time, drifting toward the Oort Cloud.
//   • Damping    — Velocity decays to prevent runaway oscillation.
//   • Repulsion  — Short-range repulsion prevents node overlap.
//   • Oort Drift — Cold nodes are gently pushed outward.
//   • GridHash   — Spatial partitioning for O(N·k) when N > 100.
//
// All forces are computed in 2D (x, y) for Phase 2.  The z-axis
// is reserved for the Chronos Slider (temporal navigation).
//
// Tick rate: called once per frame (~60 Hz) from the App timer.
// ────────────────────────────────────────────────────────────────────

use onyx_core::void_node::{NodeType, VoidNode};
use std::collections::HashMap;

// ── Tuning constants ────────────────────────────────────────────

/// Gravitational constant (scales attraction force).
const G: f32 = 800.0;

/// Maximum gravitational force magnitude (prevents explosion on overlap).
const MAX_GRAVITY_FORCE: f32 = 200.0;

/// Short-range repulsion strength (prevents node overlap).
const REPULSION_STRENGTH: f32 = 3000.0;

/// Soft repulsion zone — inverse-square repulsion kicks in below this distance.
const SOFT_REPULSION_DIST: f32 = 60.0;

/// Minimum distance^2 to prevent division by zero.
const MIN_DIST_SQ: f32 = 400.0;

/// Hover arrest damping — applied when the user's cursor is over a node.
/// Much stronger than normal damping so the node "slows down" making
/// it easy to click moving targets.
const HOVER_DAMPING: f32 = 0.5;

/// Heat decay rate per second.  heat *= (1.0 - HEAT_DECAY * dt).
const HEAT_DECAY: f32 = 0.05;

/// Minimum heat before a node is considered "cold" (Oort-eligible).
const OORT_HEAT_THRESHOLD: f32 = 0.15;

/// Gentle outward drift force for cold (Oort Cloud) nodes.
const OORT_DRIFT_FORCE: f32 = 8.0;

/// Maximum velocity magnitude (speed limit).
/// Raised to accommodate throw inertia; damping brings it down.
const MAX_SPEED: f32 = 200.0;

/// Visual radius scale: radius = mass.sqrt() * 10.0 (Ignition Protocol).
const RADIUS_SINGULARITY: f32 = 25.0;

/// Damping: air resistance for thrown nodes. 0.94 = deliberate drag.
const THROW_DAMPING: f32 = 0.94;

/// Grid cell size for spatial partitioning.
const GRID_CELL_SIZE: f32 = 200.0;

/// Threshold: use GridHash when N > 100, else brute-force N^2.
const GRID_THRESHOLD: usize = 100;

/// Number of frames to track for throw velocity averaging.
#[allow(dead_code)]
const THROW_HISTORY_LEN: usize = 5;

// ── Throw History ───────────────────────────────────────────────

/// Ring buffer tracking mouse deltas for throw feel.
#[allow(dead_code)]
pub struct ThrowHistory {
    deltas: [(f32, f32); THROW_HISTORY_LEN],
    idx: usize,
}

impl Default for ThrowHistory {
    fn default() -> Self {
        Self {
            deltas: [(0.0, 0.0); THROW_HISTORY_LEN],
            idx: 0,
        }
    }
}

#[allow(dead_code)]
impl ThrowHistory {
    /// Record a per-frame delta position.
    pub fn push(&mut self, dx: f32, dy: f32) {
        self.deltas[self.idx % THROW_HISTORY_LEN] = (dx, dy);
        self.idx += 1;
    }

    /// Average delta over the last N frames × throw_force.
    pub fn average_velocity(&self, throw_force: f32) -> (f32, f32) {
        let mut sx = 0.0_f32;
        let mut sy = 0.0_f32;
        for &(dx, dy) in &self.deltas {
            sx += dx;
            sy += dy;
        }
        let n = THROW_HISTORY_LEN as f32;
        (sx / n * throw_force, sy / n * throw_force)
    }

    /// Clear the history.
    pub fn clear(&mut self) {
        self.deltas = [(0.0, 0.0); THROW_HISTORY_LEN];
        self.idx = 0;
    }
}

// ── GridHash: Spatial Partitioning ──────────────────────────────

/// Spatial hash grid for O(N·k) force accumulation.
/// Each cell stores indices into the node array.
struct GridHash {
    cells: HashMap<(i32, i32), Vec<usize>>,
    cell_size: f32,
}

impl GridHash {
    fn new(cell_size: f32) -> Self {
        Self {
            cells: HashMap::new(),
            cell_size,
        }
    }

    fn cell_key(&self, x: f32, y: f32) -> (i32, i32) {
        let cx = (x / self.cell_size).floor() as i32;
        let cy = (y / self.cell_size).floor() as i32;
        (cx, cy)
    }

    fn insert(&mut self, idx: usize, x: f32, y: f32) {
        let key = self.cell_key(x, y);
        self.cells.entry(key).or_default().push(idx);
    }

    /// Return all node indices in the 3×3 neighbourhood of (x, y).
    fn query_neighbours(&self, x: f32, y: f32) -> Vec<usize> {
        let (cx, cy) = self.cell_key(x, y);
        let mut result = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(cell) = self.cells.get(&(cx + dx, cy + dy)) {
                    result.extend_from_slice(cell);
                }
            }
        }
        result
    }
}

// ── Public API ──────────────────────────────────────────────────────

/// Compute the visual radius of a VoidNode based on its mass.
/// Ignition Protocol: radius = mass.sqrt() * 10.0.
/// BlackHole and WhiteHole use a fixed singularity radius.
pub fn node_radius(node: &VoidNode) -> f32 {
    match node.node_type {
        NodeType::BlackHole | NodeType::WhiteHole => RADIUS_SINGULARITY,
        _ => node.spatial.mass.sqrt() * 10.0,
    }
}

/// Run one physics tick.  `dt` is the elapsed time in seconds
/// since the last tick (typically ~0.016 for 60 fps).
/// `screen_w` / `screen_h` are the viewport pixel dimensions.
/// `view_center_x` / `view_center_y` are the camera center in world space.
/// `zoom` is the camera zoom level (1.0 = default).
///
/// Uses GridHash spatial partitioning when N > 100 (O(N·k) vs O(N²)).
/// Falls back to brute-force N² for small node counts.
///
/// Mutates all node positions and velocities in-place.
pub fn tick(
    nodes: &mut [VoidNode],
    dt: f32,
    screen_w: f32,
    screen_h: f32,
    view_center_x: f32,
    view_center_y: f32,
    zoom: f32,
) {
    let dt = dt.min(0.05); // clamp to prevent physics explosion on lag spikes
    let n = nodes.len();
    if n == 0 {
        return;
    }

    // ── 0. Inverse Camera Projection — Tether Singularities ──────
    // Visible world bounds: half-extents in world space.
    let zoom_safe = zoom.max(0.01);
    let half_w = (screen_w / 2.0) / zoom_safe;
    let half_h = (screen_h / 2.0) / zoom_safe;
    // 150 screen-px margin, converted to world space.
    let margin = 150.0 / zoom_safe;
    for node in nodes.iter_mut() {
        match node.node_type {
            // BlackHole → bottom-right corner
            NodeType::BlackHole => {
                node.spatial.pos[0] = view_center_x + half_w - margin;
                node.spatial.pos[1] = view_center_y + half_h - margin;
                node.spatial.velocity = [0.0, 0.0, 0.0];
            }
            // WhiteHole → bottom-left corner
            NodeType::WhiteHole => {
                node.spatial.pos[0] = view_center_x - half_w + margin;
                node.spatial.pos[1] = view_center_y + half_h - margin;
                node.spatial.velocity = [0.0, 0.0, 0.0];
            }
            _ => {}
        }
    }

    // ── 1. Accumulate forces ─────────────────────────────────────
    let mut accel = vec![[0.0f32; 2]; n];

    if n > GRID_THRESHOLD {
        // ── GridHash path: O(N·k) ──
        let mut grid = GridHash::new(GRID_CELL_SIZE);
        for (i, node) in nodes.iter().enumerate() {
            grid.insert(i, node.spatial.pos[0], node.spatial.pos[1]);
        }

        for i in 0..n {
            let neighbours = grid.query_neighbours(nodes[i].spatial.pos[0], nodes[i].spatial.pos[1]);
            for &j in &neighbours {
                if j <= i {
                    continue; // avoid double-counting and self-interaction
                }
                accumulate_pair_forces(nodes, &mut accel, i, j);
            }
            accumulate_single_forces(nodes, &mut accel, i);
        }
    } else {
        // ── Brute-force N² path ──
        for i in 0..n {
            for j in (i + 1)..n {
                accumulate_pair_forces(nodes, &mut accel, i, j);
            }
            accumulate_single_forces(nodes, &mut accel, i);
        }
    }

    // ── 2. Integrate (semi-implicit Euler) ───────────────────────
    for i in 0..n {
        let node = &mut nodes[i];

        // ── Kinematic lock: dragged nodes get zero velocity, skip integration ──
        if node.spatial.is_dragged
            || matches!(node.node_type, NodeType::BlackHole | NodeType::WhiteHole)
        {
            node.spatial.velocity = [0.0, 0.0, 0.0];
            // Still decay heat
            node.spatial.heat *= 1.0 - HEAT_DECAY * dt;
            node.spatial.heat = node.spatial.heat.max(0.0);
            continue;
        }

        // Update velocity
        node.spatial.velocity[0] += accel[i][0] * dt;
        node.spatial.velocity[1] += accel[i][1] * dt;

        // Adaptive damping — thrown nodes use 0.94 (air resistance),
        // hover nodes arrest hard, slow nodes stop firmly.
        let speed_sq = node.spatial.velocity[0].powi(2) + node.spatial.velocity[1].powi(2);
        let damp = if node.spatial.hovered {
            HOVER_DAMPING
        } else if speed_sq > 50.0 * 50.0 {
            THROW_DAMPING // 0.94 — deliberate air resistance on throw
        } else {
            0.92 // firm stop when slow
        };
        node.spatial.velocity[0] *= damp;
        node.spatial.velocity[1] *= damp;

        // Speed limit
        let speed_sq = node.spatial.velocity[0].powi(2) + node.spatial.velocity[1].powi(2);
        if speed_sq > MAX_SPEED * MAX_SPEED {
            let speed = speed_sq.sqrt();
            node.spatial.velocity[0] *= MAX_SPEED / speed;
            node.spatial.velocity[1] *= MAX_SPEED / speed;
        }

        // Update position
        node.spatial.pos[0] += node.spatial.velocity[0] * dt;
        node.spatial.pos[1] += node.spatial.velocity[1] * dt;

        // ── 3. Heat decay ──
        node.spatial.heat *= 1.0 - HEAT_DECAY * dt;
        node.spatial.heat = node.spatial.heat.max(0.0);

        // ── 4. NaN quarantine — forcefully sanitize ──
        if node.spatial.pos[0].is_nan() {
            node.spatial.pos[0] = 0.0;
        }
        if node.spatial.pos[1].is_nan() {
            node.spatial.pos[1] = 0.0;
        }
        if node.spatial.velocity[0].is_nan() {
            node.spatial.velocity[0] = 0.0;
        }
        if node.spatial.velocity[1].is_nan() {
            node.spatial.velocity[1] = 0.0;
        }
    }

    // ── 5. Singularity Collisions (BlackHole / WhiteHole) ────
    let mut tombstone_indices: Vec<usize> = Vec::new();
    let mut kick_list: Vec<(usize, f32, f32)> = Vec::new();

    for i in 0..n {
        if nodes[i].tombstone {
            continue;
        }
        if matches!(
            nodes[i].node_type,
            NodeType::BlackHole | NodeType::WhiteHole
        ) {
            continue;
        }

        for j in 0..n {
            if i == j || nodes[j].tombstone {
                continue;
            }

            let dx = nodes[j].spatial.pos[0] - nodes[i].spatial.pos[0];
            let dy = nodes[j].spatial.pos[1] - nodes[i].spatial.pos[1];
            let dist = (dx * dx + dy * dy).sqrt().max(0.1);

            let ri = node_radius(&nodes[i]);
            let rj = node_radius(&nodes[j]);
            let radius_sum = ri + rj;

            if dist < radius_sum {
                match nodes[j].node_type {
                    NodeType::BlackHole => {
                        // Safety physics: only tombstone if the node is being
                        // intentionally dragged into the BlackHole.
                        if nodes[i].spatial.is_dragged {
                            tombstone_indices.push(i);
                        } else {
                            // Safety bounce — repel drifting nodes away
                            let nx = dx / dist;
                            let ny = dy / dist;
                            kick_list.push((i, -nx * 800.0, -ny * 800.0));
                        }
                    }
                    NodeType::WhiteHole => {
                        println!("EXPORTING NODE [{}]", nodes[i].id);
                        let nx = dx / dist;
                        let ny = dy / dist;
                        kick_list.push((i, -nx * 500.0, -ny * 500.0));
                    }
                    _ => {}
                }
            }
        }
    }

    for idx in tombstone_indices {
        nodes[idx].tombstone = true;
    }
    for (idx, vx, vy) in kick_list {
        nodes[idx].spatial.velocity[0] += vx;
        nodes[idx].spatial.velocity[1] += vy;
    }
}

/// Accumulate gravitational + repulsion forces for a single pair (i, j).
fn accumulate_pair_forces(
    nodes: &[VoidNode],
    accel: &mut [[f32; 2]],
    i: usize,
    j: usize,
) {
    // ── Kinematic lock: skip force accumulation for dragged nodes ──
    let i_dragged = nodes[i].spatial.is_dragged;
    let j_dragged = nodes[j].spatial.is_dragged;
    let i_kinematic = matches!(
        nodes[i].node_type,
        NodeType::BlackHole | NodeType::WhiteHole
    );
    let j_kinematic = matches!(
        nodes[j].node_type,
        NodeType::BlackHole | NodeType::WhiteHole
    );

    let dx = nodes[j].spatial.pos[0] - nodes[i].spatial.pos[0];
    let dy = nodes[j].spatial.pos[1] - nodes[i].spatial.pos[1];
    let dist_sq = (dx * dx + dy * dy).max(MIN_DIST_SQ);
    let dist = dist_sq.sqrt().max(0.1);
    // If any component is NaN, skip this pair entirely
    if dx.is_nan() || dy.is_nan() || dist.is_nan() {
        return;
    }

    // ── Gravity (attractive, mass-proportional) ──
    let mi = effective_mass(&nodes[i]);
    let mj = effective_mass(&nodes[j]);

    let grav_mag = (G * mi * mj / dist_sq).min(MAX_GRAVITY_FORCE);
    let gx = grav_mag * (dx / dist);
    let gy = grav_mag * (dy / dist);
    if gx.is_nan() || gy.is_nan() {
        return;
    }

    // ── Safety Field: BlackHole repulsion for non-dragged nodes ──
    let j_is_bh = matches!(nodes[j].node_type, NodeType::BlackHole);
    let i_is_bh = matches!(nodes[i].node_type, NodeType::BlackHole);
    let safety_i = j_is_bh && !i_dragged && !i_kinematic;
    let safety_j = i_is_bh && !j_dragged && !j_kinematic;
    let force_sign_i = if safety_i { -3.0_f32 } else { 1.0 };
    let force_sign_j = if safety_j { -3.0_f32 } else { 1.0 };

    // Newton's 3rd law: equal and opposite
    if !i_dragged && !i_kinematic {
        accel[i][0] += gx / mi.max(1.0) * force_sign_i;
        accel[i][1] += gy / mi.max(1.0) * force_sign_i;
    }
    if !j_dragged && !j_kinematic {
        accel[j][0] -= gx / mj.max(1.0) * force_sign_j;
        accel[j][1] -= gy / mj.max(1.0) * force_sign_j;
    }

    // ── Surface-to-Surface Repulsion (rigid separation) ──
    let ri = node_radius(&nodes[i]);
    let rj = node_radius(&nodes[j]);
    let min_dist = ri + rj + 10.0;
    if dist < min_dist {
        let overlap = min_dist - dist;
        let sep_strength = 50.0 * overlap;
        let nx = dx / dist;
        let ny = dy / dist;
        if !i_dragged && !i_kinematic {
            accel[i][0] -= sep_strength * nx / mi.max(1.0);
            accel[i][1] -= sep_strength * ny / mi.max(1.0);
        }
        if !j_dragged && !j_kinematic {
            accel[j][0] += sep_strength * nx / mj.max(1.0);
            accel[j][1] += sep_strength * ny / mj.max(1.0);
        }
    }

    // ── Soft inverse-square repulsion (smooth push when close) ──
    if dist < SOFT_REPULSION_DIST && !dist.is_nan() {
        let safe_dist = dist.max(5.0);
        let rep_mag = REPULSION_STRENGTH / (safe_dist * safe_dist);
        let rx = rep_mag * (dx / dist);
        let ry = rep_mag * (dy / dist);
        if !i_dragged && !i_kinematic {
            accel[i][0] -= rx / mi.max(1.0);
            accel[i][1] -= ry / mi.max(1.0);
        }
        if !j_dragged && !j_kinematic {
            accel[j][0] += rx / mj.max(1.0);
            accel[j][1] += ry / mj.max(1.0);
        }
    }
}

/// Accumulate single-node forces: Oort drift + anti-clump tangential.
fn accumulate_single_forces(
    nodes: &[VoidNode],
    accel: &mut [[f32; 2]],
    i: usize,
) {
    // ── Oort Cloud drift (cold nodes pushed outward) ──
    if nodes[i].spatial.heat < OORT_HEAT_THRESHOLD {
        let px = nodes[i].spatial.pos[0];
        let py = nodes[i].spatial.pos[1];
        let center_dist = (px * px + py * py).sqrt().max(1.0);
        let coldness = 1.0 - nodes[i].spatial.heat / OORT_HEAT_THRESHOLD;
        accel[i][0] += OORT_DRIFT_FORCE * coldness * (px / center_dist);
        accel[i][1] += OORT_DRIFT_FORCE * coldness * (py / center_dist);
    }

    // ── Anti-clump tangential injection (orbital mechanics) ──
    if !nodes[i].spatial.is_dragged {
        let px = nodes[i].spatial.pos[0];
        let py = nodes[i].spatial.pos[1];
        let center_dist = (px * px + py * py).sqrt().max(1.0);
        if center_dist < 300.0 {
            let dir_x = px / center_dist;
            let dir_y = py / center_dist;
            let tang_x = -dir_y;
            let tang_y = dir_x;
            let tang_strength = 15.0 / center_dist.max(20.0);
            accel[i][0] += tang_x * tang_strength;
            accel[i][1] += tang_y * tang_strength;
        }
    }
}

/// Effective mass for gravity calculations.
/// Sun has dominant pull; BlackHole has infinite-equivalent mass.
fn effective_mass(node: &VoidNode) -> f32 {
    match node.node_type {
        NodeType::Sun => node.spatial.mass * 5.0,
        NodeType::BlackHole => 1_000_000.0,
        NodeType::WhiteHole => 500_000.0,
        NodeType::GasGiant => node.spatial.mass * 3.0,
        NodeType::RockyPlanet => node.spatial.mass * 1.5,
        NodeType::Asteroid => node.spatial.mass * 0.3,
    }
}
