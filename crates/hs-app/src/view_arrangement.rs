//! Arrangement view: a 16-bar timeline with draggable clips and playhead.

use crate::app::{clip_at_mut, Act, ClipRef, DragState, HarpsiApp};
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{self, Align2, Color32, Id, Pos2, Rect, Sense, Ui, Vec2};

pub fn show(app: &mut HarpsiApp, ui: &mut Ui, rect: Rect) {
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, BG);

    let left_w = 152.0;
    let ruler_h = 28.0;
    let bar_h = 30.0;
    let bars_visible = 16.0f64;
    let beats_visible = bars_visible * 4.0;
    let timeline = Rect::from_min_max(
        Pos2::new(rect.left() + left_w, rect.top() + ruler_h),
        Pos2::new(rect.right(), rect.bottom() - bar_h),
    );
    let ppb = timeline.width() as f64 / beats_visible;
    let n = app.song.tracks.len();
    let lane_h = if n == 0 {
        80.0
    } else {
        (timeline.height() / n as f32).clamp(58.0, 104.0)
    };

    let mut acts: Vec<Act> = Vec::new();
    let mut select_track: Option<usize> = None;
    let mut mute_toggle: Option<usize> = None;
    let mut solo_toggle: Option<usize> = None;
    let mut select_clip: Option<ClipRef> = None;
    let mut move_clip: Option<(ClipRef, f64)> = None;
    let mut new_clip_at: Option<(usize, f64)> = None;
    let mut seek: Option<f64> = None;

    let beat = app.shared.beat();
    let playing = app.shared.is_playing();
    let clip_edit = app.clip_edit;
    let sel_track = app.sel_track;

    let beats_to_x = |b: f64| timeline.left() + (b * ppb) as f32;
    let x_to_beats = |x: f32| ((x - timeline.left()) as f64 / ppb).max(0.0);

    // Ruler
    let ruler = Rect::from_min_max(
        Pos2::new(rect.left(), rect.top()),
        Pos2::new(rect.right(), rect.top() + ruler_h),
    );
    fill(&p, ruler, PANEL);
    fill(
        &p,
        Rect::from_min_size(rect.min, Vec2::new(left_w, ruler_h)),
        PANEL,
    );
    txt(
        &p,
        Pos2::new(rect.left() + 12.0, ruler.center().y),
        Align2::LEFT_CENTER,
        "ARRANGEMENT",
        9.0,
        ACCENT,
    );
    for bar in 0..=(bars_visible as usize) {
        let b = bar as f64 * 4.0;
        let x = beats_to_x(b);
        if x > ruler.right() {
            break;
        }
        vline(&p, x, ruler.top() + 8.0, ruler.bottom(), LINE);
        txt(
            &p,
            Pos2::new(x + 4.0, ruler.top() + 4.0),
            Align2::LEFT_TOP,
            format!("{}", bar + 1),
            8.5,
            if bar % 4 == 0 { TEXT } else { DIM },
        );
        if bar < bars_visible as usize {
            for beat_i in 1..4 {
                let bx = beats_to_x(b + beat_i as f64);
                vline(&p, bx, ruler.bottom() - 6.0, ruler.bottom(), LINE_SOFT);
            }
        }
    }
    if app.song.loop_enabled {
        let l0 = beats_to_x(app.song.loop_start).max(timeline.left());
        let l1 = beats_to_x(app.song.loop_end).min(timeline.right());
        if l1 > l0 {
            p.rect_filled(
                Rect::from_min_max(
                    Pos2::new(l0, ruler.top() + 21.0),
                    Pos2::new(l1, ruler.bottom()),
                ),
                1.0,
                alpha(ACCENT, 90),
            );
            txt(
                &p,
                Pos2::new(l0 + 5.0, ruler.bottom() - 2.0),
                Align2::LEFT_BOTTOM,
                "LOOP",
                6.5,
                BG,
            );
        }
    }
    let ruler_resp = ui.interact(
        Rect::from_min_max(
            Pos2::new(timeline.left(), rect.top()),
            Pos2::new(rect.right(), rect.top() + ruler_h),
        ),
        Id::unique("ruler"),
        Sense::click_and_drag(),
    );
    if ruler_resp.clicked() || ruler_resp.dragged() {
        if let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
            let b = x_to_beats(pos.x);
            seek = Some((b / 4.0).floor() * 4.0);
        }
    }

    // Loop region shading in lanes
    if app.song.loop_enabled {
        let l0 = beats_to_x(app.song.loop_start).max(timeline.left());
        let l1 = beats_to_x(app.song.loop_end).min(timeline.right());
        if l1 > l0 {
            p.rect_filled(
                Rect::from_min_max(
                    Pos2::new(l0, timeline.top()),
                    Pos2::new(l1, timeline.bottom()),
                ),
                0.0,
                alpha(ACCENT, 12),
            );
        }
    }

    for (ti, track) in app.song.tracks.iter().enumerate() {
        let y = timeline.top() + ti as f32 * lane_h;
        if y >= timeline.bottom() {
            break;
        }
        let lane = Rect::from_min_max(
            Pos2::new(rect.left(), y),
            Pos2::new(rect.right(), (y + lane_h).min(timeline.bottom())),
        );
        let lane_grid = Rect::from_min_max(
            Pos2::new(timeline.left(), y),
            Pos2::new(timeline.right(), (y + lane_h).min(timeline.bottom())),
        );
        let col = c(track.color);
        fill(
            &p,
            lane,
            if ti % 2 == 0 {
                BG
            } else {
                Color32::from_rgb(12, 14, 18)
            },
        );
        hline(
            &p,
            rect.left(),
            rect.right(),
            lane.bottom() - 1.0,
            LINE_SOFT,
        );

        // Header
        let hr = Rect::from_min_size(
            Pos2::new(rect.left(), y),
            Vec2::new(left_w - 1.0, lane_h - 1.0),
        );
        let selected = ti == sel_track;
        fill(
            &p,
            hr,
            if selected {
                mix(PANEL, col, 0.1)
            } else {
                PANEL
            },
        );
        p.rect_filled(
            Rect::from_min_size(hr.min, Vec2::new(3.0, hr.height())),
            0.0,
            col,
        );
        if selected {
            outline(&p, hr, 0.0, alpha(col, 140), 1.0);
        }
        let resp = ui.interact(hr, Id::unique(("arhdr", ti)), Sense::click());
        if resp.clicked() {
            select_track = Some(ti);
        }
        mono(
            &p,
            Pos2::new(hr.left() + 10.0, hr.top() + 6.0),
            Align2::LEFT_TOP,
            format!("{:02}", ti + 1),
            7.5,
            alpha(col, 210),
        );
        txt(
            &p,
            Pos2::new(hr.left() + 28.0, hr.top() + 6.0),
            Align2::LEFT_TOP,
            fit(&track.name, hr.width() - 36.0, 11.0),
            11.0,
            TEXT,
        );
        txt(
            &p,
            Pos2::new(hr.left() + 10.0, hr.top() + 22.0),
            Align2::LEFT_TOP,
            fit(
                &format!(
                    "{} · {} FX",
                    track.instrument.kind.short(),
                    track.effects.len()
                ),
                hr.width() - 20.0,
                7.5,
            ),
            7.5,
            alpha(col, 210),
        );
        if lane_h > 68.0 {
            let m = Rect::from_min_size(
                Pos2::new(hr.left() + 10.0, hr.bottom() - 19.0),
                Vec2::new(20.0, 15.0),
            );
            let s = Rect::from_min_size(
                Pos2::new(hr.left() + 34.0, hr.bottom() - 19.0),
                Vec2::new(20.0, 15.0),
            );
            let plus = Rect::from_min_size(
                Pos2::new(hr.right() - 30.0, hr.bottom() - 19.0),
                Vec2::new(24.0, 15.0),
            );
            if toggle_sq(ui, m, Id::unique(("amute", ti)), "M", track.mute, RED).clicked() {
                mute_toggle = Some(ti);
            }
            if toggle_sq(ui, s, Id::unique(("asolo", ti)), "S", track.solo, ACCENT).clicked() {
                solo_toggle = Some(ti);
            }
            if toggle_sq(ui, plus, Id::unique(("aadd", ti)), "＋", false, MINT).clicked() {
                new_clip_at = Some((ti, 0.0));
            }
        }

        // Grid click to create a clip
        let grid_resp = ui.interact(
            lane_grid,
            Id::unique(("alane", ti)),
            Sense::click_and_drag(),
        );
        if grid_resp.double_clicked() {
            if let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                let b = x_to_beats(pos.x);
                new_clip_at = Some((ti, (b / 4.0).floor() * 4.0));
            }
        }

        // Bar lines inside lane
        for bar in 0..=(bars_visible as usize) {
            let x = beats_to_x(bar as f64 * 4.0);
            if x > rect.right() {
                break;
            }
            vline(
                &p,
                x,
                lane.top(),
                lane.bottom(),
                if bar % 4 == 0 { LINE } else { LINE_SOFT },
            );
        }

        for (ci, clip) in track.clips.iter().filter(|c| c.scene < 0).enumerate() {
            let x0 = beats_to_x(clip.start).max(timeline.left());
            let x1 = beats_to_x(clip.end()).min(timeline.right());
            if x1 <= x0 {
                continue;
            }
            let cr = Rect::from_min_max(
                Pos2::new(x0 + 2.0, y + 6.0),
                Pos2::new(
                    (x1 - 2.0).max(x0 + 8.0),
                    (y + lane_h - 7.0).min(timeline.bottom()),
                ),
            );
            let cref = ClipRef {
                track: ti,
                scene: -1,
                start: clip.start,
            };
            let is_sel = clip_edit.map(|c| c == cref).unwrap_or(false);
            let is_playing = playing && beat >= clip.start && beat < clip.end();
            let resp = ui.interact(cr, Id::unique(("aclip", ti, ci)), Sense::click_and_drag());
            let bg = mix(PANEL2, col, if resp.hovered() { 0.34 } else { 0.24 });
            rounded(&p, cr, 3.0, bg);
            outline(
                &p,
                cr,
                3.0,
                if is_sel { ACCENT } else { mix(PANEL, col, 0.6) },
                if is_sel { 1.5 } else { 1.0 },
            );
            p.rect_filled(
                Rect::from_min_size(cr.min, Vec2::new(cr.width(), 2.0)),
                0.0,
                col,
            );
            txt(
                &p,
                Pos2::new(cr.left() + 7.0, cr.top() + 5.0),
                Align2::LEFT_TOP,
                fit(&clip.name, cr.width() - 34.0, 9.5),
                9.5,
                TEXT,
            );
            mono(
                &p,
                Pos2::new(cr.right() - 5.0, cr.top() + 5.0),
                Align2::RIGHT_TOP,
                format!("{:.1}", clip.length / 4.0),
                7.5,
                alpha(col, 220),
            );
            draw_clip_notes(
                &p,
                Rect::from_min_max(
                    Pos2::new(cr.left() + 6.0, cr.top() + 20.0),
                    Pos2::new(cr.right() - 5.0, cr.bottom() - 5.0),
                ),
                clip,
                alpha(col, 230),
            );
            if is_playing {
                p.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(cr.left() + cr.width() * 0.0, cr.bottom() - 2.0),
                        Vec2::new(cr.width(), 2.0),
                    ),
                    0.0,
                    MINT,
                );
            }
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
            if resp.clicked() {
                select_clip = Some(cref);
            }
            if resp.double_clicked() {
                select_clip = Some(cref);
                acts.push(Act::OpenClip(cref));
            }
            if resp.drag_started() {
                if let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                    let grab = x_to_beats(pos.x) - clip.start;
                    app.drag = DragState::ArrangeClip {
                        clip: cref,
                        grab_beat: grab,
                    };
                }
            }
            if resp.dragged() {
                if let DragState::ArrangeClip {
                    clip: active,
                    grab_beat,
                    ..
                } = app.drag
                {
                    if active.track == ti && (active.start - clip.start).abs() < 0.001 {
                        if let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                            let b = x_to_beats(pos.x) - grab_beat;
                            let snapped = (b.max(0.0) * 2.0).round() / 2.0;
                            move_clip = Some((cref, snapped));
                        }
                    }
                }
            }
        }
    }

    if n == 0 {
        txt(
            &p,
            rect.center(),
            Align2::CENTER_CENTER,
            "No tracks yet — load an instrument from the browser.",
            12.0,
            DIM,
        );
    }

    // Playhead
    if playing || beat > 0.0 {
        let x = beats_to_x(beat);
        if x >= timeline.left() && x <= timeline.right() {
            p.line_segment(
                [
                    Pos2::new(x, ruler.bottom()),
                    Pos2::new(x, timeline.bottom()),
                ],
                egui::Stroke::new(1.0, ACCENT),
            );
            p.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(x - 4.0, ruler.bottom() - 7.0),
                    Pos2::new(x + 4.0, ruler.bottom() - 7.0),
                    Pos2::new(x, ruler.bottom()),
                ],
                ACCENT,
                egui::Stroke::new(0.0, Color32::TRANSPARENT),
            ));
        }
    }

    // Bottom bar
    let bar = Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - bar_h), rect.max);
    fill(&p, bar, PANEL);
    hline(&p, bar.left(), bar.right(), bar.top(), LINE);
    txt(
        &p,
        Pos2::new(bar.left() + 12.0, bar.center().y),
        Align2::LEFT_CENTER,
        "Double-click empty lane: new clip  ·  drag clips to move  ·  double-click clip: piano roll",
        8.5,
        FAINT,
    );
    let loop_btn = Rect::from_min_size(
        Pos2::new(bar.right() - 196.0, bar.top() + 3.0),
        Vec2::new(58.0, 24.0),
    );
    if chip(
        ui,
        loop_btn,
        Id::unique("aloop"),
        "LOOP",
        app.song.loop_enabled,
        ACCENT,
    )
    .clicked()
    {
        acts.push(Act::ToggleLoop);
    }
    let stop_btn = Rect::from_min_size(
        Pos2::new(bar.right() - 130.0, bar.top() + 3.0),
        Vec2::new(58.0, 24.0),
    );
    if chip(ui, stop_btn, Id::unique("astop"), "STOP", false, RED).clicked() {
        acts.push(Act::StopClips);
    }
    let add_btn = Rect::from_min_size(
        Pos2::new(bar.right() - 64.0, bar.top() + 3.0),
        Vec2::new(52.0, 24.0),
    );
    if chip(ui, add_btn, Id::unique("aadd2"), "＋ CLIP", false, MINT).clicked() {
        let b = app.shared.beat();
        acts.push(Act::AddArrangementClip {
            track: sel_track,
            beat: b,
            bars: 4.0,
        });
    }

    if let Some(ti) = select_track {
        app.sel_track = ti;
    }
    if let Some(ti) = mute_toggle {
        if let Some(t) = app.song.tracks.get_mut(ti) {
            t.mute = !t.mute;
            app.dirty = true;
        }
    }
    if let Some(ti) = solo_toggle {
        if let Some(t) = app.song.tracks.get_mut(ti) {
            t.solo = !t.solo;
            app.dirty = true;
        }
    }
    if let Some(cr) = select_clip {
        app.clip_edit = Some(cr);
    }
    if let Some((cr, new_start)) = move_clip {
        {
            if let Some(clip) = clip_at_mut(&mut app.song, &cr) {
                clip.start = new_start;
            }
        }
        let new_cr = ClipRef {
            start: new_start,
            ..cr
        };
        if app.clip_edit.map(|c| c == cr).unwrap_or(false) {
            app.clip_edit = Some(new_cr);
        }
        if let DragState::ArrangeClip { clip, .. } = &mut app.drag {
            if *clip == cr {
                clip.start = new_start;
            }
        }
        app.publish_now();
    }
    if let Some((track, b)) = new_clip_at {
        acts.push(Act::AddArrangementClip {
            track,
            beat: b,
            bars: 4.0,
        });
    }
    if let Some(b) = seek {
        acts.push(Act::Seek(b));
    }
    for a in acts {
        app.act(a);
    }
}
