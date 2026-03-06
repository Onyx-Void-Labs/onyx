// ─── Phase 4 Stellar Physics Engine ───────────────────────────────
// O(N²) CPU baseline for the Singularity Engine.
//
// This is the robust reference implementation.  When N exceeds ~1000
// nodes, this will be migrated to GPU compute shaders.  For now,
// correctness and NaN-safety are the absolute priority.
//
// Invariants:
//   • Every distance clamped: dist.max(0.1)
//   • Damping is exactly 0.92 per tick
//   • Sun and Utility nodes are kinematic (infinite mass, fixed)
//   • Every tick ends with recursive NaN quarantine
//   • Pure Rust — zero C-library deps — Android-safe
// ─────────────────────────────────────────────────────────────────────

use crate::core_state::{NodeType, VoidNode};

// ── Tuning Constants ────────────────────────────────────────────

/// Gravitational constant (scales attraction force).
const G: f32 = 800.0;

/// Maximum gravitational force magnitude (prevents explosion on overlap).
const MAX_GRAVITY_FORCE: f32 = 200.0;

/// Short-range repulsion strength (prevents node overlap).
const REPULSION_STRENGTH: f32 = 3000.0;

/// Soft repulsion zone — inverse-square repulsion kicks in below this dist.
const SOFT_REPULSION_DIST: f32 = 60.0;

/// Minimum distance² to prevent division by zero.
const MIN_DIST_SQ: f32 = 400.0;

/// Velocity damping factor per tick (legacy — now using speed-threshold logic).
#[allow(dead_code)]
const DAMPING: f32 = 0.92;

/// Throw damping (legacy — now using speed-threshold logic).
#[allow(dead_code)]
const THROW_DAMPING: f32 = 0.98;

/// Maximum velocity magnitude (speed limit).
const MAX_SPEED: f32 = 40.0;

/// Effective mass assigned to kinematic bodies (Sun, Utility).
/// Large enough for dominant attraction, finite to prevent f32 overflow.
const KINEMATIC_MASS: f32 = 1_000_000.0;

// ── Physics Engine ──────────────────────────────────────────────

/// The Phase 4 Stellar Dynamics physics engine.
///
/// Stateless O(N²) integrator — all state lives in the `VoidNode` slice.
/// Call `tick()` once per frame with the full node array and delta time.
#[derive(Debug, Clone, Default)]
pub struct PhysicsEngine;

impl PhysicsEngine {
    pub fn new() -> Self {
        Self
    }

