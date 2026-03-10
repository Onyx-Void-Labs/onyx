// ─── Onyx App — Contextual Ribbon ─────────────────────────────────────────

use vello::kurbo::{Rect, RoundedRect, Affine, Line, Stroke, BezPath, Circle};
use vello::peniko::{Brush, Color, Fill};
use vello::Scene;
use std::collections::HashMap;
use parley::style::{StyleProperty, FontWeight, FontStyle};
use parley::layout::{Alignment, AlignmentOptions};

const BTN_H: f64 = 32.0;
const BTN_R: f64 = 5.0;

#[derive(Clone, Copy, PartialEq)]
pub enum BtnStyle { Normal, Active }

fn measure_text(
    font_cx: &mut parley::FontContext,
    layout_cx: &mut parley::LayoutContext<vello::peniko::Brush>,
    text: &str, size: f32, bold: bool,
) -> f64 {
    let mut builder = layout_cx.ranged_builder(font_cx, text, 1.0, false);
    builder.push_default(StyleProperty::FontSize(size));
    builder.push_default(StyleProperty::Brush(Brush::Solid(Color::WHITE)));
    if bold { builder.push_default(StyleProperty::FontWeight(FontWeight::BOLD)); }
    let mut layout = builder.build(text);
    layout.break_all_lines(None);
    layout.align(None, Alignment::Start, AlignmentOptions::default());
    layout.width() as f64
}

fn draw_ui_text(
    scene: &mut Scene,
    font_cx: &mut parley::FontContext,
    layout_cx: &mut parley::LayoutContext<vello::peniko::Brush>,
    text: &str, x: f64, y: f64,
    size: f32, color: Color, bold: bool, italic: bool,
) {
    let mut builder = layout_cx.ranged_builder(font_cx, text, 1.0, false);
    builder.push_default(StyleProperty::FontSize(size));
    builder.push_default(StyleProperty::Brush(Brush::Solid(color)));
    if bold   { builder.push_default(StyleProperty::FontWeight(FontWeight::BOLD)); }
    if italic { builder.push_default(StyleProperty::FontStyle(FontStyle::Italic)); }
    let mut layout = builder.build(text);
    layout.break_all_lines(None);
    layout.align(None, Alignment::Start, AlignmentOptions::default());
    parley_vello::render_text(scene, Affine::translate((x, y)), &layout);
}

pub fn draw_btn_bg(scene: &mut Scene, rect: Rect, style: BtnStyle, is_hovered: bool) {
    let (bg, border) = match style {
        BtnStyle::Active => (
            Color::from_rgba8(68, 95, 208, 235),
            Color::from_rgba8(108, 138, 255, 210),
        ),
        BtnStyle::Normal => {
            if is_hovered {
                (
                    Color::from_rgba8(36, 36, 44, 255),
                    Color::from_rgba8(78, 78, 102, 255),
                )
            } else {
                (
                    Color::from_rgba8(26, 26, 32, 255),
                    Color::from_rgba8(68, 68, 92, 255),
                )
            }
        }
    };

    let rr = RoundedRect::from_rect(rect, BTN_R);
    scene.fill(Fill::NonZero, Affine::IDENTITY, &Brush::Solid(bg), None, &rr);

    let inner = RoundedRect::from_rect(rect.inset(0.5), (BTN_R - 0.5).max(0.0));
    scene.stroke(
        &Stroke::new(1.0), Affine::IDENTITY,
        &Brush::Solid(border), None, &inner,
    );
}

// ── Icons ─────────────────────────────────────────────────────────────────

