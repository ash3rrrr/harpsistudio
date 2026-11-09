//! Custom-painted controls: knobs, faders, meters, chips and clip previews.

use crate::theme::*;
use eframe::egui::{self, Align2, Color32, Id, Painter, Pos2, Rect, Response, Sense, Ui, Vec2};
use hs_core::song::Clip;

pub fn chip(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    label: &str,
    active: bool,
    accent: Color32,
) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    let hover = resp.hovered();
    let bg = if active {
        accent
    } else if hover {
        PANEL3
    } else {
        PANEL2
    };
    let fg = if active { BG } else { TEXT };
    let p = ui.painter();
    rounded(p, rect, 4.0, bg);
    if !active {
        outline(p, rect, 4.0, LINE, 1.0);
    }
    txt(p, rect.center(), Align2::CENTER_CENTER, label, 11.0, fg);
    resp
}

pub fn small_btn(ui: &mut Ui, rect: Rect, id: Id, label: &str, accent: Color32) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    let p = ui.painter();
    let bg = if resp.hovered() { PANEL3 } else { PANEL2 };
    rounded(p, rect, 3.0, bg);
    outline(p, rect, 3.0, LINE, 1.0);
    txt(p, rect.center(), Align2::CENTER_CENTER, label, 10.0, accent);
    resp
}

pub fn toggle_sq(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    label: &str,
    on: bool,
    accent: Color32,
) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    let p = ui.painter();
    let bg = if on {
        alpha(accent, 220)
    } else if resp.hovered() {
        PANEL3
    } else {
        PANEL2
    };
    rounded(p, rect, 3.0, bg);
    outline(p, rect, 3.0, if on { accent } else { LINE }, 1.0);
    txt(
        p,
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        10.0,
        if on { BG } else { DIM },
    );
    resp
}

/// Static description of a knob: its range, default, label and accent colour.
pub struct KnobSpec<'a> {
    pub def: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub bipolar: bool,
    pub label: &'a str,
    pub accent: Color32,
}

/// Rotary control. Returns true when the value changed.
pub fn knob(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    value: &mut f32,
    display: &str,
    spec: &KnobSpec,
) -> bool {
    let KnobSpec {
        def,
        min,
        max,
        step,
        bipolar,
        label,
        accent,
    } = *spec;
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let mut changed = false;
    let range = (max - min).max(1e-6);

    if resp.dragged() {
        let (dy, shift) = ui.ctx().input(|i| (i.pointer.delta().y, i.modifiers.shift));
        let speed = if shift { 0.0012 } else { 0.0055 };
        let norm = ((*value - min) / range).clamp(0.0, 1.0);
        let mut nv = (norm - dy * speed).clamp(0.0, 1.0);
        let mut v = min + nv * range;
        if step > 0.0 {
            v = (v / step).round() * step;
        }
        nv = ((v - min) / range).clamp(0.0, 1.0);
        v = min + nv * range;
        if (v - *value).abs() > 1e-7 {
            *value = v;
            changed = true;
        }
    }
    if resp.double_clicked() {
        *value = def;
        changed = true;
    }

    let p = ui.painter();
    let center = Pos2::new(rect.center().x, rect.top() + 25.0);
    let r = 17.0;
    let hover = resp.hovered() || resp.dragged();
    let norm = ((*value - min) / range).clamp(0.0, 1.0);
    let start = 135.0f32.to_radians();
    let sweep = 270.0f32.to_radians();

    p.circle_filled(center, r + 3.0, if hover { PANEL3 } else { PANEL2 });
    p.circle_stroke(center, r + 3.0, egui::Stroke::new(1.0, LINE_SOFT));

    // Track arc
    let mut pts = Vec::with_capacity(40);
    for i in 0..=36 {
        let a = start + sweep * (i as f32 / 36.0);
        pts.push(center + Vec2::new(a.cos(), a.sin()) * (r - 1.0));
    }
    p.add(egui::Shape::line(pts, egui::Stroke::new(2.5, LINE)));

    // Value arc
    if norm > 0.001 {
        let mut pts = Vec::with_capacity(40);
        let steps = ((norm * 36.0).ceil() as usize).max(2);
        for i in 0..=steps {
            let a = start + sweep * norm * (i as f32 / steps as f32);
            pts.push(center + Vec2::new(a.cos(), a.sin()) * (r - 1.0));
        }
        p.add(egui::Shape::line(pts, egui::Stroke::new(2.5, accent)));
    }

    // Bipolar centre mark
    if bipolar {
        let a = start + sweep * 0.5;
        let d = Vec2::new(a.cos(), a.sin());
        p.line_segment(
            [center + d * (r + 1.0), center + d * (r + 5.0)],
            egui::Stroke::new(1.0, DIM),
        );
    }

    // Indicator
    let a = start + sweep * norm;
    let d = Vec2::new(a.cos(), a.sin());
    p.line_segment(
        [center + d * 4.0, center + d * (r - 5.0)],
        egui::Stroke::new(2.0, if hover { TEXT } else { mix(TEXT, DIM, 0.3) }),
    );
    p.circle_filled(center, 2.5, accent);

    txt(
        p,
        Pos2::new(rect.center().x, rect.bottom() - 24.0),
        Align2::CENTER_CENTER,
        fit(label, rect.width(), 9.5),
        9.5,
        DIM,
    );
    mono(
        p,
        Pos2::new(rect.center().x, rect.bottom() - 10.0),
        Align2::CENTER_CENTER,
        fit(display, rect.width(), 10.0),
        10.0,
        if hover { accent } else { TEXT },
    );
    changed
}

