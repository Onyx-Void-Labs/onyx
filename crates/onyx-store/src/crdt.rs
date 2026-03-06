// ─── CRDT Document ─────────────────────────────────────────────────
// Wraps Loro's LoroDoc for conflict-free collaborative editing.
// Each document has an associated CRDT that tracks text changes.
//
// The delta export is ZSTD-compressed for network transport (Phase 2).
// ────────────────────────────────────────────────────────────────────

use loro::LoroDoc;
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