fn icon_highlight(
    scene: &mut Scene,
    font_cx: &mut parley::FontContext,
    layout_cx: &mut parley::LayoutContext<vello::peniko::Brush>,
    cx: f64, cy: f64, last_color: Color,
) {
    // Background highlight box
    let bg_rect = Rect::new(cx - 9.0, cy - 10.0, cx + 9.0, cy + 10.0);
    scene.fill(
        Fill::NonZero, Affine::IDENTITY,
        &Brush::Solid(last_color), None,
        &RoundedRect::from_rect(bg_rect, 2.0),
    );

    // "A" drawn in dark color over the yellow/colored highlight
    let size = 14.0f32;
    let tw = measure_text(font_cx, layout_cx, "A", size, false);
    let text_y = cy - 10.0;
    draw_ui_text(scene, font_cx, layout_cx, "A",
        cx - tw / 2.0, text_y, size,
        Color::from_rgba8(20, 20, 20, 255), true, false);
}

fn icon_color(
    scene: &mut Scene,
    font_cx: &mut parley::FontContext,
    layout_cx: &mut parley::LayoutContext<vello::peniko::Brush>,
    cx: f64, cy: f64, last_color: Color,
) {
    // White A + thick colored bar below — classic font-color-picker icon
    let tw = measure_text(font_cx, layout_cx, "A", 14.0, false);
    draw_ui_text(scene, font_cx, layout_cx, "A",
        cx - tw / 2.0, cy - 10.0, 14.0,
        Color::from_rgba8(230, 230, 240, 255), false, false);
    // 3px bar, centered under the letter (not full button width)
    scene.stroke(&Stroke::new(3.0), Affine::IDENTITY,
        &Brush::Solid(last_color), None,
        &Line::new((cx - 7.0, cy + 7.0), (cx + 7.0, cy + 7.0)));
}

fn icon_underline(
    scene: &mut Scene,
    font_cx: &mut parley::FontContext,
    layout_cx: &mut parley::LayoutContext<vello::peniko::Brush>,
    x: f64, y: f64, w: f64,
) {
    let cx = x + w / 2.0;
    let c = Color::from_rgba8(230, 230, 240, 255);

    let tw = measure_text(font_cx, layout_cx, "U", 14.0, false);
    let ty = y + 7.0; // more centered
    draw_ui_text(scene, font_cx, layout_cx, "U", cx - tw / 2.0, ty, 14.0, c, false, false);
    scene.stroke(&Stroke::new(2.0), Affine::IDENTITY, &Brush::Solid(Color::from_rgba8(100, 160, 255, 255)), None,
        &Line::new((cx - 6.0, ty + 15.0), (cx + 6.0, ty + 15.0)));
}

fn icon_script(
    scene: &mut Scene,
    font_cx: &mut parley::FontContext,
    layout_cx: &mut parley::LayoutContext<vello::peniko::Brush>,
    cx: f64, cy: f64, is_super: bool,
) {
    let c = Color::from_rgba8(230, 230, 240, 255);
    let x_width = measure_text(font_cx, layout_cx, "x", 14.0, false);
    
    // Draw base "x"
    let base_x = cx - 7.0; 
    let base_y = cy - 10.0;
    draw_ui_text(scene, font_cx, layout_cx, "x", base_x, base_y, 14.0, c, false, false);
    
    // Draw offset "2" in accent color
    let two_x = base_x + x_width + 1.0;
    let two_y = if is_super { base_y - 2.0 } else { base_y + 7.0 };
    let accent = Color::from_rgba8(100, 160, 255, 255);
    draw_ui_text(scene, font_cx, layout_cx, "2", two_x, two_y, 9.0, accent, false, false);
}

fn icon_strikethrough(
    scene: &mut Scene,
    font_cx: &mut parley::FontContext,
    layout_cx: &mut parley::LayoutContext<vello::peniko::Brush>,
    x: f64, y: f64, w: f64,
) {
    let cx = x + w / 2.0;
    let c = Color::from_rgba8(230, 230, 240, 255);
    let size = 14.0f32;

    let ty = y + (BTN_H - size as f64) / 2.0 - 1.0;
    let tw = measure_text(font_cx, layout_cx, "S", size, false);
    draw_ui_text(scene, font_cx, layout_cx, "S", cx - tw / 2.0, ty, size, c, false, false);

    let line_y = ty + 8.0;
    scene.stroke(&Stroke::new(1.5), Affine::IDENTITY,
        &Brush::Solid(Color::from_rgba8(100, 160, 255, 255)), None,
        &Line::new((cx - 8.0, line_y), (cx + 8.0, line_y)));
}

