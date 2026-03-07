// ─── Onyx Core — Layout State Persistence (CRDT-backed Grid) ────────

use loro::{LoroDoc, LoroMap, LoroValue, ValueOrContainer};

use crate::grid_layout::{GridRow, Slot};

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

    /// Save a grid row to the CRDT map. Serializes slots as JSON.
    pub fn save_row(&self, doc: &LoroDoc, row_id: &str, row: &GridRow) {
        let slots_json: Vec<serde_json::Value> = row
            .slots
            .iter()
            .map(|s| {
                serde_json::json!({
                    "col_span": s.col_span,
                    "widget_id": s.widget_id,
                })
            })
            .collect();
        let json = serde_json::to_string(&slots_json).expect("serialize grid row");
        self.map.insert(row_id, json).expect("insert layout row");
        doc.commit();
    }

    /// Load a grid row from the CRDT map.
    pub fn load_row(&self, row_id: &str) -> Option<GridRow> {
        match self.map.get(row_id) {
            Some(ValueOrContainer::Value(v)) => {
                let s = v.as_string()?;
                let arr: Vec<serde_json::Value> = serde_json::from_str(s).ok()?;
                let slots = arr
                    .iter()
                    .filter_map(|item| {
                        let col_span = item.get("col_span")?.as_u64()? as u8;
                        let widget_id = item.get("widget_id")?.as_str()?.to_string();
                        Some(Slot { col_span, widget_id })
                    })
                    .collect();
                Some(GridRow { slots })
            }
            _ => None,
        }
    }

    /// Remove a grid row from the CRDT map.
    pub fn remove_row(&self, doc: &LoroDoc, row_id: &str) {
        self.map.delete(row_id).ok();
        doc.commit();
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
    fn round_trip_save_load() {
        let doc = LoroDoc::new();
        let state = LayoutState::new(&doc);
        let row = GridRow {
            slots: vec![
                Slot { col_span: 6, widget_id: "widget-a".into() },
                Slot { col_span: 6, widget_id: "widget-b".into() },
            ],
        };
        state.save_row(&doc, "row-1", &row);

        let loaded = state.load_row("row-1").expect("row should exist");
        assert_eq!(loaded.slots.len(), 2);
        assert_eq!(loaded.slots[0].col_span, 6);
        assert_eq!(loaded.slots[0].widget_id, "widget-a");
        assert_eq!(loaded.slots[1].col_span, 6);
        assert_eq!(loaded.slots[1].widget_id, "widget-b");
    }

    #[test]
    fn load_missing_row() {
        let doc = LoroDoc::new();
        let state = LayoutState::new(&doc);
        assert!(state.load_row("nonexistent").is_none());
    }

    #[test]
    fn remove_row() {
        let doc = LoroDoc::new();
        let state = LayoutState::new(&doc);
        let row = GridRow {
            slots: vec![Slot { col_span: 12, widget_id: "w".into() }],
        };
        state.save_row(&doc, "r1", &row);
        assert!(state.load_row("r1").is_some());

        state.remove_row(&doc, "r1");
        assert!(state.load_row("r1").is_none());
    }

    #[test]
    fn all_row_ids() {
        let doc = LoroDoc::new();
        let state = LayoutState::new(&doc);
        let row = GridRow {
            slots: vec![Slot { col_span: 12, widget_id: "w".into() }],
        };
        state.save_row(&doc, "row-a", &row);
        state.save_row(&doc, "row-b", &row);

        let mut ids = state.all_row_ids();
        ids.sort();
        assert_eq!(ids, vec!["row-a", "row-b"]);
    }
}
