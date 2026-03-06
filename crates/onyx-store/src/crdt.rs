// ─── CRDT Document ─────────────────────────────────────────────────
// Wraps Loro's LoroDoc for conflict-free collaborative editing.
// Each document has an associated CRDT that tracks text changes.
//
// The delta export is ZSTD-compressed for network transport (Phase 2).
// ────────────────────────────────────────────────────────────────────

use loro::LoroDoc;
use onyx_core::core_state::{LaneDocSnapshot, RowSnapshot, SlotKind, SlotSnapshot};
use onyx_core::error::{OnyxError, OnyxResult};
use tracing::trace;

/// A CRDT-backed document state.
pub struct CrdtDoc {
    doc: LoroDoc,
    /// Number of uncommitted ops since last commit/compact.
    ops_since_commit: std::cell::Cell<u32>,
    /// Saved version vector from before the last batch of edits.
    /// Used for incremental delta export.
    pre_edit_vv: std::cell::RefCell<Option<loro::VersionVector>>,
}

/// How many ops before we compact the Loro history.
const COMPACT_THRESHOLD: u32 = 500;

impl CrdtDoc {
    /// Create a new empty CRDT document.
    pub fn new() -> Self {
        Self {
            doc: LoroDoc::new(),
            ops_since_commit: std::cell::Cell::new(0),
            pre_edit_vv: std::cell::RefCell::new(None),
        }
    }

    /// Commit pending operations and compact history if we've
    /// accumulated enough ops to warrant it.
    ///
    /// Call this periodically (e.g. after every edit) to keep
    /// memory usage bounded. Loro will merge small ops into
    /// larger, more compact internal structures.
    pub fn maybe_compact(&self) {
        let count = self.ops_since_commit.get() + 1;
        self.ops_since_commit.set(count);
        if count >= COMPACT_THRESHOLD {
            self.doc.commit();
            trace!(ops = count, "loro commit (compaction)");
            self.ops_since_commit.set(0);
        }
    }

    /// Force a commit right now (e.g. before export).
    pub fn force_commit(&self) {
        self.doc.commit();
        self.ops_since_commit.set(0);
    }

    /// Snapshot the current version vector *before* a local edit.
    /// Call `export_incremental_delta()` after the edit to get
    /// just the bytes that changed.
    pub fn capture_pre_edit(&self) {
        *self.pre_edit_vv.borrow_mut() = Some(self.doc.oplog_vv());
    }

    /// Export an incremental delta covering only the ops since the
    /// last `capture_pre_edit()` call. Falls back to a full snapshot
    /// if no pre-edit version was captured.
    pub fn export_incremental_delta(&self) -> Vec<u8> {
        self.doc.commit();
        let vv = self.pre_edit_vv.borrow();
        if let Some(ref vv) = *vv {
            match self.export_updates_since(vv) {
                Ok(delta) if !delta.is_empty() => delta,
                _ => self.export_snapshot(),
            }
        } else {
            self.export_snapshot()
        }
    }

    /// Get the text content of the "body" text container.
    pub fn get_text(&self) -> OnyxResult<String> {
        let text = self.doc.get_text("body");
        Ok(text.to_string())
    }

    /// Get text content for a named text container (keyed by node ID).
    pub fn get_text_for(&self, key: &str) -> OnyxResult<String> {
        let text = self.doc.get_text(key);
        Ok(text.to_string())
    }

    /// Insert `s` at character position `pos` in the "body" text.
    pub fn insert(&self, pos: usize, s: &str) -> OnyxResult<()> {
        let text = self.doc.get_text("body");
        text.insert(pos, s)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        trace!(pos, s, "crdt insert");
        self.maybe_compact();
        Ok(())
    }

    /// Insert `s` at character position `pos` in a named text container.
    pub fn insert_for(&self, key: &str, pos: usize, s: &str) -> OnyxResult<()> {
        let text = self.doc.get_text(key);
        text.insert(pos, s)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        trace!(key, pos, s, "crdt insert_for");
        self.maybe_compact();
        Ok(())
    }

    /// Delete `len` characters starting at `pos`.
    pub fn delete(&self, pos: usize, len: usize) -> OnyxResult<()> {
        let text = self.doc.get_text("body");
        text.delete(pos, len)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        trace!(pos, len, "crdt delete");
        self.maybe_compact();
        Ok(())
    }

    /// Delete `len` characters starting at `pos` in a named text container.
    pub fn delete_for(&self, key: &str, pos: usize, len: usize) -> OnyxResult<()> {
        let text = self.doc.get_text(key);
        text.delete(pos, len)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        trace!(key, pos, len, "crdt delete_for");
        self.maybe_compact();
        Ok(())
    }