fn icon_italic(scene: &mut Scene, cx: f64, cy: f64) {
    let b = Brush::Solid(Color::from_rgba8(230, 230, 240, 255));
    let s = Stroke::new(1.8);
    // top bar
    scene.stroke(&s, Affine::IDENTITY, &b, None, &Line::new((cx - 3.0, cy - 6.0), (cx + 4.0, cy - 6.0)));
    // bottom bar
    scene.stroke(&s, Affine::IDENTITY, &b, None, &Line::new((cx - 4.0, cy + 6.0), (cx + 3.0, cy + 6.0)));
    // main stem
    scene.stroke(&s, Affine::IDENTITY, &b, None, &Line::new((cx + 0.5, cy - 6.0), (cx - 0.5, cy + 6.0)));
}

fn icon_plus_minus(scene: &mut Scene, cx: f64, cy: f64, is_plus: bool) {
    let b = Brush::Solid(Color::from_rgba8(180, 180, 205, 255));
    let s = Stroke::new(1.5);
    scene.stroke(&s, Affine::IDENTITY, &b, None,
        &Line::new((cx - 5.0, cy), (cx + 5.0, cy)));
    if is_plus {
        scene.stroke(&s, Affine::IDENTITY, &b, None,
            &Line::new((cx, cy - 5.0), (cx, cy + 5.0)));
    }
}

fn icon_align(scene: &mut Scene, x: f64, y: f64, w: f64, align: &str) {
    let cx = x + w / 2.0;
    let cy = y + BTN_H / 2.0;
    let b = Brush::Solid(Color::from_rgba8(220, 220, 235, 255));
    let s = Stroke::new(1.6);
    let full = 14.0_f64;
    let short = 9.0_f64;
    for (dy, is_full) in [(-7.5, true), (-2.5, false), (2.5, true), (7.5, false)] {
        let len = if is_full { full } else { short };
        let (x0, x1) = match align {
            "align_left"  => (cx - full / 2.0, cx - full / 2.0 + len),
            "align_right" => (cx + full / 2.0 - len, cx + full / 2.0),
            _             => (cx - len / 2.0, cx + len / 2.0),
        };
        scene.stroke(&s, Affine::IDENTITY, &b, None, &Line::new((x0, cy + dy), (x1, cy + dy)));
    }
}

fn icon_bullet(scene: &mut Scene, cx: f64, cy: f64) {
    let b = Brush::Solid(Color::from_rgba8(220, 220, 235, 255));
    let s = Stroke::new(1.4);
    for &dy in &[-6.5_f64, 0.0, 6.5] {
        scene.fill(Fill::NonZero, Affine::IDENTITY, &b, None,
            &Circle::new((cx - 6.5, cy + dy), 1.8));
        scene.stroke(&s, Affine::IDENTITY, &b, None,
            &Line::new((cx - 2.5, cy + dy), (cx + 8.0, cy + dy)));
    }
}

fn draw_caret(scene: &mut Scene, x: f64, cy: f64) {
    let mut p = BezPath::new();
    p.move_to((x - 4.0, cy - 2.0));
    p.line_to((x + 4.0, cy - 2.0));
    p.line_to((x, cy + 3.0));
    p.close_path();
    scene.fill(Fill::NonZero, Affine::IDENTITY,
        &Brush::Solid(Color::from_rgba8(150, 150, 175, 255)), None, &p);
}

fn draw_divider(scene: &mut Scene, x: f64, y_top: f64, h: f64) {
    scene.fill(Fill::NonZero, Affine::IDENTITY,
        &Brush::Solid(Color::from_rgba8(255, 255, 255, 10)), None,
        &Rect::new(x, y_top, x + 1.0, y_top + h));
    scene.fill(Fill::NonZero, Affine::IDENTITY,
        &Brush::Solid(Color::from_rgba8(0, 0, 0, 35)), None,
        &Rect::new(x + 1.0, y_top, x + 2.0, y_top + h));
}

