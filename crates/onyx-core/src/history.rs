// ─── Onyx Core — Undo / Redo History Stack ─────────────────────────
// Tracks LoroDoc snapshots for undo/redo operations.
// ───────────────────────────────────────────────────────────────────

use loro::LoroDoc;

/// Maximum number of undo snapshots to keep.
const MAX_HISTORY: usize = 100;

/// Manages undo/redo via LoroDoc binary snapshots.
pub struct HistoryStack {
    undo_stack: Vec<Vec<u8>>,
    redo_stack: Vec<Vec<u8>>,
}

impl HistoryStack {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Push the current document state as a snapshot. Clears the redo stack.
    pub fn push_snapshot(&mut self, doc: &LoroDoc) -> anyhow::Result<()> {
        let snapshot = doc.export(loro::ExportMode::Snapshot)?;
        self.undo_stack.push(snapshot);

        // Trim to max history size
        if self.undo_stack.len() > MAX_HISTORY {
            self.undo_stack.remove(0);
        }

        // Any new action invalidates the redo stack
        self.redo_stack.clear();
        Ok(())
    }

    /// Undo: pop the last snapshot and return a restored LoroDoc.
    /// The current state should be pushed to redo before calling this.
    pub fn undo(&mut self, current_doc: &LoroDoc) -> anyhow::Result<Option<LoroDoc>> {
        let snapshot = match self.undo_stack.pop() {
            Some(s) => s,
            None => return Ok(None),
        };

        // Save current state to redo stack
        let current_snapshot = current_doc.export(loro::ExportMode::Snapshot)?;
        self.redo_stack.push(current_snapshot);

        let restored = LoroDoc::new();
        restored.import(&snapshot)?;
        Ok(Some(restored))
    }

    /// Redo: pop from redo stack and return a restored LoroDoc.
    pub fn redo(&mut self, current_doc: &LoroDoc) -> anyhow::Result<Option<LoroDoc>> {
        let snapshot = match self.redo_stack.pop() {
            Some(s) => s,
            None => return Ok(None),
        };

        // Save current state to undo stack
        let current_snapshot = current_doc.export(loro::ExportMode::Snapshot)?;
        self.undo_stack.push(current_snapshot);

        let restored = LoroDoc::new();
        restored.import(&snapshot)?;
        Ok(Some(restored))
    }

    /// Number of undo steps available.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of redo steps available.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Whether undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Whether redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loro::LoroValue;

    #[test]
    fn undo_redo_cycle() -> anyhow::Result<()> {
        let mut history = HistoryStack::new();

        // Create initial state
        let doc = LoroDoc::new();
        let map = doc.get_map("test");
        map.insert("key", "value1")?;
        doc.commit();

        // Push snapshot
        history.push_snapshot(&doc)?;

        // Modify document
        map.insert("key", "value2")?;
        doc.commit();

        // Undo should restore previous state
        assert!(history.can_undo());
        if let Some(restored) = history.undo(&doc)? {
            let restored_map = restored.get_map("test");
            let val = restored_map.get_deep_value();
            if let LoroValue::Map(obj) = val {
                if let Some(v) = obj.get("key") {
                    if let LoroValue::String(s) = v {
                        assert_eq!(s.as_str(), "value1");
                    }
                }
            }
        } else {
            panic!("undo returned none");
        }

        // Redo should restore the state we undid from
        assert!(history.can_redo());
        Ok(())
    }
}
