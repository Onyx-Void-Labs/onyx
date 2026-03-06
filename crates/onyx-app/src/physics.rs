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
const REPULSION_STRENGTH: f32 = 3000.0;

/// Soft repulsion zone — inverse-square repulsion kicks in below this distance.
const SOFT_REPULSION_DIST: f32 = 60.0;

/// Minimum distance^2 to prevent division by zero.
const MIN_DIST_SQ: f32 = 400.0;

/// Velocity damping factor per frame (0.92 = "space friction").
/// Lower = more viscous, more majestic orbital glide.
#[allow(dead_code)]
const DAMPING: f32 = 0.92;

/// Throw damping — applied at high velocities so thrown nodes
/// retain momentum longer before settling to normal DAMPING.
#[allow(dead_code)]
const THROW_DAMPING: f32 = 0.98;

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
                continue;
            }

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

            // ── Safety Field: BlackHole repulsion for non-dragged nodes ──
            // When a Normal node drifts near a BlackHole without being dragged,
            // reverse gravity into strong repulsion ("safety bounce").
            // Only intentional drag allows attraction (pull-to-delete).
            let j_is_bh = matches!(nodes[j].node_type, NodeType::BlackHole);
            let i_is_bh = matches!(nodes[i].node_type, NodeType::BlackHole);
            let safety_i = j_is_bh && !i_dragged && !i_kinematic;
            let safety_j = i_is_bh && !j_dragged && !j_kinematic;
            let force_sign_i = if safety_i { -3.0_f32 } else { 1.0 };
            let force_sign_j = if safety_j { -3.0_f32 } else { 1.0 };

            // Newton's 3rd law: equal and opposite
            // Skip force application for dragged/kinematic nodes
            if !i_dragged && !i_kinematic {
                accel[i][0] += gx / mi.max(1.0) * force_sign_i;
                accel[i][1] += gy / mi.max(1.0) * force_sign_i;
            }
            if !j_dragged && !j_kinematic {
                accel[j][0] -= gx / mj.max(1.0) * force_sign_j;
                accel[j][1] -= gy / mj.max(1.0) * force_sign_j;
            }

            // ── Surface-to-Surface Repulsion (rigid separation) ──
            // Uses actual visual radii + padding to prevent overlap.
            let ri = node_radius(&nodes[i]);
            let rj = node_radius(&nodes[j]);
            let min_dist = ri + rj + 10.0; // 10px padding
            if dist < min_dist {
                let overlap = min_dist - dist;
                // Rigid separation: force proportional to overlap
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
            // Active below SOFT_REPULSION_DIST — prevents overlap without
            // violent jitter.  Uses inverse-square falloff for a gentle
            // pressure gradient instead of a hard spring.
            if dist < SOFT_REPULSION_DIST && !dist.is_nan() {
                // Soft repulsion: strength ∝ 1/dist² (clamped to avoid explosion)
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
        // If a node is being pulled toward center [0,0], add a
        // perpendicular force [-dir.y, dir.x] so nodes swirl into
        // orbit instead of crashing straight into the center.
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

        // Adaptive damping — thrown nodes retain momentum longer
        // (0.98 at high speed, settling to 0.92 when slow).
        // Hover arrest uses even stronger damping.
        let speed_sq = node.spatial.velocity[0].powi(2) + node.spatial.velocity[1].powi(2);
        let speed = speed_sq.sqrt();
        let damp = if node.spatial.hovered {
            HOVER_DAMPING
        } else if speed > 50.0 {
            // Aero-friction: glides when thrown fast
            0.99
        } else {
            // Stops firmly when slow
            0.92
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
