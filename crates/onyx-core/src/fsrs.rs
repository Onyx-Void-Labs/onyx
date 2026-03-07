// ─── Onyx Core — FSRS v4 (Free Spaced Repetition Scheduler) ────────

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── FSRS Constants ──────────────────────────────────────────────────

const DECAY: f64 = -0.5;
const FACTOR: f64 = 1.0; // -0.5 / DECAY where DECAY is -0.5

/// Default FSRS-4.5 parameters (w0..w16).
const W: [f64; 17] = [
    0.4,  // w0:  initial stability for Again
    0.6,  // w1:  initial stability for Hard
    2.4,  // w2:  initial stability for Good
    5.8,  // w3:  initial stability for Easy
    4.93, // w4:  initial difficulty mean
    0.94, // w5:  difficulty grade scaling
    0.86, // w6:  difficulty update rate
    0.01, // w7:  mean reversion weight
    1.49, // w8:  stability increase log factor
    0.14, // w9:  stability decrease exponent
    0.94, // w10: recall bonus factor
    2.18, // w11: lapse stability base
    0.05, // w12: lapse difficulty exponent
    0.34, // w13: lapse stability power
    1.26, // w14: lapse recall penalty
    0.29, // w15: hard penalty
    2.61, // w16: easy bonus
];

/// Desired retention rate (90%).
const DESIRED_RETENTION: f64 = 0.9;

// ── Data Structures ─────────────────────────────────────────────────

/// The state of a flashcard's memory schedule.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardState {
    pub stability: f64,
    pub difficulty: f64,
    pub reps: u32,
    pub lapses: u32,
    pub last_review: DateTime<Utc>,
}

impl CardState {
    /// Create a brand-new card state (never reviewed).
    pub fn new() -> Self {
        Self {
            stability: 0.0,
            difficulty: 0.0,
            reps: 0,
            lapses: 0,
            last_review: Utc::now(),
        }
    }
}

/// Full flashcard data stored in the workspace.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlashcardData {
    pub front: String,
    pub back: String,
    pub note_id: String,
    pub state: CardState,
}

// ── FSRS Algorithm ──────────────────────────────────────────────────

/// Clamp a value between min and max.
fn clamp(val: f64, min: f64, max: f64) -> f64 {
    val.max(min).min(max)
}

/// Initial stability for a given rating (1-4).
fn initial_stability(rating: u8) -> f64 {
    W[(rating - 1) as usize].max(0.1)
}

/// Initial difficulty for a given rating (1-4).
fn initial_difficulty(rating: u8) -> f64 {
    let d = W[4] - (W[5] * (rating as f64 - 1.0)).exp() + 1.0;
    clamp(d, 1.0, 10.0)
}

/// Retrievability: probability of recall after `elapsed` days with given stability.
fn retrievability(elapsed_days: f64, stability: f64) -> f64 {
    (1.0 + FACTOR * elapsed_days / stability).powf(DECAY)
}

/// Compute the interval (days) for a desired retention and stability.
fn interval_for_retention(stability: f64) -> u32 {
    // R_target = DESIRED_RETENTION = 0.9
    // interval = stability * (ln(R_target) / ln(0.9))
    let interval = stability * (DESIRED_RETENTION.ln() / 0.9_f64.ln());
    interval.round().max(1.0) as u32
}

/// Update difficulty after a review.
fn next_difficulty(d: f64, rating: u8) -> f64 {
    let d0_ref = initial_difficulty(3); // Mean reversion target (Good)
    let d_new = d - W[6] * (rating as f64 - 3.0);
    let d_final = W[7] * d0_ref + (1.0 - W[7]) * d_new;
    clamp(d_final, 1.0, 10.0)
}

/// Stability after a successful recall (rating >= 2).
fn next_recall_stability(d: f64, s: f64, r: f64, rating: u8) -> f64 {
    let hard_penalty = if rating == 2 { W[15] } else { 1.0 };
    let easy_bonus = if rating == 4 { W[16] } else { 1.0 };

    s * (W[8].exp()
        * (11.0 - d)
        * s.powf(-W[9])
        * ((W[10] * (1.0 - r)).exp() - 1.0)
        * hard_penalty
        * easy_bonus
        + 1.0)
}

/// Stability after a lapse (rating == 1, forgotten).
fn next_forget_stability(d: f64, s: f64, r: f64) -> f64 {
    let s_new = W[11] * d.powf(-W[12]) * ((s + 1.0).powf(W[13]) - 1.0) * (W[14] * (1.0 - r)).exp();
    s_new.max(0.1).min(s) // Lapse stability must not exceed previous
}

/// Schedule the next review for a card.
///
/// - `state`: current card state.
/// - `rating`: 1 (Again), 2 (Hard), 3 (Good), 4 (Easy).
///
/// Returns `(new_state, days_until_next_review)`.
pub fn next_interval(state: &CardState, rating: u8) -> (CardState, u32) {
    let rating = rating.clamp(1, 4);
    let now = Utc::now();

    if state.reps == 0 {
        // ── First review ────────────────────────────────────────
        let s = initial_stability(rating);
        let d = initial_difficulty(rating);
        let interval = interval_for_retention(s);

        let new_state = CardState {
            stability: s,
            difficulty: d,
            reps: 1,
            lapses: if rating == 1 { 1 } else { 0 },
            last_review: now,
        };

        (new_state, interval)
    } else {
        // ── Subsequent reviews ──────────────────────────────────
        let elapsed = (now - state.last_review).num_seconds().max(0) as f64 / 86400.0;
        let r = retrievability(elapsed, state.stability);
        let d = next_difficulty(state.difficulty, rating);

        let (s, lapses) = if rating == 1 {
            // Lapse
            let s = next_forget_stability(state.difficulty, state.stability, r);
            (s, state.lapses + 1)
        } else {
            // Recall
            let s = next_recall_stability(state.difficulty, state.stability, r, rating);
            (s, state.lapses)
        };

        let interval = interval_for_retention(s);

        let new_state = CardState {
            stability: s,
            difficulty: d,
            reps: state.reps + 1,
            lapses,
            last_review: now,
        };

        (new_state, interval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_review_good() {
        let state = CardState::new();
        let (new_state, days) = next_interval(&state, 3);
        assert_eq!(new_state.reps, 1);
        assert_eq!(new_state.lapses, 0);
        assert!(new_state.stability > 0.0);
        assert!(days >= 1);
    }

    #[test]
    fn first_review_again_counts_lapse() {
        let state = CardState::new();
        let (new_state, _days) = next_interval(&state, 1);
        assert_eq!(new_state.lapses, 1);
    }

    #[test]
    fn rating_clamped() {
        let state = CardState::new();
        let (s1, _) = next_interval(&state, 0); // Clamped to 1
        assert_eq!(s1.lapses, 1);
        let (s2, _) = next_interval(&state, 255); // Clamped to 4
        assert_eq!(s2.lapses, 0);
    }
}
