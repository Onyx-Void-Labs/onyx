// ─── Onyx Core — Query Engine ──────────────────────────────────────
// Filters and sorts notes within a Void based on SectionConfig.
// ───────────────────────────────────────────────────────────────────

use crate::document::OnyxWorkspace;
use crate::model::{NodeType, SectionConfig};

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
