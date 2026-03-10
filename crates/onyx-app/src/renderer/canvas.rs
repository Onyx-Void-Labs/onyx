// ─── Onyx App — Infinite Canvas Renderer ─────────

use vello::Scene;
use vello::kurbo::{Affine, Vec2, Rect, Stroke, BezPath, CubicBez};
use vello::peniko::{Brush, Color, Fill};
use onyx_core::OnyxWorkspace;
use onyx_core::canvas::Geometry;
use crate::app::draw_text;

pub struct CanvasRenderer {
    pub offset: Vec2,
    pub zoom: f64,
}

impl CanvasRenderer {
    pub fn new() -> Self {
        Self {
            offset: Vec2::ZERO,
            zoom: 1.0,
        }
    }

    pub fn draw(
        &self,
        scene: &mut Scene,
        font_cx: &mut parley::FontContext,
        layout_cx: &mut parley::LayoutContext<Brush>,
        ws: &OnyxWorkspace,
        void_id: &str,
        width: f64,
        height: f64,
    ) {
        // --- DRAW BACKGROUND GRID ---
        self.draw_grid(scene, width, height);

        let transform = Affine::translate(self.offset).pre_scale(self.zoom);
        
        // --- DRAW ELEMENTS ---
        let elements = ws.get_canvas_elements(void_id);
        for elem in elements {
            let brush = Brush::Solid(Color::from_rgba8(
                (elem.color[0] * 255.0) as u8,
                (elem.color[1] * 255.0) as u8,
                (elem.color[2] * 255.0) as u8,
                (elem.color[3] * 255.0) as u8,
            ));
            
            match elem.geometry {
                Geometry::Rect { x0, y0, x1, y1 } => {
                    let r = Rect::new(x0, y0, x1, y1);
                    // Draw card shadow/border for aesthetic
                    let shadow_brush = Brush::Solid(Color::from_rgba8(0, 0, 0, 40));
                    scene.fill(Fill::NonZero, transform, &shadow_brush, None, &r.inset(2.0));
                    
                    scene.fill(Fill::NonZero, transform, &brush, None, &r);
                    
                    if let Some(tag) = &elem.text {
                        if tag.starts_with("note:") {
                            let note_id = &tag[5..];
                            let blocks = ws.get_note_blocks(note_id);
                            let mut by = y0 + 15.0;
                            for block in blocks.iter().take(8) {
                                let tx = (x0 * self.zoom) + self.offset.x + 15.0;
                                let ty = (by * self.zoom) + self.offset.y;
                                draw_text(scene, font_cx, layout_cx, &block.content, tx, ty, (12.0 * self.zoom) as f32, Color::WHITE);
                                by += 20.0;
                            }
                        } else {
                            let tx = (x0 * self.zoom) + self.offset.x + 10.0;
                            let ty = (y0 * self.zoom) + self.offset.y + 10.0;
                            draw_text(scene, font_cx, layout_cx, tag, tx, ty, (14.0 * self.zoom) as f32, Color::WHITE);
                        }
                    }
                }
                Geometry::Line { start, end } => {
                    let line = vello::kurbo::Line::new((start.0, start.1), (end.0, end.1));
                    scene.stroke(&Stroke::new(2.0), transform, &brush, None, &line);
                }
                Geometry::Arrow { p0, p1, p2, p3 } => {
                     let bz = CubicBez::new(
                        (p0.0, p0.1), (p1.0, p1.1), (p2.0, p2.1), (p3.0, p3.1)
                     );
                     scene.stroke(&Stroke::new(2.0 * self.zoom), transform, &brush, None, &bz);
                }
                Geometry::Freehand(ref points) => {
                    if points.len() > 1 {
                        let mut path = BezPath::new();
                        path.move_to((points[0].0, points[0].1));
                        for p in &points[1..] {
                            path.line_to((p.0, p.1));
                        }
                        scene.stroke(&Stroke::new(2.0 * self.zoom), transform, &brush, None, &path);
                    }
                }
            }
        }
    }

    fn draw_grid(&self, scene: &mut Scene, width: f64, height: f64) {
        let grid_size = 40.0 * self.zoom;
        if grid_size < 5.0 { return; } // Don't draw too dense grid
        
        let stroke = Stroke::new(1.0);
        let brush = Brush::Solid(Color::from_rgba8(255, 255, 255, 10)); // Subdued blueprint
        
        let start_x = self.offset.x % grid_size;
        let start_y = self.offset.y % grid_size;
        
        let mut x = start_x;
        while x < width {
            scene.stroke(&stroke, Affine::IDENTITY, &brush, None, &vello::kurbo::Line::new((x, 0.0), (x, height)));
            x += grid_size;
        }
        
        let mut y = start_y;
        while y < height {
            scene.stroke(&stroke, Affine::IDENTITY, &brush, None, &vello::kurbo::Line::new((0.0, y), (width, y)));
            y += grid_size;
        }
    }
}