    /// Export the full state as a binary snapshot.
    pub fn export_snapshot(&self) -> Vec<u8> {
        self.doc
            .export(loro::ExportMode::Snapshot)
            .unwrap_or_default()
    }

    /// Get the current version vector (for incremental delta sync).
    ///
    /// Use this before an edit, then after the edit call
    /// `export_updates_since()` with the pre-edit version vector
    /// to get just the delta — vastly smaller than a full snapshot.
    pub fn version_vector(&self) -> loro::VersionVector {
        self.doc.oplog_vv()
    }

    /// Export only updates since the given version vector.
    ///
    /// This is the heart of efficient P2P sync: on each keystroke
    /// we export ~50 bytes of delta instead of cloning the whole doc.
    pub fn export_updates_since(&self, vv: &loro::VersionVector) -> OnyxResult<Vec<u8>> {
        use std::borrow::Cow;
        self.doc
            .export(loro::ExportMode::Updates {
                from: Cow::Owned(vv.clone()),
            })
            .map_err(|e| OnyxError::Crdt(e.to_string()))
    }

    /// Export a compressed snapshot (ZSTD level 3).
    pub fn export_compressed(&self) -> OnyxResult<Vec<u8>> {
        let raw = self.export_snapshot();
        let compressed = zstd::encode_all(raw.as_slice(), 3)
            .map_err(|e| OnyxError::Serialization(e.to_string()))?;
        trace!(
            raw_bytes = raw.len(),
            compressed_bytes = compressed.len(),
            "crdt snapshot compressed"
        );
        Ok(compressed)
    }

    /// Import a snapshot (raw, uncompressed).
    pub fn import_snapshot(&self, data: &[u8]) -> OnyxResult<()> {
        self.doc
            .import(data)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        Ok(())
    }

    /// Import a ZSTD-compressed snapshot.
    pub fn import_compressed(&self, data: &[u8]) -> OnyxResult<()> {
        let raw = zstd::decode_all(data).map_err(|e| OnyxError::Serialization(e.to_string()))?;
        self.import_snapshot(&raw)
    }

    // ── Lane & Slot System: LoroTree Document Operations ────────
    //
    // The document is a TREE, not a list.
    // Tree nodes = Row | Slot.
    // Metadata lives in LoroMap attached to each tree node.
    // Text content lives in LoroText containers keyed by slot ID.
    // Atomic moves via LoroTree::mov() — NEVER delete-then-insert.
    // Ghost Box: empty nodes get collapsed: true, never deleted.
    // ────────────────────────────────────────────────────────────

    /// Get the LoroTree for a VoidNode's document structure.
    fn doc_tree(&self, node_id: &str) -> loro::LoroTree {
        self.doc.get_tree(format!("dtree_{}", node_id))
    }

    /// Generate a unique text container key for a slot.
    fn slot_text_key(node_id: &str, tree_id: loro::TreeID) -> String {
        format!("dtext_{}_{}_{}", node_id, tree_id.peer, tree_id.counter)
    }

    /// Ensure a VoidNode has a document tree. If missing, creates
    /// one Row with one Slot (the default initial state).
    pub fn ensure_doc_tree(&self, node_id: &str) -> OnyxResult<()> {
        let tree = self.doc_tree(node_id);
        let roots = tree.roots();
        if roots.is_empty() {
            let row_id = self.create_row_for(node_id)?;
            self.create_slot_for(node_id, row_id)?;
            self.doc.commit();
        }
        Ok(())
    }

    /// Create a new Row at document root level.
    pub fn create_row_for(&self, node_id: &str) -> OnyxResult<loro::TreeID> {
        let tree = self.doc_tree(node_id);
        let row_id = tree
            .create(loro::TreeParentId::Root)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        let meta = tree
            .get_meta(row_id)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        meta.insert("kind", "row")
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        meta.insert("collapsed", false)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        trace!(%node_id, "created row in doc tree");
        self.maybe_compact();
        Ok(row_id)
    }

    /// Create a new Slot under the given Row.
    /// Returns (TreeID, text_key) — the text_key names the LoroText container.
    pub fn create_slot_for(
        &self,
        node_id: &str,
        row_id: loro::TreeID,
    ) -> OnyxResult<(loro::TreeID, String)> {
        let tree = self.doc_tree(node_id);
        let slot_id = tree
            .create(loro::TreeParentId::Node(row_id))
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        let text_key = Self::slot_text_key(node_id, slot_id);
        let meta = tree
            .get_meta(slot_id)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        meta.insert("kind", "slot")
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        meta.insert("widget_type", "text")
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        meta.insert("width_ratio", 1.0_f64)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        meta.insert("collapsed", false)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        meta.insert("text_key", text_key.as_str())
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        trace!(%node_id, %text_key, "created slot in doc tree");
        self.maybe_compact();
        Ok((slot_id, text_key))
    }

