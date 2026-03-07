pub mod dock;
pub mod text;
pub mod titlebar;

use vello::Scene;

pub struct LayoutCtx<'a> {
    pub font_cx: &'a mut parley::FontContext,
    pub layout_cx: &'a mut parley::LayoutContext<vello::peniko::Brush>,
    pub scale: f32,
}

pub struct PaintCtx<'a> {
    pub scene: &'a mut Scene,
}

pub trait Widget {
    fn layout(&mut self, cx: &mut LayoutCtx, max_width: f32) -> (f32, f32);
    fn paint(&self, cx: &mut PaintCtx, x: f32, y: f32);
    fn hit_test(&self, x: f32, y: f32, wx: f32, wy: f32, w: f32, h: f32) -> bool {
        x >= wx && x <= wx + w && y >= wy && y <= wy + h
    }
}
