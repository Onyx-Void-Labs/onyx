// ─── Onyx Core — Layout State Persistence (CRDT-backed Grid) ────────

use loro::{LoroDoc, LoroMap, LoroValue, ValueOrContainer};

use crate::grid_layout::{GridRow, Slot};
use anyhow::{Context, Result};

/// Persists GridRow layouts into a LoroMap within a LoroDoc.
/// Each row is keyed by a `row_id` and stored as JSON.
pub struct LayoutState {
    map: LoroMap,
}

impl LayoutState {
    /// Create a new LayoutState backed by the given LoroDoc.
    pub fn new(doc: &LoroDoc) -> Self {
        Self {
            map: doc.get_map("layout"),
        }
    }

    /// Save a grid row to the CRDT map. Serializes the entire GridRow (includes
    /// `collapsed` flag).
    pub fn save_row(&self, doc: &LoroDoc, row_id: &str, row: &GridRow) -> Result<()> {
        let json = serde_json::to_string(row).context("serialize grid row")?;
        self.map.insert(row_id, json).context("insert layout row")?;
        doc.commit();
        Ok(())
    }

    /// Load a grid row from the CRDT map.
    pub fn load_row(&self, row_id: &str) -> Option<GridRow> {
        match self.map.get(row_id) {
            Some(ValueOrContainer::Value(v)) => {
                let s = v.as_string()?;
                serde_json::from_str(s).ok()
            }
            _ => None,
        }
    }

    /// Mark a grid row as collapsed instead of deleting it.  This implements the
    /// "ghost row" behaviour requested by the audit.
    pub fn remove_row(&self, doc: &LoroDoc, row_id: &str) {
        if let Some(mut row) = self.load_row(row_id) {
            row.collapsed = true;
            let _ = self.save_row(doc, row_id, &row);
        }
    }

    /// List all stored row IDs.
    pub fn all_row_ids(&self) -> Vec<String> {
        let deep = self.map.get_deep_value();
        if let LoroValue::Map(map) = &deep {
            map.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_save_load() -> Result<()> {
        let doc = LoroDoc::new();
        let state = LayoutState::new(&doc);
        let row = GridRow {
            slots: vec![
                Slot {
                    col_start: 0,
                    col_span: 6,
                    widget_id: "widget-a".into(),
                },
                Slot {
                    col_start: 6,
                    col_span: 6,
                    widget_id: "widget-b".into(),
                },
            ],
            collapsed: false,
        };
        state.save_row(&doc, "row-1", &row)?;

        if let Some(loaded) = state.load_row("row-1") {
            assert_eq!(loaded.slots.len(), 2);
            assert_eq!(loaded.slots[0].col_span, 6);
            assert_eq!(loaded.slots[0].widget_id, "widget-a");
            assert_eq!(loaded.slots[1].col_span, 6);
            assert_eq!(loaded.slots[1].widget_id, "widget-b");
            assert!(!loaded.collapsed);
        } else {
            panic!("row should exist");
        }
        Ok(())
    }

    #[test]
    fn load_missing_row() {
        let doc = LoroDoc::new();
        let state = LayoutState::new(&doc);
        assert!(state.load_row("nonexistent").is_none());
    }

    #[test]
    fn remove_row() -> Result<()> {
        let doc = LoroDoc::new();
        let state = LayoutState::new(&doc);
        let row = GridRow {
            slots: vec![Slot {
                col_start: 0,
                col_span: 12,
                widget_id: "w".into(),
            }],
            collapsed: false,
        };
        state.save_row(&doc, "r1", &row)?;
        assert!(state.load_row("r1").is_some());

        state.remove_row(&doc, "r1");
        if let Some(loaded) = state.load_row("r1") {
            assert!(loaded.collapsed);
        } else {
            panic!("row should still exist (collapsed)");
        }
        Ok(())
    }

    #[test]
    fn all_row_ids() -> Result<()> {
        let doc = LoroDoc::new();
        let state = LayoutState::new(&doc);
        let row = GridRow {
            slots: vec![Slot {
                col_start: 0,
                col_span: 12,
                widget_id: "w".into(),
            }],
            collapsed: false,
        };
        state.save_row(&doc, "row-a", &row)?;
        state.save_row(&doc, "row-b", &row)?;

        let mut ids = state.all_row_ids();
        ids.sort();
        assert_eq!(ids, vec!["row-a", "row-b"]);
        Ok(())
    }
}
