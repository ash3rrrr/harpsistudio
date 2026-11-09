//! Harpsistudio visual language: near-black panels, gold primary accent and
//! mint secondary, with vivid track colours.

use eframe::egui::{self, Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, Vec2};

pub const BG: Color32 = Color32::from_rgb(10, 11, 14);
pub const PANEL: Color32 = Color32::from_rgb(18, 20, 25);
pub const PANEL2: Color32 = Color32::from_rgb(24, 27, 33);
pub const PANEL3: Color32 = Color32::from_rgb(32, 36, 44);
pub const LINE: Color32 = Color32::from_rgb(45, 50, 60);
pub const LINE_SOFT: Color32 = Color32::from_rgb(34, 38, 46);
pub const TEXT: Color32 = Color32::from_rgb(228, 231, 238);
pub const DIM: Color32 = Color32::from_rgb(134, 141, 155);
pub const FAINT: Color32 = Color32::from_rgb(86, 92, 105);
pub const ACCENT: Color32 = Color32::from_rgb(246, 196, 83);
pub const ACCENT_DARK: Color32 = Color32::from_rgb(150, 112, 30);
pub const MINT: Color32 = Color32::from_rgb(88, 224, 186);
pub const RED: Color32 = Color32::from_rgb(238, 92, 92);
pub const BLUE: Color32 = Color32::from_rgb(106, 156, 255);

pub fn c(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

pub fn alpha(color: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

pub fn txt(p: &Painter, pos: Pos2, anchor: Align2, s: impl ToString, size: f32, color: Color32) {
    p.text(
        pos,
        anchor,
        s.to_string(),
        FontId::proportional(size),
        color,
    );
}

pub fn mono(p: &Painter, pos: Pos2, anchor: Align2, s: impl ToString, size: f32, color: Color32) {
    p.text(pos, anchor, s.to_string(), FontId::monospace(size), color);
}

pub fn fill(p: &Painter, rect: Rect, color: Color32) {
    p.rect_filled(rect, 0.0, color);
}

pub fn rounded(p: &Painter, rect: Rect, radius: f32, color: Color32) {
    p.rect_filled(rect, radius, color);
}

pub fn outline(p: &Painter, rect: Rect, radius: f32, color: Color32, width: f32) {
    p.rect_stroke(
        rect,
        radius,
        Stroke::new(width, color),
        egui::StrokeKind::Inside,
    );
}

pub fn hline(p: &Painter, x0: f32, x1: f32, y: f32, color: Color32) {
    p.line_segment(
        [Pos2::new(x0, y), Pos2::new(x1, y)],
        Stroke::new(1.0, color),
    );
}

pub fn vline(p: &Painter, x: f32, y0: f32, y1: f32, color: Color32) {
    p.line_segment(
        [Pos2::new(x, y0), Pos2::new(x, y1)],
        Stroke::new(1.0, color),
    );
}

/// Truncate text with an ellipsis to roughly fit a pixel width.
pub fn fit(s: &str, width: f32, size: f32) -> String {
    let max_chars = (width / (size * 0.56)).max(1.0) as usize;
    if s.chars().count() <= max_chars {
        s.to_string()
    } else if max_chars <= 1 {
        "…".to_string()
    } else {
        let mut out: String = s.chars().take(max_chars - 1).collect();
        out.push('…');
        out
    }
}

/// Section label with a small accent tick.
pub fn section(p: &Painter, pos: Pos2, label: &str) {
    p.rect_filled(Rect::from_min_size(pos, Vec2::new(3.0, 11.0)), 1.0, ACCENT);
    txt(
        p,
        pos + Vec2::new(9.0, 0.0),
        Align2::LEFT_TOP,
        label,
        10.0,
        DIM,
    );
}

pub fn db_text(gain: f32) -> String {
    if gain <= 0.0001 {
        "-inf".to_string()
    } else {
        format!("{:+.1}", 20.0 * gain.log10())
    }
}
