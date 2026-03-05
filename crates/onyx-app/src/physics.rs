// ─── Stellar Physics Engine ────────────────────────────────────────
// Semi-implicit Euler integrator for VoidNode spatial dynamics.
//
// The physics simulation governs:
//   • Gravity    — Planets attract Asteroids; mass-proportional.
//   • Heat Decay — Nodes cool over time, drifting toward the Oort Cloud.
//   • Damping    — Velocity decays to prevent runaway oscillation.
//   • Repulsion  — Short-range repulsion prevents node overlap.
//   • Oort Drift — Cold nodes are gently pushed outward.
//
// All forces are computed in 2D (x, y) for Phase 2.  The z-axis
// is reserved for the Chronos Slider (temporal navigation).
//
// Tick rate: called once per frame (~60 Hz) from the App timer.
// ────────────────────────────────────────────────────────────────────

use onyx_core::void_node::{NodeType, VoidNode};

// ── Tuning constants ────────────────────────────────────────────

/// Gravitational constant (scales attraction force).
const G: f32 = 800.0;

/// Maximum gravitational force magnitude (prevents explosion on overlap).
const MAX_GRAVITY_FORCE: f32 = 200.0;

/// Short-range repulsion strength (prevents node overlap).
const REPULSION_STRENGTH: f32 = 5000.0;

/// Minimum distance^2 to prevent division by zero.
const MIN_DIST_SQ: f32 = 400.0;

/// Velocity damping factor per frame (0.98 = 2% loss per tick).
const DAMPING: f32 = 0.96;

/// Heat decay rate per second.  heat *= (1.0 - HEAT_DECAY * dt).
const HEAT_DECAY: f32 = 0.05;

/// Minimum heat before a node is considered "cold" (Oort-eligible).
const OORT_HEAT_THRESHOLD: f32 = 0.15;

/// Gentle outward drift force for cold (Oort Cloud) nodes.
const OORT_DRIFT_FORCE: f32 = 8.0;

/// Maximum velocity magnitude (speed limit).
const MAX_SPEED: f32 = 400.0;

/// Visual radius scale: radius = RADIUS_BASE + sqrt(mass) * RADIUS_SCALE.
const RADIUS_BASE: f32 = 12.0;

/// Visual radius scale factor.
const RADIUS_SCALE: f32 = 2.5;

// ── Public API ──────────────────────────────────────────────────

/// Compute the visual radius of a VoidNode based on its mass.
pub fn node_radius(node: &VoidNode) -> f32 {
    RADIUS_BASE + node.spatial.mass.sqrt() * RADIUS_SCALE
}

/// Run one physics tick.  `dt` is the elapsed time in seconds
/// since the last tick (typically ~0.016 for 60 fps).
///
/// Mutates all node positions and velocities in-place.
pub fn tick(nodes: &mut [VoidNode], dt: f32) {
    let dt = dt.min(0.05); // clamp to prevent physics explosion on lag spikes
    let n = nodes.len();
    if n == 0 {
        return;
    }

    // ── 1. Accumulate forces ─────────────────────────────────────
    // We store accelerations separately to avoid borrow conflicts.
    let mut accel = vec![[0.0f32; 2]; n];

    for i in 0..n {
        for j in (i + 1)..n {
            let dx = nodes[j].spatial.pos[0] - nodes[i].spatial.pos[0];
            let dy = nodes[j].spatial.pos[1] - nodes[i].spatial.pos[1];
            let dist_sq = (dx * dx + dy * dy).max(MIN_DIST_SQ);
            let dist = dist_sq.sqrt().max(0.1); // Clamp to avoid zero/NaN
            // If dx or dy is NaN, skip this pair
            if dx.is_nan() || dy.is_nan() || dist.is_nan() {
                continue;
            }
            let dist = dist_sq.sqrt();

            // ── Gravity (attractive, mass-proportional) ──
            // Only Planets exert meaningful gravity; Asteroids are light.
            let mi = effective_mass(&nodes[i]);
            let mj = effective_mass(&nodes[j]);

            let grav_mag = (G * mi * mj / dist_sq).min(MAX_GRAVITY_FORCE);
            let gx = grav_mag * (dx / dist);
            let gy = grav_mag * (dy / dist);
            // If gx or gy is NaN, skip force application
            if gx.is_nan() || gy.is_nan() {
                continue;
            }

            // Newton's 3rd law: equal and opposite
            accel[i][0] += gx / mi.max(1.0);
            accel[i][1] += gy / mi.max(1.0);
            accel[j][0] -= gx / mj.max(1.0);
            accel[j][1] -= gy / mj.max(1.0);

            // ── Short-range repulsion (overlap prevention) ──
            let ri = node_radius(&nodes[i]);
            let rj = node_radius(&nodes[j]);
            let min_dist = ri + rj;
            if dist < min_dist && !dist.is_nan() {
                let overlap = min_dist - dist;
                let rep_mag = REPULSION_STRENGTH * overlap / dist;
                let rx = rep_mag * (dx / dist);
                let ry = rep_mag * (dy / dist);
                accel[i][0] -= rx / mi.max(1.0);
                accel[i][1] -= ry / mi.max(1.0);
                accel[j][0] += rx / mj.max(1.0);
                accel[j][1] += ry / mj.max(1.0);
            }
        }

        // ── Oort Cloud drift (cold nodes pushed outward) ──
        if nodes[i].spatial.heat < OORT_HEAT_THRESHOLD {
            let px = nodes[i].spatial.pos[0];
            let py = nodes[i].spatial.pos[1];
            let center_dist = (px * px + py * py).sqrt().max(1.0);
            let coldness = 1.0 - nodes[i].spatial.heat / OORT_HEAT_THRESHOLD;
            accel[i][0] += OORT_DRIFT_FORCE * coldness * (px / center_dist);
            accel[i][1] += OORT_DRIFT_FORCE * coldness * (py / center_dist);
        }
    }

    // ── 2. Integrate (semi-implicit Euler) ───────────────────────
    for i in 0..n {
        let node = &mut nodes[i];

        // Update velocity
        node.spatial.velocity[0] += accel[i][0] * dt;
        node.spatial.velocity[1] += accel[i][1] * dt;

        // Damping
        node.spatial.velocity[0] *= DAMPING;
        node.spatial.velocity[1] *= DAMPING;

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
    }
}

/// Effective mass for gravity calculations.
/// Planets have amplified gravitational pull; Asteroids are light.
fn effective_mass(node: &VoidNode) -> f32 {
    match node.node_type {
        NodeType::Planet => node.spatial.mass * 3.0,
        NodeType::DysonSphere => node.spatial.mass * 2.0,
        NodeType::Satellite => node.spatial.mass * 0.5,
        NodeType::Asteroid => node.spatial.mass * 0.3,
    }
}
