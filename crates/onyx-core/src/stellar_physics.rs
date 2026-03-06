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

/// Velocity damping factor per tick.  Exactly 0.92 — non-negotiable.
const DAMPING: f32 = 0.92;

/// Maximum velocity magnitude (speed limit).
const MAX_SPEED: f32 = 10.0;

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
    ///
    /// # Algorithm
    /// 1. O(N²) pairwise force accumulation (gravity + repulsion)
    /// 2. Semi-implicit Euler integration with 0.92 damping
    /// 3. Recursive NaN quarantine on all position/velocity vectors
    pub fn tick(&mut self, nodes: &mut [VoidNode], dt: f32) {
        let dt = dt.min(0.05);
        let n = nodes.len();
        if n == 0 {
            return;
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

                // Newton's 3rd law — equal and opposite
                if !i_kinematic {
                    let inv_mi = 1.0 / mi.max(1.0);
                    accel[i][0] += gx * inv_mi;
                    accel[i][1] += gy * inv_mi;
                    accel[i][2] += gz * inv_mi;
                }
                if !j_kinematic {
                    let inv_mj = 1.0 / mj.max(1.0);
                    accel[j][0] -= gx * inv_mj;
                    accel[j][1] -= gy * inv_mj;
                    accel[j][2] -= gz * inv_mj;
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

            // Damping — exactly 0.92 per tick (game feel)
            node.velocity[0] *= DAMPING;
            node.velocity[1] *= DAMPING;
            node.velocity[2] *= DAMPING;

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
    }

    /// Returns `true` if this node type is kinematic (infinite mass, fixed).
    /// Sun and Utility nodes never move.
    #[inline]
    fn is_kinematic(node: &VoidNode) -> bool {
        matches!(node.node_type, NodeType::Sun | NodeType::Utility(_))
    }

    /// Effective mass for gravity calculations.
    ///
    /// Kinematic bodies (Sun, Utility) get `KINEMATIC_MASS` for dominant
    /// attraction without f32 overflow.  Non-kinematic types scale their
    /// stored mass by a type-specific multiplier.
    #[inline]
    fn effective_mass(node: &VoidNode) -> f32 {
        match node.node_type {
            NodeType::Sun | NodeType::Utility(_) => KINEMATIC_MASS,
            NodeType::GasGiant => node.mass * 3.0,
            NodeType::RockyMoon => node.mass * 0.5,
            NodeType::Atom => node.mass,
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
}
