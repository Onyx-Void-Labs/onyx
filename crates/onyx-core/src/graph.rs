// ─── Onyx Core — Backlink Engine (Graph Connections) ────────────────

use std::collections::{HashMap, HashSet};

use anyhow::Context;
use loro::{LoroDoc, LoroValue};
use regex::Regex;

/// Bidirectional link index tracking [[NoteID]] references between notes.
pub struct BacklinkIndex {
    /// Forward edges: source -> set of targets it links to
    forward: HashMap<String, HashSet<String>>,
    /// Reverse edges: target -> set of sources that link to it
    reverse: HashMap<String, HashSet<String>>,
}

impl BacklinkIndex {
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    /// Scan `content` for `[[NoteID]]` patterns and update the graph edges
    /// for `source_note_id`. Previous links from this source are cleared first.
    pub fn update_links(&mut self, source_note_id: &str, content: &str) {
        // Remove old forward edges for this source
        if let Some(old_targets) = self.forward.remove(source_note_id) {
            for target in &old_targets {
                if let Some(sources) = self.reverse.get_mut(target) {
                    sources.remove(source_note_id);
                    if sources.is_empty() {
                        self.reverse.remove(target);
                    }
                }
            }
        }

        // Scan for [[NoteID]] references; if regex fails to compile we bail early.
        let link_re = match Regex::new(r"\[\[([^\]]+)\]\]") {
            Ok(r) => r,
            Err(_) => return,
        };
        let mut new_targets = HashSet::new();
        for caps in link_re.captures_iter(content) {
            new_targets.insert(caps[1].to_string());
        }

        // Insert new edges
        for target in &new_targets {
            self.reverse
                .entry(target.clone())
                .or_default()
                .insert(source_note_id.to_string());
        }

        if !new_targets.is_empty() {
            self.forward.insert(source_note_id.to_string(), new_targets);
        }
    }

    /// Return all note IDs that reference `target_note_id`.
    pub fn get_backlinks(&self, target_note_id: &str) -> Vec<String> {
        self.reverse
            .get(target_note_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Return all note IDs that `source_note_id` links to.
    pub fn get_forward_links(&self, source_note_id: &str) -> Vec<String> {
        self.forward
            .get(source_note_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Persist the current graph state into a LoroMap keyed `"graph"` on the doc.
    /// Stores forward edges as JSON: `{ "source_id": ["target1", "target2"] }`.
    pub fn save_to_loro(&self, doc: &LoroDoc) -> anyhow::Result<()> {
        let map = doc.get_map("graph");
        // Clear existing keys by overwriting
        for (source, targets) in &self.forward {
            let targets_vec: Vec<&str> = targets.iter().map(|s| s.as_str()).collect();
            let json = serde_json::to_string(&targets_vec).context("serialize targets")?;
            map.insert(source, json).context("insert graph edge")?;
        }
        doc.commit();
        Ok(())
    }

    /// Load graph state from a LoroMap keyed `"graph"` on the doc.
    pub fn load_from_loro(doc: &LoroDoc) -> Self {
        let map = doc.get_map("graph");
        let deep = map.get_deep_value();
        let mut forward: HashMap<String, HashSet<String>> = HashMap::new();
        let mut reverse: HashMap<String, HashSet<String>> = HashMap::new();

        if let LoroValue::Map(entries) = &deep {
            for (source, val) in entries.iter() {
                if let LoroValue::String(json) = val {
                    if let Ok(targets) = serde_json::from_str::<Vec<String>>(json) {
                        for target in &targets {
                            reverse
                                .entry(target.clone())
                                .or_default()
                                .insert(source.clone());
                        }
                        forward.insert(source.clone(), targets.into_iter().collect());
                    }
                }
            }
        }

        Self { forward, reverse }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_backlinks() {
        let mut idx = BacklinkIndex::new();
        idx.update_links("note-A", "See [[note-B]] and [[note-C]] for details.");
        idx.update_links("note-D", "Related to [[note-B]].");

        let mut backlinks = idx.get_backlinks("note-B");
        backlinks.sort();
        assert_eq!(backlinks, vec!["note-A", "note-D"]);

        assert_eq!(idx.get_backlinks("note-C"), vec!["note-A"]);
    }

    #[test]
    fn update_clears_old_links() {
        let mut idx = BacklinkIndex::new();
        idx.update_links("note-A", "Link to [[note-B]].");
        assert_eq!(idx.get_backlinks("note-B"), vec!["note-A"]);

        // Update note-A to remove the link
        idx.update_links("note-A", "No links here.");
        assert!(idx.get_backlinks("note-B").is_empty());
    }

    #[test]
    fn no_links() {
        let idx = BacklinkIndex::new();
        assert!(idx.get_backlinks("nonexistent").is_empty());
    }

    #[test]
    fn loro_round_trip() -> anyhow::Result<()> {
        let doc = loro::LoroDoc::new();
        let mut idx = BacklinkIndex::new();
        idx.update_links("note-A", "See [[note-B]] and [[note-C]].");
        idx.update_links("note-D", "Ref to [[note-B]].");
        idx.save_to_loro(&doc)?;

        let loaded = BacklinkIndex::load_from_loro(&doc);
        let mut bl = loaded.get_backlinks("note-B");
        bl.sort();
        assert_eq!(bl, vec!["note-A", "note-D"]);
        assert_eq!(loaded.get_forward_links("note-A").len(), 2);
        Ok(())
    }
}
