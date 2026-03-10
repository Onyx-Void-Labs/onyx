use chrono::{DateTime, Utc};
use fsrs::{ItemState, MemoryState, FSRS};
use serde::{Deserialize, Serialize};

pub const DESIRED_RETENTION: f64 = 0.9;

/// Your CardState wrapper.
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CardState {
    pub stability: f64,
    pub difficulty: f64,
    pub reps: u32,
    pub lapses: u32,
    pub last_review: DateTime<Utc>,
}

/// Flashcard data.
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FlashcardData {
    pub front: String,
    pub back: String,
    pub note_id: String,
    pub state: CardState,
}

/// FSRS-6 Scheduler wrapper around the `fsrs` crate.
pub struct Scheduler(FSRS);

impl Default for Scheduler {
    fn default() -> Self {
        // create with default parameters
        Scheduler(FSRS::new(Some(&[])).expect("failed to initialize fsrs"))
    }
}

impl Scheduler {
    /// Review a card and compute the next interval.
    ///
    /// `rating` should be in the 1‑4 range (again/hard/good/easy).
    pub fn next_interval(&mut self, state: &CardState, rating: u8) -> (CardState, u32) {
        let elapsed: u32 = (Utc::now() - state.last_review).num_days() as u32;

        let prev = MemoryState {
            stability: state.stability as f32,
            difficulty: state.difficulty as f32,
        };

        let next = self
            .0
            .next_states(Some(prev), DESIRED_RETENTION as f32, elapsed)
            .expect("fsrs.next_states failed");

        let item: ItemState = match rating {
            1 => next.again,
            2 => next.hard,
            3 => next.good,
            4 => next.easy,
            _ => next.again,
        };

        let mem = item.memory;
        let interval_f = item.interval;
        let interval = interval_f.round() as u32;

        let new_state = CardState {
            stability: mem.stability as f64,
            difficulty: mem.difficulty as f64,
            reps: state.reps + 1,
            lapses: state.lapses + if rating == 1 { 1 } else { 0 },
            last_review: Utc::now(),
        };

        (new_state, interval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduler_basic() {
        let mut sched = Scheduler::default();
        let state = CardState::default();
        let (new_state, _days) = sched.next_interval(&state, 3);
        assert_eq!(new_state.reps, 1);
    }
}