/// Vertical fader with a wide grab area. Returns true when changed.
pub fn fader(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    value: &mut f32,
    def: f32,
    max: f32,
    accent: Color32,
) -> bool {
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() {
        let dy = ui.ctx().input(|i| i.pointer.delta().y);
        let nv = (*value + (-dy) * (max / rect.height().max(20.0))).clamp(0.0, max);
        if (nv - *value).abs() > 1e-7 {
            *value = nv;
            changed = true;
        }
    }
    if resp.double_clicked() {
        *value = def;
        changed = true;
    }

    let p = ui.painter();
    let x = rect.center().x;
    let top = rect.top() + 8.0;
    let bot = rect.bottom() - 8.0;
    let norm = (*value / max).clamp(0.0, 1.0);

    rounded(
        p,
        Rect::from_min_max(Pos2::new(x - 3.0, top), Pos2::new(x + 3.0, bot)),
        2.0,
        LINE_SOFT,
    );
    let y = bot + (top - bot) * norm;
    rounded(
        p,
        Rect::from_min_max(Pos2::new(x - 3.0, y), Pos2::new(x + 3.0, bot)),
        2.0,
        accent,
    );
    let handle = Rect::from_center_size(Pos2::new(x, y), Vec2::new(26.0, 12.0));
    rounded(
        p,
        handle,
        3.0,
        if resp.hovered() || resp.dragged() {
            PANEL3
        } else {
            PANEL2
        },
    );
    outline(p, handle, 3.0, accent, 1.2);
    hline(p, x - 9.0, x + 9.0, y, TEXT);
    changed
}

/// Peak meter, segmented and colour-coded.
pub fn meter(p: &Painter, rect: Rect, level: f32, vertical: bool) {
    rounded(p, rect, 2.0, PANEL2);
    let n = if vertical { 26 } else { 40 };
    let lv = level.clamp(0.0, 1.0).powf(0.55);
    for i in 0..n {
        let t = i as f32 / n as f32;
        if t > lv {
            break;
        }
        let color = if t > 0.92 {
            RED
        } else if t > 0.75 {
            ACCENT
        } else {
            MINT
        };
        if vertical {
            let seg_h = (rect.height() - 4.0) / n as f32;
            let y = rect.bottom() - 2.0 - (i as f32 + 1.0) * seg_h;
            p.rect_filled(
                Rect::from_min_size(
                    Pos2::new(rect.left() + 2.0, y + 0.5),
                    Vec2::new(rect.width() - 4.0, seg_h - 1.0),
                ),
                1.0,
                color,
            );
        } else {
            let seg_w = (rect.width() - 4.0) / n as f32;
            let x = rect.left() + 2.0 + i as f32 * seg_w;
            p.rect_filled(
                Rect::from_min_size(
                    Pos2::new(x + 0.5, rect.top() + 2.0),
                    Vec2::new(seg_w - 1.0, rect.height() - 4.0),
                ),
                1.0,
                color,
            );
        }
    }
}

/// Drag a numeric value horizontally (BPM etc.).
pub fn drag_number(
    ui: &mut Ui,
    rect: Rect,
    id: Id,
    value: &mut f64,
    min: f64,
    max: f64,
    step: f64,
) -> bool {
    let resp = ui.interact(rect, id, Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() {
        let dx = ui.ctx().input(|i| i.pointer.delta().x);
        let shift = ui.ctx().input(|i| i.modifiers.shift);
        let sp = if shift { step * 0.1 } else { step };
        let nv = (*value + dx as f64 * sp).clamp(min, max);
        let nv = (nv / sp).round() * sp;
        if (nv - *value).abs() > 1e-9 {
            *value = nv;
            changed = true;
        }
    }
    changed
}

/// Miniature note preview used inside clip blocks.
pub fn draw_clip_notes(p: &Painter, rect: Rect, clip: &Clip, color: Color32) {
    if clip.notes.is_empty() || clip.length <= 0.0 || rect.width() < 4.0 {
        return;
    }
    let (lo, hi) = clip.note_bounds();
    let span = (hi.saturating_sub(lo)).max(8) as f32;
    for n in &clip.notes {
        let x = rect.left() + (n.start / clip.length).clamp(0.0, 1.0) as f32 * rect.width();
        let w = ((n.len / clip.length) as f32 * rect.width()).max(1.5);
        let y = rect.bottom() - ((n.pitch.saturating_sub(lo)) as f32 / span) * rect.height();
        p.rect_filled(
            Rect::from_min_size(Pos2::new(x, y - 1.5), Vec2::new(w, 2.6)),
            1.0,
            color,
        );
    }
}

/// Small horizontal level bar for compact places.
pub fn hbar(p: &Painter, rect: Rect, level: f32) {
    rounded(p, rect, 2.0, PANEL2);
    let w = rect.width() * level.clamp(0.0, 1.0).powf(0.6);
    rounded(
        p,
        Rect::from_min_size(rect.min, Vec2::new(w, rect.height())),
        2.0,
        if level > 0.95 { RED } else { MINT },
    );
}
