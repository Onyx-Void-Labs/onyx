// ─── Onyx App — 3D Flashcard Deck Renderer ─────────

use vello::Scene;
use vello::kurbo::{Affine, RoundedRect, Vec2};
use vello::peniko::{Brush, Color, Fill};
use crate::app::draw_text;

pub struct CardDeck {
    pub cards: Vec<String>,
}

impl CardDeck {
    pub fn new() -> Self {
         Self {
            cards: vec![
                "Calculus III".to_string(),
                "Discrete Math".to_string(),
                "Neural Networks".to_string(),
                "History 101".to_string(),
            ],
        }
    }

    pub fn draw(
        &self,
        scene: &mut Scene,
        font_cx: &mut parley::FontContext,
        layout_cx: &mut parley::LayoutContext<Brush>,
        x: f64,
        y: f64,
    ) {
        for (i, title) in self.cards.iter().enumerate().rev() {
            let offset_y = i as f64 * -6.0;
            let rotation = (i as f64 * 0.04).sin() * 0.08;
            let transform = Affine::translate(Vec2::new(x, y + offset_y)).pre_rotate(rotation);
            
            let card_rect = RoundedRect::new(0.0, 0.0, 200.0, 280.0, 16.0);
            
            // Depth Layer (Simulated 3D Shadow)
            let shadow_color = Color::from_rgba8(0, 0, 0, (40 + (i * 10)) as u8);
            scene.fill(Fill::NonZero, transform.pre_translate(Vec2::new(6.0, 6.0)), &Brush::Solid(shadow_color), None, &card_rect);
            
            // Gradient Surface
            let card_color = if i == 0 {
                Color::from_rgba8(45, 45, 55, 255)
            } else {
                Color::from_rgba8(35, 35, 40, 255)
            };
            
            // Border Glow
            let border_brush = Brush::Solid(Color::from_rgba8(70, 70, 90, 255));
            let border_rect = RoundedRect::new(-1.5, -1.5, 201.5, 281.5, 17.5);
            scene.fill(Fill::NonZero, transform, &border_brush, None, &border_rect);
            scene.fill(Fill::NonZero, transform, &Brush::Solid(card_color), None, &card_rect);

            // Title on Front Card
            if i == 0 {
                 draw_text(scene, font_cx, layout_cx, title, x + 25.0, y + 50.0 + offset_y, 18.0, Color::WHITE);
                 draw_text(scene, font_cx, layout_cx, "Review Now →", x + 25.0, y + 240.0 + offset_y, 14.0, Color::from_rgba8(150, 150, 180, 255));
            }
        }
    }
}