    /// Run one physics tick.
    ///
    /// `dt` is elapsed time in seconds since the last tick.
    /// Clamped internally to 0.05s to prevent physics explosion on lag.
    /// `screen_w` and `screen_h` are the pixel viewport dimensions.
    /// `view_center_x` / `view_center_y` are the camera center in world space.
    /// `zoom` is the camera zoom level (1.0 = default).
    ///
    /// # Algorithm
    /// 1. Inverse camera projection — tether singularities to screen corners
    /// 2. O(N²) pairwise force accumulation (gravity + repulsion)
    /// 3. Semi-implicit Euler integration with 0.92 damping
    /// 4. Recursive NaN quarantine on all position/velocity vectors
    #[allow(clippy::too_many_arguments)]
    pub fn tick(
        &mut self,
        nodes: &mut [VoidNode],
        dt: f32,
        screen_w: f32,
        screen_h: f32,
        view_center_x: f64,
        view_center_y: f64,
        zoom: f64,
    ) {
        let dt = dt.min(0.05);
        let n = nodes.len();
        if n == 0 {
            return;
        }

        // ── 0. Inverse Camera Projection — Tether Singularities ────
        // Visible world bounds: half-extents in world space.
        let zoom_safe = zoom.max(0.01);
        let half_w = (screen_w as f64 / 2.0) / zoom_safe;
        let half_h = (screen_h as f64 / 2.0) / zoom_safe;
        // 150 screen-px margin, converted to world space.
        let margin = 150.0 / zoom_safe;
        for node in nodes.iter_mut() {
            match node.node_type {
                // BlackHole → bottom-right corner
                NodeType::BlackHole => {
                    node.position = [
                        (view_center_x + half_w - margin) as f32,
                        (view_center_y + half_h - margin) as f32,
                        0.0,
                    ];
                    node.velocity = [0.0, 0.0, 0.0];
                }
                // WhiteHole → bottom-left corner
                NodeType::WhiteHole => {
                    node.position = [
                        (view_center_x - half_w + margin) as f32,
                        (view_center_y + half_h - margin) as f32,
                        0.0,
                    ];
                    node.velocity = [0.0, 0.0, 0.0];
                }
                _ => {}
            }
        }

        // ── 1. Force Accumulation (O(N²) pairwise) ─────────────
        let mut accel = vec![[0.0f32; 3]; n];

        for i in 0..n {
            let i_kinematic = Self::is_kinematic(&nodes[i]);

            for j in (i + 1)..n {
                let j_kinematic = Self::is_kinematic(&nodes[j]);

                let dx = nodes[j].position[0] - nodes[i].position[0];
                let dy = nodes[j].position[1] - nodes[i].position[1];
                let dz = nodes[j].position[2] - nodes[i].position[2];

                // NaN pair rejection — skip entirely if any delta is corrupt
                if dx.is_nan() || dy.is_nan() || dz.is_nan() {
                    continue;
                }

                let dist_sq = (dx * dx + dy * dy + dz * dz).max(MIN_DIST_SQ);
                let dist = dist_sq.sqrt().max(0.1);

                if dist.is_nan() {
                    continue;
                }

                let mi = Self::effective_mass(&nodes[i]);
                let mj = Self::effective_mass(&nodes[j]);

                // Unit direction vector (i → j)
                let nx = dx / dist;
                let ny = dy / dist;
                let nz = dz / dist;

                // ── Gravity (attractive, mass-proportional) ──
                let grav_mag = (G * mi * mj / dist_sq).min(MAX_GRAVITY_FORCE);
                let gx = grav_mag * nx;
                let gy = grav_mag * ny;
                let gz = grav_mag * nz;

                if gx.is_nan() || gy.is_nan() || gz.is_nan() {
                    continue;
                }

                // ── Safety Field: BlackHole repulsion for non-kinematic nodes ──
                let j_is_bh = matches!(nodes[j].node_type, NodeType::BlackHole);
                let i_is_bh = matches!(nodes[i].node_type, NodeType::BlackHole);
                let safety_i = j_is_bh && !i_kinematic;
                let safety_j = i_is_bh && !j_kinematic;
                let force_sign_i = if safety_i { -3.0_f32 } else { 1.0 };
                let force_sign_j = if safety_j { -3.0_f32 } else { 1.0 };

                // Newton's 3rd law — equal and opposite
                if !i_kinematic {
                    let inv_mi = 1.0 / mi.max(1.0);
                    accel[i][0] += gx * inv_mi * force_sign_i;
                    accel[i][1] += gy * inv_mi * force_sign_i;
                    accel[i][2] += gz * inv_mi * force_sign_i;
                }
                if !j_kinematic {
                    let inv_mj = 1.0 / mj.max(1.0);
                    accel[j][0] -= gx * inv_mj * force_sign_j;
                    accel[j][1] -= gy * inv_mj * force_sign_j;
                    accel[j][2] -= gz * inv_mj * force_sign_j;
                }

                // ── Surface-to-Surface Repulsion (rigid separation) ──
                let min_dist = 30.0 + 10.0; // base node diameter + padding
                if dist < min_dist {
                    let overlap = min_dist - dist;
                    let sep_strength = 50.0 * overlap;
                    if !i_kinematic {
                        let inv_mi = 1.0 / mi.max(1.0);
                        accel[i][0] -= sep_strength * nx * inv_mi;
                        accel[i][1] -= sep_strength * ny * inv_mi;
                        accel[i][2] -= sep_strength * nz * inv_mi;
                    }
                    if !j_kinematic {
                        let inv_mj = 1.0 / mj.max(1.0);
                        accel[j][0] += sep_strength * nx * inv_mj;
                        accel[j][1] += sep_strength * ny * inv_mj;
                        accel[j][2] += sep_strength * nz * inv_mj;
                    }
                }

                // ── Soft Repulsion (short-range, prevents overlap) ──
                if dist < SOFT_REPULSION_DIST {
                    let safe_dist = dist.max(5.0);
                    let rep_mag = REPULSION_STRENGTH / (safe_dist * safe_dist);
                    let rx = rep_mag * nx;
                    let ry = rep_mag * ny;
                    let rz = rep_mag * nz;

                    if !i_kinematic {
                        let inv_mi = 1.0 / mi.max(1.0);
                        accel[i][0] -= rx * inv_mi;
                        accel[i][1] -= ry * inv_mi;
                        accel[i][2] -= rz * inv_mi;
                    }
                    if !j_kinematic {
                        let inv_mj = 1.0 / mj.max(1.0);
                        accel[j][0] += rx * inv_mj;
                        accel[j][1] += ry * inv_mj;
                        accel[j][2] += rz * inv_mj;
                    }
                }
            }
            // Anti-clump tangential injection — prevents straight-line center crash.
            // Adds perpendicular force [-dir.y, dir.x] so nodes swirl into orbit.
            if !i_kinematic {
                let px = nodes[i].position[0];
                let py = nodes[i].position[1];
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

        // ── 2. Semi-Implicit Euler Integration ──────────────────
        for i in 0..n {
            if Self::is_kinematic(&nodes[i]) {
                // Kinematic bodies: zero velocity, fixed position
                nodes[i].velocity = [0.0, 0.0, 0.0];
                continue;
            }

            let node = &mut nodes[i];

            // Apply accumulated acceleration
            node.velocity[0] += accel[i][0] * dt;
            node.velocity[1] += accel[i][1] * dt;
            node.velocity[2] += accel[i][2] * dt;

            // Aero-friction damping: glides when thrown fast, stops firmly when slow.
            let speed_sq =
                node.velocity[0].powi(2) + node.velocity[1].powi(2) + node.velocity[2].powi(2);
            let speed = speed_sq.sqrt();
            let damp = if speed > 50.0 { 0.99 } else { 0.92 };
            node.velocity[0] *= damp;
            node.velocity[1] *= damp;
            node.velocity[2] *= damp;

            // Speed clamping
            let speed_sq =
                node.velocity[0].powi(2) + node.velocity[1].powi(2) + node.velocity[2].powi(2);
            if speed_sq > MAX_SPEED * MAX_SPEED {
                let speed = speed_sq.sqrt().max(0.1);
                let scale = MAX_SPEED / speed;
                node.velocity[0] *= scale;
                node.velocity[1] *= scale;
                node.velocity[2] *= scale;
            }

            // Position update
            node.position[0] += node.velocity[0] * dt;
            node.position[1] += node.velocity[1] * dt;
            node.position[2] += node.velocity[2] * dt;
        }

        // ── 3. NaN Quarantine (recursive sanitization) ──────────
        Self::nan_quarantine(nodes);

        // ── 4. Singularity Collisions (BlackHole / WhiteHole) ────
        Self::singularity_collisions(nodes);
    }

    /// Returns `true` if this node type is kinematic (infinite mass, fixed).
    /// Sun nodes never move.
    #[inline]
    fn is_kinematic(node: &VoidNode) -> bool {
        matches!(
            node.node_type,
            NodeType::Sun | NodeType::BlackHole | NodeType::WhiteHole
        )
    }

    /// Effective mass for gravity calculations.
    ///
    /// Kinematic bodies (Sun, Utility) get `KINEMATIC_MASS` for dominant
    /// attraction without f32 overflow.  Non-kinematic types scale their
    /// stored mass by a type-specific multiplier.
    #[inline]
    fn effective_mass(node: &VoidNode) -> f32 {
        match node.node_type {
            NodeType::Sun => KINEMATIC_MASS,
            NodeType::BlackHole => KINEMATIC_MASS,
            NodeType::WhiteHole => KINEMATIC_MASS * 0.5,
            NodeType::GasGiant => node.mass * 3.0,
            NodeType::RockyPlanet => node.mass * 1.5,
            NodeType::Asteroid => node.mass * 0.3,
        }
    }

    /// Recursive NaN quarantine.
    ///
    /// Checks every component of every position and velocity vector.
    /// If ANY component is NaN, the ENTIRE vector is reset to `[0, 0, 0]`.
    /// This prevents NaN propagation through the physics pipeline.
    fn nan_quarantine(nodes: &mut [VoidNode]) {
        for node in nodes.iter_mut() {
            let pos_corrupted =
                node.position[0].is_nan() || node.position[1].is_nan() || node.position[2].is_nan();
            if pos_corrupted {
                node.position = [0.0, 0.0, 0.0];
            }

            let vel_corrupted =
                node.velocity[0].is_nan() || node.velocity[1].is_nan() || node.velocity[2].is_nan();
            if vel_corrupted {
                node.velocity = [0.0, 0.0, 0.0];
            }
        }
    }

    /// Singularity collision detection.
    ///
    /// - Normal node overlaps a **BlackHole** → `tombstone = true`.
    /// - Normal node overlaps a **WhiteHole** → console log + massive repulsion kick.
    fn singularity_collisions(nodes: &mut [VoidNode]) {
        let n = nodes.len();

        // Collect deferred mutations to avoid aliased &mut borrows.
        let tombstone_indices: Vec<usize> = Vec::new();
        let mut kick_list: Vec<(usize, f32, f32, f32)> = Vec::new();

        for i in 0..n {
            if nodes[i].tombstone {
                continue;
            }
            // Only normal nodes can be affected
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

                let dx = nodes[j].position[0] - nodes[i].position[0];
                let dy = nodes[j].position[1] - nodes[i].position[1];
                let dz = nodes[j].position[2] - nodes[i].position[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.1);

                let ri = nodes[i].mass.sqrt() * 10.0;
                let rj = nodes[j].mass.sqrt() * 10.0;
                let radius_sum = ri + rj;

                if dist < radius_sum {
                    match nodes[j].node_type {
                        NodeType::BlackHole => {
                            // Safety: only tombstone if no safety mechanism
                            // (core_state VoidNode has no is_dragged; bounce instead)
                            let nx = dx / dist;
                            let ny = dy / dist;
                            let nz = dz / dist;
                            kick_list.push((i, -nx * 800.0, -ny * 800.0, -nz * 800.0));
                        }
                        NodeType::WhiteHole => {
                            println!("EXPORTING NODE [{}]", nodes[i].id);
                            let nx = dx / dist;
                            let ny = dy / dist;
                            let nz = dz / dist;
                            kick_list.push((i, -nx * 500.0, -ny * 500.0, -nz * 500.0));
                        }
                        _ => {}
                    }
                }
            }
        }

        for idx in tombstone_indices {
            nodes[idx].tombstone = true;
        }
        for (idx, vx, vy, vz) in kick_list {
            nodes[idx].velocity[0] += vx;
            nodes[idx].velocity[1] += vy;
            nodes[idx].velocity[2] += vz;
        }
    }
}
