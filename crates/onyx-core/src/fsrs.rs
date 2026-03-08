// ─── Onyx Core — FSRS v4 (Free Spaced Repetition Scheduler) ────────

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// external spaced repetition scheduler
use fsrs;

// ── FSRS Constants ──────────────────────────────────────────────────

pub const DECAY: f64 = -0.5;
/// FSRS-5 spec: FACTOR = 19/81 ensures R(t=S) = 0.9 exactly.
/// Derivation: solve (1 + FACTOR)^DECAY = 0.9 → FACTOR = 0.9^(1/DECAY) - 1 = 19/81
pub const FACTOR: f64 = 19.0 / 81.0;

// FSRS version 6 parameters supplied by the external fsrs crate.
// FSRS-6 introduces 21 tunable weights (w0..w20).
pub const WEIGHTS: [f64; 21] = fsrs::DEFAULT_WEIGHTS; // FSRS-6 21 params

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

impl Default for CardState {
    fn default() -> Self {
        Self::new()
    }
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
pub fn initial_stability(rating: u8) -> f64 {
    WEIGHTS[(rating - 1) as usize].max(0.1)
}

/// Initial difficulty for a given rating (1-4).
fn initial_difficulty(rating: u8) -> f64 {
    let d = WEIGHTS[4] - (WEIGHTS[5] * (rating as f64 - 1.0)).exp() + 1.0;
    clamp(d, 1.0, 10.0)
}

/// Retrievability: probability of recall after `elapsed` days with given stability.
pub fn retrievability(elapsed_days: f64, stability: f64) -> f64 {
    (1.0 + FACTOR * elapsed_days / stability).powf(DECAY)
}

/// Compute the interval (days) for a desired retention and stability.
/// Derived from inverting the R formula:
/// interval = (S / FACTOR) * (R_target^(1/DECAY) - 1)
pub fn interval_for_retention(stability: f64) -> u32 {
    let raw = (stability / FACTOR) * (DESIRED_RETENTION.powf(1.0 / DECAY) - 1.0);
    raw.round().max(1.0) as u32
}

/// Update difficulty after a review.
fn next_difficulty(d: f64, rating: u8) -> f64 {
    let d0_ref = initial_difficulty(3); // Mean reversion target (Good)
    let d_new = d - WEIGHTS[6] * (rating as f64 - 3.0);
    let d_final = WEIGHTS[7] * d0_ref + (1.0 - WEIGHTS[7]) * d_new;
    clamp(d_final, 1.0, 10.0)
}

/// Stability after a successful recall (rating >= 2).
fn next_recall_stability(d: f64, s: f64, r: f64, rating: u8) -> f64 {
    let hard_penalty = if rating == 2 { WEIGHTS[15] } else { 1.0 };
    let easy_bonus = if rating == 4 { WEIGHTS[16] } else { 1.0 };

    s * (WEIGHTS[8].exp()
        * (11.0 - d)
        * s.powf(-WEIGHTS[9])
        * ((WEIGHTS[10] * (1.0 - r)).exp() - 1.0)
        * hard_penalty
        * easy_bonus
        + 1.0)
}

/// Stability after a lapse (rating == 1, forgotten).
pub fn next_forget_stability(d: f64, s: f64, r: f64) -> f64 {
    let s_new = WEIGHTS[11] * d.powf(-WEIGHTS[12]) * ((s + 1.0).powf(WEIGHTS[13]) - 1.0) * (WEIGHTS[14] * (1.0 - r)).exp();
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
