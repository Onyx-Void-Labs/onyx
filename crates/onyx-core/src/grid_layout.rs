// ─── Onyx Core — Grid Engine (12-Column Layout System) ──────────────

use serde::{Deserialize, Serialize};

/// A slot within a grid row, spanning some number of columns.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub col_start: u8,
    pub col_span: u8,
    pub widget_id: String,
}

/// A single row in the 12-column grid.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GridRow {
    pub slots: Vec<Slot>,
    pub collapsed: bool,
}

/// A rectangle representing the resolved position of a slot.
#[derive(Clone, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

const TOTAL_COLUMNS: u8 = 12;

/// Resolve a grid row into absolute pixel rectangles.
///
/// Each slot's width is proportional to its `col_span` out of 12.
/// All rects share `y = 0.0` and a default height equal to the container width / 4.
/// Slots that would exceed 12 columns are clamped.
pub fn resolve_layout(container_w: f32, grid: &GridRow) -> Vec<Rect> {
    if grid.collapsed {
        return Vec::new();
    }
    let col_w = container_w / TOTAL_COLUMNS as f32;
    let row_h = container_w / 4.0; // default aspect ratio
    let mut rects = Vec::new();

    for slot in &grid.slots {
        // ensure the slot fits within bounds
        let start = slot.col_start.min(TOTAL_COLUMNS);
        let span = slot.col_span.min(TOTAL_COLUMNS.saturating_sub(start));
        if span == 0 {
            continue;
        }
        rects.push(Rect {
            x: start as f32 * col_w,
            y: 0.0,
            width: span as f32 * col_w,
            height: row_h,
        });
    }

    rects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_width_single_slot() {
        let grid = GridRow {
            slots: vec![Slot {
                col_start: 0,
                col_span: 12,
                widget_id: "w1".into(),
            }],
            collapsed: false,
        };
        let rects = resolve_layout(1200.0, &grid);
        assert_eq!(rects.len(), 1);
        assert_eq!(
            rects[0],
            Rect {
                x: 0.0,
                y: 0.0,
                width: 1200.0,
                height: 300.0,
            }
        );
    }

    #[test]
    fn two_equal_halves() {
        let grid = GridRow {
            slots: vec![
                Slot {
                    col_start: 0,
                    col_span: 6,
                    widget_id: "a".into(),
                },
                Slot {
                    col_start: 6,
                    col_span: 6,
                    widget_id: "b".into(),
                },
            ],
            collapsed: false,
        };
        let rects = resolve_layout(1200.0, &grid);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].x, 0.0);
        assert_eq!(rects[0].width, 600.0);
        assert_eq!(rects[1].x, 600.0);
        assert_eq!(rects[1].width, 600.0);
    }

    #[test]
    fn overflow_clamped() {
        let grid = GridRow {
            slots: vec![
                Slot {
                    col_start: 0,
                    col_span: 8,
                    widget_id: "a".into(),
                },
                Slot {
                    col_start: 8,
                    col_span: 8,
                    widget_id: "b".into(),
                }, // exceeds 12
            ],
            collapsed: false,
        };
        let rects = resolve_layout(1200.0, &grid);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[1].width, 400.0); // clamped to 4 cols
    }

    #[test]
    fn three_column_layout() {
        let grid = GridRow {
            slots: vec![
                Slot {
                    col_start: 0,
                    col_span: 4,
                    widget_id: "a".into(),
                },
                Slot {
                    col_start: 4,
                    col_span: 4,
                    widget_id: "b".into(),
                },
                Slot {
                    col_start: 8,
                    col_span: 4,
                    widget_id: "c".into(),
                },
            ],
            collapsed: false,
        };
        let rects = resolve_layout(1200.0, &grid);
        assert_eq!(rects.len(), 3);
        for r in &rects {
            assert_eq!(r.width, 400.0);
        }
    }

    #[test]
    fn empty_grid() {
        let grid = GridRow {
            slots: vec![],
            collapsed: false,
        };
        let rects = resolve_layout(1200.0, &grid);
        assert!(rects.is_empty());
    }

    #[test]
    fn collapsed_row_skips_layout() {
        let grid = GridRow {
            slots: vec![Slot {
                col_start: 0,
                col_span: 12,
                widget_id: "w".into(),
            }],
            collapsed: true,
        };
        let rects = resolve_layout(1200.0, &grid);
        assert!(rects.is_empty());
    }
}
