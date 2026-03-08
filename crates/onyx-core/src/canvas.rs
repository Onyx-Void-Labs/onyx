use serde::{Deserialize, Serialize};
use vello::kurbo::{CubicBez, ParamCurve, Point, Rect};

use crate::fsrs::CardState;

/// A single element on the infinite canvas.  Elements are stored in a
/// void-scoped LoroMap and serialized as JSON, allowing them to be
/// synchronized by the workspace just like blocks or flashcards.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanvasElement {
    pub id: String,           // UUID
    pub parent_void: String,  // LoroTree context
    pub geometry: Geometry,   // kurbo shape data
    pub text: Option<String>, // parley text content
    pub color: [f32; 4],
    pub z_index: u32,
    pub neuro_state: Option<NeuroState>, // FSRS/Cloze data
}

/// Geometric primitives that can be hit‑tested and rendered.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum Geometry {
    Rect {
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    },
    Line {
        start: (f64, f64),
        end: (f64, f64),
    },
    Arrow {
        p0: (f64, f64),
        p1: (f64, f64),
        p2: (f64, f64),
        p3: (f64, f64),
    },
    Freehand(Vec<(f64, f64)>), // Stroke clustering
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
#[derive(PartialEq)]
pub struct NeuroState {
    pub is_cloze_mask: bool,
    pub fsrs: CardState,
    pub keywords: Vec<String>, // For Feynman audio match
}

impl CanvasElement {
    /// Basic hit test against the element's geometry.  The caller will
    /// typically transform the input point from screen coordinates into
    /// world space before invoking this method.
    pub fn hit_test(&self, point: Point) -> bool {
        self.geometry.hit_test(point)
    }
}

impl Geometry {
    /// Returns true if the supplied point lies within (or sufficiently
    /// close to) the geometry.  A small tolerance is applied to the
    /// stroked primitives so that thin lines can be clicked more easily.
    pub fn hit_test(&self, pt: Point) -> bool {
        const TOLERANCE: f64 = 2.0;
        match self {
            Geometry::Rect { x0, y0, x1, y1 } => Rect::new(*x0, *y0, *x1, *y1).contains(pt),
            Geometry::Line { start, end } => {
                // compute distance from point to segment manually
                let a = Point::new(start.0, start.1);
                let b = Point::new(end.0, end.1);
                let ab = (b.x - a.x, b.y - a.y);
                let ap = (pt.x - a.x, pt.y - a.y);
                let ab_len2 = ab.0 * ab.0 + ab.1 * ab.1;
                let t = if ab_len2 > 0.0 {
                    ((ap.0 * ab.0 + ap.1 * ab.1) / ab_len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let closest = Point::new(a.x + ab.0 * t, a.y + ab.1 * t);
                ((closest.x - pt.x).powi(2) + (closest.y - pt.y).powi(2)).sqrt() <= TOLERANCE
            }
            Geometry::Arrow { p0, p1, p2, p3 } => {
                let bz = CubicBez::new(
                    Point::new(p0.0, p0.1),
                    Point::new(p1.0, p1.1),
                    Point::new(p2.0, p2.1),
                    Point::new(p3.0, p3.1),
                );
                let mut t = 0.0;
                while t <= 1.0 {
                    let p = bz.eval(t);
                    let dx = p.x - pt.x;
                    let dy = p.y - pt.y;
                    if (dx * dx + dy * dy).sqrt() <= TOLERANCE {
                        return true;
                    }
                    t += 0.05;
                }
                false
            }
            Geometry::Freehand(points) => {
                for window in points.windows(2) {
                    let p0 = Point::new(window[0].0, window[0].1);
                    let p1 = Point::new(window[1].0, window[1].1);
                    // line variable not needed for our manual distance calc
                    // reuse segment-distance logic from above
                    let a = p0;
                    let b = p1;
                    let ab = (b.x - a.x, b.y - a.y);
                    let ap = (pt.x - a.x, pt.y - a.y);
                    let ab_len2 = ab.0 * ab.0 + ab.1 * ab.1;
                    let t = if ab_len2 > 0.0 {
                        ((ap.0 * ab.0 + ap.1 * ab.1) / ab_len2).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let closest = Point::new(a.x + ab.0 * t, a.y + ab.1 * t);
                    if ((closest.x - pt.x).powi(2) + (closest.y - pt.y).powi(2)).sqrt() <= TOLERANCE
                    {
                        return true;
                    }
                }
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vello::kurbo::Point;

    #[test]
    fn hit_test_rect() {
        let elem = CanvasElement {
            id: "x".into(),
            parent_void: "v".into(),
            geometry: Geometry::Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 100.0,
                y1: 100.0,
            },
            text: None,
            color: [1.0, 0.0, 0.0, 1.0],
            z_index: 0,
            neuro_state: None,
        };
        assert!(elem.hit_test(Point::new(50.0, 50.0)));
        assert!(!elem.hit_test(Point::new(150.0, 150.0)));
    }

    #[test]
    fn hit_test_line() {
        let elem = CanvasElement {
            id: "l".into(),
            parent_void: "v".into(),
            geometry: Geometry::Line {
                start: (0.0, 0.0),
                end: (100.0, 0.0),
            },
            text: None,
            color: [0.0, 0.0, 0.0, 1.0],
            z_index: 0,
            neuro_state: None,
        };
        // point near the segment
        assert!(elem.hit_test(Point::new(50.0, 1.0)));
        // far away
        assert!(!elem.hit_test(Point::new(50.0, 10.0)));
    }
}