    /// Move a Slot to a different Row. ATOMIC — never delete-then-insert.
    pub fn move_slot(
        &self,
        node_id: &str,
        slot_id: loro::TreeID,
        target_row: loro::TreeID,
    ) -> OnyxResult<()> {
        let tree = self.doc_tree(node_id);
        tree.mov(slot_id, loro::TreeParentId::Node(target_row))
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        trace!(%node_id, "moved slot to new row (atomic LoroTree::mov)");
        self.maybe_compact();
        Ok(())
    }

    /// Ghost Box: mark a tree node as collapsed (render-layer only).
    /// NEVER structurally delete nodes reactively.
    pub fn collapse_tree_node(&self, node_id: &str, tree_node_id: loro::TreeID) -> OnyxResult<()> {
        let tree = self.doc_tree(node_id);
        let meta = tree
            .get_meta(tree_node_id)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        meta.insert("collapsed", true)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        trace!(%node_id, "collapsed tree node (Ghost Box)");
        self.maybe_compact();
        Ok(())
    }

    /// Split a Slot: create a new sibling Slot in the same Row,
    /// dividing the parent width 50/50.
    pub fn split_slot(
        &self,
        node_id: &str,
        slot_id: loro::TreeID,
    ) -> OnyxResult<(loro::TreeID, String)> {
        let tree = self.doc_tree(node_id);
        // Find the parent row
        let parent = tree.parent(slot_id);
        let row_id = match parent {
            Some(loro::TreeParentId::Node(rid)) => rid,
            _ => return Err(OnyxError::Crdt("slot has no parent row".into())),
        };
        // Set the original slot to 50% width
        let orig_meta = tree
            .get_meta(slot_id)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        orig_meta
            .insert("width_ratio", 0.5_f64)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        // Create new sibling slot at 50% width
        let (new_id, new_key) = self.create_slot_for(node_id, row_id)?;
        let new_meta = tree
            .get_meta(new_id)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        new_meta
            .insert("width_ratio", 0.5_f64)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        trace!(%node_id, "split slot 50/50");
        Ok((new_id, new_key))
    }

    /// Insert text at position `pos` in the slot's LoroText container.
    pub fn insert_slot_text(&self, text_key: &str, pos: usize, s: &str) -> OnyxResult<()> {
        let text = self.doc.get_text(text_key);
        text.insert(pos, s)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        trace!(%text_key, pos, s, "slot text insert");
        self.maybe_compact();
        Ok(())
    }

    /// Delete `len` characters at position `pos` in the slot's LoroText.
    pub fn delete_slot_text(&self, text_key: &str, pos: usize, len: usize) -> OnyxResult<()> {
        let text = self.doc.get_text(text_key);
        text.delete(pos, len)
            .map_err(|e| OnyxError::Crdt(e.to_string()))?;
        trace!(%text_key, pos, len, "slot text delete");
        self.maybe_compact();
        Ok(())
    }

    /// Get the text content of a slot's LoroText container.
    pub fn get_slot_text(&self, text_key: &str) -> OnyxResult<String> {
        let text = self.doc.get_text(text_key);
        Ok(text.to_string())
    }

    /// Build a complete snapshot of a VoidNode's Lane Document.
    /// Walks the LoroTree and reads all metadata + text content.
    pub fn snapshot_lane_doc(&self, node_id: &str) -> OnyxResult<LaneDocSnapshot> {
        let tree = self.doc_tree(node_id);
        let root_nodes = tree.roots();
        let mut rows = Vec::new();

        for row_id in root_nodes {
            let row_meta = match tree.get_meta(row_id) {
                Ok(m) => m,
                Err(_) => continue,
            };
            // Read collapsed flag
            let collapsed = Self::read_meta_bool(&row_meta, "collapsed");

            // Get children (slots)
            let slot_ids = tree
                .children(loro::TreeParentId::Node(row_id))
                .unwrap_or_default();
            let mut slots = Vec::new();

            for slot_id in slot_ids {
                let slot_meta = match tree.get_meta(slot_id) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let slot_collapsed = Self::read_meta_bool(&slot_meta, "collapsed");
                let text_key = Self::read_meta_string(&slot_meta, "text_key");
                let width_ratio = Self::read_meta_f64(&slot_meta, "width_ratio") as f32;
                let widget_type = Self::read_meta_string(&slot_meta, "widget_type");

                let slot_kind = match widget_type.as_str() {
                    "text" => SlotKind::Text,
                    "node_ref" => SlotKind::NodeReference {
                        node_id: Self::read_meta_string(&slot_meta, "ref_node_id"),
                    },
                    other => SlotKind::Widget {
                        widget_type: other.to_string(),
                    },
                };

                let text_content = if !text_key.is_empty() {
                    self.get_slot_text(&text_key).unwrap_or_default()
                } else {
                    String::new()
                };

                slots.push(SlotSnapshot {
                    id: format!("{}_{}", slot_id.peer, slot_id.counter),
                    text_key,
                    width_ratio,
                    slot_kind,
                    collapsed: slot_collapsed,
                    text_content,
                });
            }

            rows.push(RowSnapshot {
                id: format!("{}_{}", row_id.peer, row_id.counter),
                collapsed,
                slots,
            });
        }

        Ok(LaneDocSnapshot { rows })
    }

