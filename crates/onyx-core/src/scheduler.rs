// ─── Onyx Core — Notification Scheduler ─────────────────────────────

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::document::OnyxWorkspace;
use crate::fsrs::FlashcardData;
use loro::LoroValue;

/// A notification alert for an upcoming due item.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Notification {
    /// Human-readable description.
    pub message: String,
    /// When the item is due.
    pub due_at: DateTime<Utc>,
    /// The related note ID (if applicable).
    pub note_id: String,
    /// Source: "flashcard" or "property".
    pub source: String,
}

/// Scan the workspace for all due flashcards and notes with due-date properties,
/// and return a list of notifications.
pub fn get_due_notifications(ws: &OnyxWorkspace) -> Vec<Notification> {
    let now = Utc::now();
    let mut notifications = Vec::new();

    // ── Scan flashcards ─────────────────────────────────────────
    let deep = ws.flashcards.get_deep_value();
    if let LoroValue::Map(map) = &deep {
        for (_card_id, val) in map.iter() {
            if let LoroValue::String(json) = val {
                if let Ok(card) = serde_json::from_str::<FlashcardData>(json) {
                    let due = card.state.last_review;
                    let stability_days = card.state.stability;
                    // convert stability_days*86400 to i64 with bounds checking
                    let secs_f = (stability_days * 86400.0).round();
                    let secs_i64 = if secs_f < (i64::MIN as f64) {
                        i64::MIN
                    } else if secs_f > (i64::MAX as f64) {
                        i64::MAX
                    } else {
                        secs_f as i64
                    };
                    let next_due = due + Duration::seconds(secs_i64);

                    if next_due <= now + Duration::hours(24) {
                        notifications.push(Notification {
                            message: format!("Flashcard due: {}", card.front),
                            due_at: next_due,
                            note_id: card.note_id.clone(),
                            source: "flashcard".to_string(),
                        });
                    }
                }
            }
        }
    }

    // ── Scan note properties for due dates ──────────────────────
    for note_id in ws.all_note_ids() {
        if let Some(void_id) = ws.parent_void_of(&note_id) {
            let values = ws.get_note_values(&note_id, &void_id);
            // Check for any property named "Due Date" or "due_date"
            for (key, value) in &values {
                let key_lower = key.to_lowercase();
                if key_lower.contains("due") && key_lower.contains("date") {
                    if let Ok(due) = value.parse::<DateTime<Utc>>() {
                        if due <= now + Duration::hours(24) {
                            let title = ws.node_title(&note_id).unwrap_or_default();
                            notifications.push(Notification {
                                message: format!("Due: {}", title),
                                due_at: due,
                                note_id: note_id.clone(),
                                source: "property".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Sort by due date (most urgent first)
    notifications.sort_by_key(|n| n.due_at);
    notifications
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fsrs::{CardState, FlashcardData};

    #[test]
    fn detects_due_flashcards() -> anyhow::Result<()> {
        let mut ws = OnyxWorkspace::new();
        let void_id = ws.create_void(None, "Test Void")?;
        let note_id = ws.create_note(&void_id, "Test Note")?;

        // Create a flashcard that is already due (last_review in the past, low stability)
        let card = FlashcardData {
            front: "What is Rust?".to_string(),
            back: "A systems language.".to_string(),
            note_id: note_id.clone(),
            state: CardState {
                stability: 0.001, // Very small stability → due immediately
                difficulty: 5.0,
                reps: 1,
                lapses: 0,
                last_review: Utc::now() - Duration::days(1),
            },
        };
        ws.set_flashcard("card1", &card)?;

        let notifications = get_due_notifications(&ws);
        assert!(!notifications.is_empty());
        assert_eq!(notifications[0].source, "flashcard");
        assert!(notifications[0].message.contains("What is Rust?"));
        Ok(())
    }

    #[test]
    fn empty_workspace_no_notifications() -> anyhow::Result<()> {
        let ws = OnyxWorkspace::new();
        let notifications = get_due_notifications(&ws);
        assert!(notifications.is_empty());
        Ok(())
    }
}