// ── Main ribbon ───────────────────────────────────────────────────────────

pub fn draw_ribbon(
    scene: &mut Scene,
    font_cx: &mut parley::FontContext,
    layout_cx: &mut parley::LayoutContext<vello::peniko::Brush>,
    width: f64,
    y_pos: f64,
    active_ids: &[&str],
    hovered_id: Option<&str>,
    font_family_label: &str,
    font_size_label: &str,
    _active_dropdown: crate::app::DropdownType,
    _dropdown_search: &str,
    last_text_color: Color,
    last_highlight_color: Color,
    hitboxes: &mut HashMap<String, Rect>,
) {
    const RIBBON_H: f64 = 92.0;
    const TOP_ROW:  f64 = 12.0;
    const BOT_ROW:  f64 = 52.0;
    const GAP:      f64 = 6.0;
    const BW:       f64 = 32.0;

    let top_y = y_pos + TOP_ROW;
    let bot_y = y_pos + BOT_ROW;

    let is_active = |id: &str| active_ids.contains(&id);
    let is_hovered = |id: &str| hovered_id == Some(&format!("ribbon:{}", id));

    // ── Background ───────────────────────────────────────────────────────
    scene.fill(Fill::NonZero, Affine::IDENTITY,
        &Brush::Solid(Color::from_rgba8(18, 18, 22, 255)), None,
        &Rect::new(0.0, y_pos, width, y_pos + RIBBON_H));
    scene.fill(Fill::NonZero, Affine::IDENTITY,
        &Brush::Solid(Color::from_rgba8(255, 255, 255, 14)), None,
        &Rect::new(0.0, y_pos, width, y_pos + 1.0));
    scene.fill(Fill::NonZero, Affine::IDENTITY,
        &Brush::Solid(Color::from_rgba8(0, 0, 0, 100)), None,
        &Rect::new(0.0, y_pos + RIBBON_H - 1.0, width, y_pos + RIBBON_H));

    // ── Button helper closure ─────────────────────────────────────────────
    let draw_btn = |
        scene: &mut Scene,
        font_cx: &mut parley::FontContext,
        layout_cx: &mut parley::LayoutContext<vello::peniko::Brush>,
        hitboxes: &mut HashMap<String, Rect>,
        label: &str, x: f64, y: f64, w: f64, id: &str,
    | {
        let rect = Rect::new(x, y, x + w, y + BTN_H);
        let is_dropdown = id == "font_family" || id == "font_size";
        let style = if is_active(id) { BtnStyle::Active } else { BtnStyle::Normal };
        draw_btn_bg(scene, rect, style, is_hovered(id));

        let btn_cx = x + w / 2.0;
        let btn_cy = y + BTN_H / 2.0;
        let text_color = Color::from_rgba8(230, 230, 240, 255);

        match id {
            "highlight"     => icon_highlight(scene, font_cx, layout_cx, btn_cx, btn_cy, last_highlight_color),
            "color"         => icon_color(scene, font_cx, layout_cx, btn_cx, btn_cy, last_text_color),
            "underline"     => icon_underline(scene, font_cx, layout_cx, x, y, w),
            "strikethrough" => icon_strikethrough(scene, font_cx, layout_cx, x, y, w),
            "font_grow"     => icon_plus_minus(scene, btn_cx, btn_cy, true),
            "font_shrink"   => icon_plus_minus(scene, btn_cx, btn_cy, false),
            "superscript"   => icon_script(scene, font_cx, layout_cx, btn_cx, btn_cy, true),
            "subscript"     => icon_script(scene, font_cx, layout_cx, btn_cx, btn_cy, false),
            "italic"        => icon_italic(scene, btn_cx, btn_cy),
            id if id.starts_with("align_") => icon_align(scene, x, y, w, id),
            "bullet"        => icon_bullet(scene, btn_cx, btn_cy),
            _ => {
                let is_bold   = id == "bold" || (id.starts_with('h') && id.len() == 2);
                let is_italic = false; // Italic has its own icon now
                let font_size = if id == "bold" { 15.0f32 } else { 13.0 };
                let tw = measure_text(font_cx, layout_cx, label, font_size, is_bold);
                let tx = if is_dropdown { x + 10.0 } else { x + (w - tw) / 2.0 };
                let ty = y + (BTN_H - font_size as f64) / 2.0 - 1.0;
                draw_ui_text(scene, font_cx, layout_cx, label, tx, ty, font_size,
                    text_color, is_bold, is_italic);
                if is_dropdown {
                    draw_caret(scene, x + w - 12.0, btn_cy);
                }
            }
        }
        hitboxes.insert(format!("ribbon:{}", id), rect);
    };

    let div_h = BOT_ROW + BTN_H - TOP_ROW - 2.0;

    // ── GROUP 1: FONT ────────────────────────────────────────────────────
    let mut cx = 20.0;

    draw_btn(scene, font_cx, layout_cx, hitboxes, font_family_label, cx, top_y, 110.0, "font_family");
    draw_btn(scene, font_cx, layout_cx, hitboxes, font_size_label, cx + 116.0, top_y, 52.0, "font_size");
    draw_btn(scene, font_cx, layout_cx, hitboxes, "", cx + 174.0, top_y, 28.0, "font_grow");
    draw_btn(scene, font_cx, layout_cx, hitboxes, "", cx + 208.0, top_y, 28.0, "font_shrink");

    let mut bx = cx;
    for id in ["bold", "italic", "underline", "strikethrough", "subscript", "superscript", "highlight", "color"] {
        let label = match id {
            "bold" => "B", "italic" => "I",
            _ => "",
        };
        draw_btn(scene, font_cx, layout_cx, hitboxes, label, bx, bot_y, BW, id);
        bx += BW + GAP;
    }

    let top_w: f64 = 28.0 + 6.0 + 28.0 + 52.0 + 6.0 + 110.0 + 6.0;
    let bot_w: f64 = (BW + GAP) * 8.0 - GAP;
    cx += top_w.max(bot_w) + 16.0;
    draw_divider(scene, cx, top_y + 2.0, div_h);
    cx += 14.0;

    // ── GROUP 2: PARAGRAPH ───────────────────────────────────────────────
    let aw = 36.0;
    for (i, id) in ["align_left", "align_center", "align_right"].iter().enumerate() {
        draw_btn(scene, font_cx, layout_cx, hitboxes, "", cx + (aw + GAP) * i as f64, top_y, aw, id);
    }
    draw_btn(scene, font_cx, layout_cx, hitboxes, "", cx, bot_y, BW, "bullet");
    draw_btn(scene, font_cx, layout_cx, hitboxes, "1.", cx + BW + GAP, bot_y, BW, "number");

    let para_top_w: f64 = (aw + GAP) * 3.0 - GAP;
    let para_bot_w: f64 = (BW + GAP) * 2.0 - GAP;
    cx += para_top_w.max(para_bot_w) + 16.0;
    draw_divider(scene, cx, top_y + 2.0, div_h);
    cx += 14.0;

    // ── GROUP 3: STYLES ──────────────────────────────────────────────────
    let sw = 38.0;
    for i in 1..=6usize {
        let id    = format!("h{}", i);
        let label = format!("H{}", i);
        draw_btn(scene, font_cx, layout_cx, hitboxes, &label,
            cx + (sw + GAP) * (i - 1) as f64, top_y, sw, &id);
    }
    draw_btn(scene, font_cx, layout_cx, hitboxes, "? Question",   cx,              bot_y,  98.0, "style_question");
    draw_btn(scene, font_cx, layout_cx, hitboxes, "📇 Flashcard", cx + 104.0,     bot_y, 112.0, "style_flashcard");

    // Dropdowns are now drawn separately to ensure they are always on top
}