    /// Helper: read a bool from LoroMap metadata.
    fn read_meta_bool(map: &loro::LoroMap, key: &str) -> bool {
        match map.get(key) {
            Some(loro::ValueOrContainer::Value(loro::LoroValue::Bool(b))) => b,
            _ => false,
        }
    }

    /// Helper: read a string from LoroMap metadata.
    fn read_meta_string(map: &loro::LoroMap, key: &str) -> String {
        match map.get(key) {
            Some(loro::ValueOrContainer::Value(loro::LoroValue::String(s))) => s.to_string(),
            _ => String::new(),
        }
    }

    /// Helper: read an f64 from LoroMap metadata.
    fn read_meta_f64(map: &loro::LoroMap, key: &str) -> f64 {
        match map.get(key) {
            Some(loro::ValueOrContainer::Value(loro::LoroValue::Double(d))) => d,
            _ => 1.0,
        }
    }

    /// Resolve a slot string ID back to a TreeID.
    /// Parses "peer_counter" format.
    pub fn parse_slot_id(slot_id_str: &str) -> Option<loro::TreeID> {
        let parts: Vec<&str> = slot_id_str.splitn(2, '_').collect();
        if parts.len() != 2 {
            return None;
        }
        let peer: u64 = parts[0].parse().ok()?;
        let counter: i32 = parts[1].parse().ok()?;
        Some(loro::TreeID::new(peer, counter))
    }

    /// Get all slot text keys in document order (non-collapsed only).
    /// Used by the Backspace interceptor to walk adjacent slots.
    pub fn ordered_slot_keys(&self, node_id: &str) -> Vec<String> {
        let tree = self.doc_tree(node_id);
        let root_nodes = tree.roots();
        let mut keys = Vec::new();

        for row_id in root_nodes {
            if let Ok(row_meta) = tree.get_meta(row_id) {
                if Self::read_meta_bool(&row_meta, "collapsed") {
                    continue;
                }
            }
            let slot_ids = tree
                .children(loro::TreeParentId::Node(row_id))
                .unwrap_or_default();
            for slot_id in slot_ids {
                if let Ok(slot_meta) = tree.get_meta(slot_id) {
                    if Self::read_meta_bool(&slot_meta, "collapsed") {
                        continue;
                    }
                    let text_key = Self::read_meta_string(&slot_meta, "text_key");
                    if !text_key.is_empty() {
                        keys.push(text_key);
                    }
                }
            }
        }
        keys
    }
}

impl Default for CrdtDoc {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_insert_read() {
        let doc = CrdtDoc::new();
        doc.insert(0, "Hello Void").unwrap();
        assert_eq!(doc.get_text().unwrap(), "Hello Void");
    }

    #[test]
    fn delta_sync() {
        let doc1 = CrdtDoc::new();
        let doc2 = CrdtDoc::new();

        // Initial content in doc1
        doc1.insert(0, "Hello ").unwrap();
        let snapshot = doc1.export_snapshot();
        doc2.import_snapshot(&snapshot).unwrap();

        // Now both docs have "Hello "
        let vv = doc1.version_vector();
        doc1.insert(6, "Void").unwrap();

        // Export only the delta
        let delta = doc1.export_updates_since(&vv).unwrap();
        assert!(!delta.is_empty());
        assert!(
            delta.len() < snapshot.len(),
            "delta should be smaller than snapshot"
        );

        // Apply delta to doc2
        doc2.import_snapshot(&delta).unwrap(); // Loro import handles both snapshots and updates
        assert_eq!(doc2.get_text().unwrap(), "Hello Void");
    }

    #[test]
    fn compressed_roundtrip() {
        let doc = CrdtDoc::new();
        doc.insert(0, "compress me").unwrap();

        let compressed = doc.export_compressed().unwrap();
        assert!(!compressed.is_empty());

        let doc2 = CrdtDoc::new();
        doc2.import_compressed(&compressed).unwrap();
        assert_eq!(doc2.get_text().unwrap(), "compress me");
    }
}
