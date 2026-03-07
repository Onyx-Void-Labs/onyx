// ─── Onyx Core — Query Engine ──────────────────────────────────────
// Filters and sorts notes within a Void based on SectionConfig.
// ───────────────────────────────────────────────────────────────────

use chrono::Utc;

use crate::document::OnyxWorkspace;
use crate::model::{NodeType, SectionConfig};

/// Advanced query types for the workspace.
pub enum AdvancedQuery {
    /// Find notes in a void that have a specific property value.
    NotesByProperty {
        void: String,
        key: String,
        value: String,
    },
    /// Find flashcards due for review, up to `limit`.
    DueFlashcards { limit: usize },
    /// Find notes that link to a given note (backlinks).
    Backlinks { note_id: String },
}

/// Process a query against the workspace using the given section config.
/// Returns a sorted list of note IDs matching the filter criteria.
pub fn query_notes(config: &SectionConfig, workspace: &OnyxWorkspace) -> Vec<String> {
    let nodes = workspace.get_tree_nodes();

    let mut matched: Vec<(String, String)> = Vec::new();

    for (node, _depth) in &nodes {
        // Only consider Note nodes
        if node.node_type != NodeType::Note {
            continue;
        }

        // Must belong to a void (membership check)
        let void_id = match workspace.parent_void_of(&node.id) {
            Some(v) => v,
            None => continue,
        };

        // Apply property filter if specified
        if let (Some(ref prop), Some(ref val)) = (&config.filter_prop, &config.filter_val) {
            let values = workspace.get_note_values(&node.id, &void_id);
            match values.get(prop.as_str()) {
                Some(v) if v == val => {} // matches — keep
                _ => continue,            // missing or mismatch — skip
            }
        }

        // Collect the sort key (defaults to title if sort field missing)
        let sort_key = node.title.clone();

        matched.push((node.id.clone(), sort_key));
    }

    // Sort by the sort key (alphabetical)
    matched.sort_by(|a, b| a.1.cmp(&b.1));

    matched.into_iter().map(|(id, _)| id).collect()
}

/// Query notes filtered to a specific void, with optional property filter
/// and custom sort field.
pub fn query_notes_sorted(
    workspace: &OnyxWorkspace,
    void_id: &str,
    filter_prop: Option<&str>,
    filter_val: Option<&str>,
    sort_by: Option<&str>,
) -> Vec<String> {
    let nodes = workspace.get_tree_nodes();

    let mut matched: Vec<(String, String)> = Vec::new();

    for (node, _depth) in &nodes {
        if node.node_type != NodeType::Note {
            continue;
        }

        // Membership: note must belong to the specified void
        let parent = match workspace.parent_void_of(&node.id) {
            Some(v) => v,
            None => continue,
        };
        if parent != void_id {
            continue;
        }

        let values = workspace.get_note_values(&node.id, void_id);

        // Apply property filter
        if let (Some(prop), Some(val)) = (filter_prop, filter_val) {
            match values.get(prop) {
                Some(v) if v == val => {}
                _ => continue,
            }
        }

        // Sort key: use specified property value, fall back to title
        let sort_key = sort_by
            .and_then(|field| values.get(field).cloned())
            .unwrap_or_else(|| node.title.clone());

        matched.push((node.id.clone(), sort_key));
    }

    matched.sort_by(|a, b| a.1.cmp(&b.1));
    matched.into_iter().map(|(id, _)| id).collect()
}

/// Execute an advanced query against the workspace.
pub fn execute_advanced_query(workspace: &OnyxWorkspace, query: AdvancedQuery) -> Vec<String> {
    match query {
        AdvancedQuery::NotesByProperty { void, key, value } => {
            query_notes_sorted(workspace, &void, Some(&key), Some(&value), None)
        }
        AdvancedQuery::DueFlashcards { limit } => {
            let now = Utc::now();
            let mut due: Vec<String> = workspace
                .all_flashcard_ids()
                .into_iter()
                .filter(|card_id| {
                    workspace
                        .get_flashcard(card_id)
                        .map(|card| {
                            let days = card.state.stability * 0.9_f64.ln() / (-0.5_f64).ln();
                            let due_date =
                                card.state.last_review + chrono::Duration::days(days.ceil() as i64);
                            due_date <= now
                        })
                        .unwrap_or(false)
                })
                .collect();
            due.truncate(limit);
            due
        }
        AdvancedQuery::Backlinks { note_id } => {
            let all_notes = workspace.all_note_ids();
            let mut backlinks = Vec::new();
            for nid in &all_notes {
                let blocks = workspace.get_note_blocks(nid);
                for block in &blocks {
                    if let crate::blocks::BlockType::Link { target_id } = &block.kind {
                        if target_id == &note_id {
                            backlinks.push(nid.clone());
                            break;
                        }
                    }
                }
            }
            backlinks
        }
    }
}