pub fn draw_dropdowns(
    scene: &mut Scene,
    font_cx: &mut parley::FontContext,
    layout_cx: &mut parley::LayoutContext<vello::peniko::Brush>,
    _width: f64,
    y_pos: f64,
    hovered_id: Option<&str>,
    active_dropdown: crate::app::DropdownType,
    dropdown_search: &str,
    hitboxes: &mut HashMap<String, Rect>,
) {
    const TOP_ROW: f64 = 12.0;
    const BTN_H: f64 = 32.0;

    if active_dropdown != crate::app::DropdownType::None {
        let (x, y, items) = match active_dropdown {
            crate::app::DropdownType::FontFamily => (20.0, TOP_ROW + BTN_H + 4.0, 
                vec!["Inter", "Georgia", "Comic Sans MS", "Roboto", "Times New Roman", "Arial"]),
            crate::app::DropdownType::FontSize => (20.0 + 116.0, TOP_ROW + BTN_H + 4.0, 
                vec!["8", "10", "12", "14", "16", "18", "20", "24", "28", "32", "36", "48", "72"]),
            crate::app::DropdownType::HighlightColor => (248.0, 52.0 + BTN_H + 4.0, 
                vec!["Red", "Blue", "Green", "Yellow", "Orange", "Purple", "Gray", "Black", "White"]),
            crate::app::DropdownType::TextColor => (286.0, 52.0 + BTN_H + 4.0, 
                vec!["Red", "Blue", "Green", "Yellow", "Orange", "Purple", "Gray", "Black", "White"]),
            _ => (0.0, 0.0, vec![]),
        };

        if active_dropdown == crate::app::DropdownType::HighlightColor || active_dropdown == crate::app::DropdownType::TextColor {
            let colors = [
                ("Red", Color::from_rgba8(255, 51, 51, 255)),
                ("Orange", Color::from_rgba8(255, 128, 0, 255)),
                ("Yellow", Color::from_rgba8(255, 204, 0, 255)),
                ("Green", Color::from_rgba8(51, 204, 51, 255)),
                ("Blue", Color::from_rgba8(77, 128, 255, 255)),
                ("Purple", Color::from_rgba8(153, 77, 230, 255)),
                ("Gray", Color::from_rgba8(153, 153, 153, 255)),
                ("Black", Color::from_rgba8(26, 26, 26, 255)),
                ("White", Color::from_rgba8(255, 255, 255, 255)),
            ];

            let menu_w = 120.0;
            let grid_size = 32.0;
            let gap = 4.0;
            let padding = 8.0;
            let menu_h = padding * 2.0 + grid_size * 3.0 + gap * 2.0 + 32.0; // grid + clear btn

            let menu_rect = Rect::new(x, y_pos + y, x + menu_w, y_pos + y + menu_h);
            scene.fill(Fill::NonZero, Affine::IDENTITY, &Brush::Solid(Color::from_rgba8(28, 28, 34, 250)), None, &RoundedRect::from_rect(menu_rect, 4.0));
            scene.stroke(&Stroke::new(1.0), Affine::IDENTITY, &Brush::Solid(Color::from_rgba8(60, 60, 75, 255)), None, &RoundedRect::from_rect(menu_rect.inset(-0.5), 4.0));

            for (i, (name, color)) in colors.iter().enumerate() {
                let col = i % 3;
                let row = i / 3;
                let cx = x + padding + col as f64 * (grid_size + gap);
                let cy = y_pos + y + padding + row as f64 * (grid_size + gap);

                let rect = Rect::new(cx, cy, cx + grid_size, cy + grid_size);
                let id = format!("dropdown_item:{}", name);
                
                if hovered_id == Some(&id) {
                    scene.stroke(&Stroke::new(2.0), Affine::IDENTITY, &Brush::Solid(Color::WHITE), None, &RoundedRect::from_rect(rect.inset(2.0), 3.0));
                }
                
                scene.fill(Fill::NonZero, Affine::IDENTITY, &Brush::Solid(*color), None, &RoundedRect::from_rect(rect, 4.0));
                hitboxes.insert(id, rect);
            }

            // Clear Button
            let clear_y = y_pos + y + padding + 3.0 * (grid_size + gap);
            let clear_rect = Rect::new(x + padding, clear_y, x + menu_w - padding, clear_y + 24.0);
            let clear_id = format!("dropdown_item:Clear");
            
            if hovered_id == Some(&clear_id) {
                scene.fill(Fill::NonZero, Affine::IDENTITY, &Brush::Solid(Color::from_rgba8(50, 50, 70, 255)), None, &RoundedRect::from_rect(clear_rect, 2.0));
            }
            
            draw_ui_text(scene, font_cx, layout_cx, "None / Clear", x + padding + 16.0, clear_y + 4.0, 12.0, Color::from_rgba8(200, 200, 210, 255), false, false);
            hitboxes.insert(clear_id, clear_rect);

        } else {
            let filtered_items: Vec<&&str> = if dropdown_search.is_empty() {
                items.iter().collect()
            } else {
                items.iter().filter(|i| i.to_lowercase().contains(&dropdown_search.to_lowercase())).collect()
            };

            let item_h = 24.0;
            let menu_w = if active_dropdown == crate::app::DropdownType::FontFamily { 150.0 } else { 60.0 };
            let menu_h = (filtered_items.len() as f64 * item_h + 8.0).min(300.0);
            
            let menu_rect = Rect::new(x, y_pos + y, x + menu_w, y_pos + y + menu_h);
            scene.fill(Fill::NonZero, Affine::IDENTITY, &Brush::Solid(Color::from_rgba8(28, 28, 34, 250)), None, &RoundedRect::from_rect(menu_rect, 4.0));
            scene.stroke(&Stroke::new(1.0), Affine::IDENTITY, &Brush::Solid(Color::from_rgba8(60, 60, 75, 255)), None, &RoundedRect::from_rect(menu_rect.inset(-0.5), 4.0));

            for (i, &item) in filtered_items.iter().enumerate() {
                let item_y = y_pos + y + 4.0 + i as f64 * item_h;
                if item_y + item_h > y_pos + y + menu_h { break; }
                
                let item_rect = Rect::new(x + 2.0, item_y, x + menu_w - 2.0, item_y + item_h);
                let id = format!("dropdown_item:{}", item);
                
                if hovered_id == Some(&id) {
                    scene.fill(Fill::NonZero, Affine::IDENTITY, &Brush::Solid(Color::from_rgba8(50, 50, 70, 255)), None, &RoundedRect::from_rect(item_rect, 2.0));
                }
                
                draw_ui_text(scene, font_cx, layout_cx, item, x + 8.0, item_y + 4.0, 13.0, Color::WHITE, false, false);
                hitboxes.insert(id, item_rect);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ribbon_hitboxes() {
        let mut scene = Scene::new();
        let mut font_cx = parley::FontContext::new();
        let mut layout_cx = parley::LayoutContext::new();
        let mut hitboxes = HashMap::new();
        
        draw_ribbon(
            &mut scene,
            &mut font_cx,
            &mut layout_cx,
            800.0,
            0.0,
            &["bold", "italic"], // simulate active buttons
            Some("ribbon:bold"),
            "Inter",
            "16",
            crate::app::DropdownType::None,
            "",
            vello::peniko::Color::WHITE,
            vello::peniko::Color::WHITE,
            &mut hitboxes,
        );
        
        assert!(hitboxes.contains_key("ribbon:bold"));
        assert!(hitboxes.contains_key("ribbon:italic"));
        assert!(hitboxes.contains_key("ribbon:h1"));
        assert!(hitboxes.contains_key("ribbon:align_center"));
        assert!(hitboxes.contains_key("ribbon:font_family"));
        assert!(hitboxes.contains_key("ribbon:font_size"));
    }
}
