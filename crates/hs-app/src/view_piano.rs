//! Piano roll and drum-pad editor: draw, multi-select, move, resize, copy,
//! paste, duplicate and quantize notes. Drum Rack tracks are shown as their
//! sixteen named pads, and melodic tracks can highlight and snap to a scale.

use crate::app::{clip_at, clip_at_mut, Act, DragState, HarpsiApp, NoteDrag};
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{self, Align2, Color32, Id, Pos2, Rect, Sense, Ui, Vec2};
use hs_core::devices::{self, DRUM_LANES};
use hs_core::midi_fx;
use hs_core::params::{DeviceKind, DRUM_KIT_NAMES};
use hs_core::samples;
use hs_core::song::Note;
use std::collections::BTreeSet;

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

fn snap(v: f64, grid: f64) -> f64 {
    if grid <= 0.0 {
        v
    } else {
        (v / grid).round() * grid
    }
}

fn drum_rows() -> Vec<u8> {
    let mut pitches: Vec<u8> = DRUM_LANES.iter().map(|(_, pitch)| *pitch).collect();
    pitches.sort_unstable_by(|a, b| b.cmp(a));
    pitches
}

pub fn show(app: &mut HarpsiApp, ui: &mut Ui, rect: Rect) {
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, BG);

    let Some(cr) = app.clip_edit else {
        txt(
            &p,
            Pos2::new(rect.center().x, rect.center().y - 46.0),
            Align2::CENTER_CENTER,
            "No clip selected",
            18.0,
            TEXT,
        );
        txt(
            &p,
            Pos2::new(rect.center().x, rect.center().y - 22.0),
            Align2::CENTER_CENTER,
            "Start a new part, or click a clip in Session or Arrangement.",
            10.0,
            DIM,
        );
        let kinds = [
            ("＋ DRUM TRACK", DeviceKind::DrumRack),
            ("＋ MELODY", DeviceKind::Salamander),
            ("＋ BASS", DeviceKind::SubBass),
        ];
        let total_w = kinds.len() as f32 * 126.0 - 6.0;
        let kx = rect.center().x - total_w / 2.0;
        for (i, (label, kind)) in kinds.into_iter().enumerate() {
            let r = Rect::from_min_size(
                Pos2::new(kx + i as f32 * 126.0, rect.center().y + 2.0),
                Vec2::new(120.0, 26.0),
            );
            if chip(ui, r, Id::unique(("newpart", i)), label, false, MINT).clicked() {
                app.act(Act::CreatePart(kind));
            }
        }
        let vocal = Rect::from_min_size(
            Pos2::new(rect.center().x - 61.0, rect.center().y + 34.0),
            Vec2::new(122.0, 24.0),
        );
        if chip(
            ui,
            vocal,
            Id::unique("newvocal"),
            "＋ LOAD VOCAL",
            false,
            BLUE,
        )
        .clicked()
        {
            app.act(Act::ImportVocal(app.vocal_style));
        }
        txt(
            &p,
            Pos2::new(rect.center().x, rect.center().y + 72.0),
            Align2::CENTER_CENTER,
            "Drums open as named pads · melodies get scale snap and multi-note editing.",
            8.5,
            FAINT,
        );
        let b = Rect::from_min_size(
            Pos2::new(rect.center().x - 70.0, rect.center().y + 94.0),
            Vec2::new(140.0, 28.0),
        );
        if chip(ui, b, Id::unique("pback"), "CLOSE CLIP", false, ACCENT).clicked() {
            app.close_clip();
        }
        return;
    };

    let clip = match clip_at(&app.song, &cr) {
        Some(c) => c.clone(),
        None => {
            app.clip_edit = None;
            return;
        }
    };
    app.sel_track = cr.track;
    let (col, tname, inst, drum, kit) = match app.song.tracks.get(cr.track) {
        Some(track) => {
            let drum = track.instrument.kind == DeviceKind::DrumRack;
            let kit = if drum {
                track.instrument.p(8).round().clamp(0.0, 12.0) as usize
            } else {
                0
            };
            (
                c(track.color),
                track.name.clone(),
                track.instrument.kind.short().to_string(),
                drum,
                kit,
            )
        }
        None => (ACCENT, String::new(), String::new(), false, 0),
    };
    let kit_name = DRUM_KIT_NAMES.get(kit).copied().unwrap_or("Drum kit");
    let root = app.song.scale_root.rem_euclid(12);
    let scale_mode = app.song.scale_mode.clamp(0, 4) as usize;

    let mut select_only: Option<usize> = None;
    let mut toggle_sel: Option<usize> = None;
    let mut add_note: Option<Note> = None;
    let mut edit_notes: Vec<(usize, Note)> = Vec::new();
    let mut delete_notes: BTreeSet<usize> = BTreeSet::new();
    let mut quantize = false;
    let mut back = false;
    let mut copy = false;
    let mut cut = false;
    let mut paste = false;
    let mut duplicate = false;
    let mut set_len: Option<f64> = None;
    let mut set_vel: Option<f32> = None;
    let mut audition: Option<(u8, f32)> = None;
    let mut cursor_set: Option<f64> = None;

    // Header. The stacked Clip pane is half width, so its transport controls
    // move to a second row instead of running past the pane edge.
    let compact = rect.width() < 900.0;
    let header = Rect::from_min_size(
        rect.min,
        Vec2::new(rect.width(), if compact { 82.0 } else { 54.0 }),
    );
    fill(&p, header, PANEL);
    hline(
        &p,
        header.left(),
        header.right(),
        header.bottom() - 1.0,
        LINE,
    );
    p.rect_filled(
        Rect::from_min_size(header.min, Vec2::new(4.0, header.height())),
        0.0,
        col,
    );
    txt(
        &p,
        Pos2::new(header.left() + 16.0, header.top() + 10.0),
        Align2::LEFT_TOP,
        fit(&clip.name, 330.0, 15.0),
        15.0,
        TEXT,
    );
    let info = if drum {
        format!(
            "{} · DRUM RACK · {} · 16 PADS · {} HITS",
            tname.to_uppercase(),
            kit_name.to_uppercase(),
            clip.notes.len()
        )
    } else {
        format!(
            "{} · {} · {:.0} BARS · {} NOTES",
            tname.to_uppercase(),
            inst,
            clip.length / 4.0,
            clip.notes.len()
        )
    };
    txt(
        &p,
        Pos2::new(header.left() + 16.0, header.top() + 31.0),
        Align2::LEFT_TOP,
        fit(&info, 330.0, 8.0),
        8.0,
        alpha(col, 230),
    );

    // Note length chips
    let label_y = if compact {
        header.top() + 44.0
    } else {
        header.top() + 12.0
    };
    let chip_y = if compact {
        header.top() + 56.0
    } else {
        header.top() + 24.0
    };
    let mut hx = if compact {
        header.left() + 12.0
    } else {
        header.left() + 360.0
    };
    txt(
        &p,
        Pos2::new(hx, label_y),
        Align2::LEFT_TOP,
        "LENGTH",
        7.0,
        FAINT,
    );
    let lens = [("1/16", 0.25), ("1/8", 0.5), ("1/4", 1.0), ("1/2", 2.0)];
    for (i, (label, v)) in lens.iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(hx + i as f32 * 42.0, chip_y),
            Vec2::new(38.0, 20.0),
        );
        if chip(
            ui,
            r,
            Id::unique(("notelen", i)),
            label,
            (app.note_len - v).abs() < 0.001,
            ACCENT,
        )
        .clicked()
        {
            app.note_len = *v;
            set_len = Some(*v);
        }
    }
    hx += 186.0;
    txt(
        &p,
        Pos2::new(hx, label_y),
        Align2::LEFT_TOP,
        "VELOCITY",
        7.0,
        FAINT,
    );
    for (i, v) in [0.4f32, 0.7, 0.9, 1.0].iter().enumerate() {
        let r = Rect::from_min_size(
            Pos2::new(hx + i as f32 * 40.0, chip_y),
            Vec2::new(36.0, 20.0),
        );
        let label = format!("{:.0}", v * 100.0);
        if chip(
            ui,
            r,
            Id::unique(("vel", i)),
            &label,
            (app.note_vel - v).abs() < 0.001,
            MINT,
        )
        .clicked()
        {
            app.note_vel = *v;
            set_vel = Some(*v);
        }
    }
    hx += 172.0;
    let q = Rect::from_min_size(Pos2::new(hx, chip_y), Vec2::new(92.0, 20.0));
    if chip(ui, q, Id::unique("quant"), "QUANTIZE 1/16", false, BLUE).clicked() {
        quantize = true;
    }
    let ex = Rect::from_min_size(
        Pos2::new(
            header.right() - 88.0,
            if compact {
                header.top() + 8.0
            } else {
                header.top() + 24.0
            },
        ),
        Vec2::new(74.0, 20.0),
    );
    if chip(ui, ex, Id::unique("pexit"), "CLOSE", false, DIM).clicked() {
        back = true;
    }

    // Composition toolbar: scale helpers plus clipboard actions.
    let tools = Rect::from_min_size(
        Pos2::new(rect.left(), header.bottom()),
        Vec2::new(rect.width(), 32.0),
    );
    fill(&p, tools, PANEL2);
    hline(
        &p,
        tools.left(),
        tools.right(),
        tools.bottom() - 1.0,
        LINE_SOFT,
    );
    let ty = tools.top() + 6.0;
    let th = 20.0;
    let mut tx = tools.left() + 12.0;
    txt(
        &p,
        Pos2::new(tx, tools.center().y),
        Align2::LEFT_CENTER,
        "SCALE",
        7.5,
        FAINT,
    );
    tx += 40.0;
    let key_r = Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(66.0, th));
    if chip(
        ui,
        key_r,
        Id::unique("pkey"),
        &format!("KEY {}", NOTE_NAMES[root as usize]),
        false,
        ACCENT,
    )
    .clicked()
    {
        app.song.scale_root = (app.song.scale_root + 1).rem_euclid(12);
        app.dirty = true;
    }
    tx += 72.0;
    let mode_r = Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(118.0, th));
    if chip(
        ui,
        mode_r,
        Id::unique("pmode"),
        &format!("MODE {}", midi_fx::SCALE_NAMES[scale_mode].to_uppercase()),
        false,
        ACCENT,
    )
    .clicked()
    {
        app.song.scale_mode = (scale_mode as i32 + 1).rem_euclid(midi_fx::SCALE_NAMES.len() as i32);
        app.dirty = true;
    }
    tx += 124.0;
    let snap_r = Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(52.0, th));
    if chip(
        ui,
        snap_r,
        Id::unique("psnap"),
        "SNAP",
        app.song.scale_snap,
        MINT,
    )
    .clicked()
    {
        app.song.scale_snap = !app.song.scale_snap;
        app.dirty = true;
    }
    tx += 66.0;
    txt(
        &p,
        Pos2::new(tx, tools.center().y),
        Align2::LEFT_CENTER,
        "EDIT",
        7.5,
        FAINT,
    );
    tx += 34.0;
    let copy_r = Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(48.0, th));
    if chip(
        ui,
        copy_r,
        Id::unique("pcopy"),
        "COPY",
        !app.sel_notes.is_empty(),
        ACCENT,
    )
    .clicked()
    {
        copy = true;
    }
    tx += 54.0;
    let cut_r = Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(40.0, th));
    if chip(
        ui,
        cut_r,
        Id::unique("pcut"),
        "CUT",
        !app.sel_notes.is_empty(),
        ACCENT,
    )
    .clicked()
    {
        cut = true;
    }
    tx += 46.0;
    let paste_r = Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(54.0, th));
    if chip(
        ui,
        paste_r,
        Id::unique("ppaste"),
        "PASTE",
        !app.note_clipboard.is_empty(),
        MINT,
    )
    .clicked()
    {
        paste = true;
    }
    tx += 60.0;
    let dup_r = Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(44.0, th));
    if chip(
        ui,
        dup_r,
        Id::unique("pdup"),
        "DUP",
        !app.sel_notes.is_empty(),
        BLUE,
    )
    .clicked()
    {
        duplicate = true;
    }
    tx += 50.0;
    let clear_r = Rect::from_min_size(Pos2::new(tx, ty), Vec2::new(50.0, th));
    if chip(ui, clear_r, Id::unique("pclear"), "CLEAR", false, DIM).clicked() {
        select_only = None;
        toggle_sel = None;
        app.clear_note_selection();
    }

    // Grid geometry
    let keyboard_w = if drum { 104.0 } else { 66.0 };
    let bottom_h = 31.0;
    let grid_top = tools.bottom() + 6.0;
    let grid_bottom = rect.bottom() - bottom_h;
    let row_pitches: Vec<u8> = if drum {
        drum_rows()
    } else {
        let (mut lo, mut hi) = clip.note_bounds();
        if (hi - lo) < 24 {
            let mid = ((lo as i32 + hi as i32) / 2) as u8;
            lo = mid.saturating_sub(12);
            hi = (mid as u16 + 12).min(108) as u8;
        }
        lo = lo.max(12);
        hi = hi.min(108);
        (lo..=hi).rev().collect()
    };
    let rows = row_pitches.len().max(1) as f32;
    let row_h = if drum {
        ((grid_bottom - grid_top - 4.0) / rows).clamp(10.0, 26.0)
    } else {
        ((grid_bottom - grid_top - 4.0) / rows).clamp(7.0, 17.0)
    };
    let grid_h = rows * row_h;
    let top_y = grid_top + ((grid_bottom - grid_top) - grid_h) * 0.5;
    let x0 = rect.left() + keyboard_w;
    let grid_w = (rect.right() - 14.0 - x0).max(100.0);
    let ppb = grid_w as f64 / clip.length.max(0.25);
    let scale_snap = app.song.scale_snap && !drum;

    // Background rows and keyboard labels.
    for (ri, pitch) in row_pitches.iter().enumerate() {
        let y = top_y + ri as f32 * row_h;
        if y > grid_bottom {
            break;
        }
        let black = !drum && matches!(pitch % 12, 1 | 3 | 6 | 8 | 10);
        let in_scale = !drum && midi_fx::scale_contains(root, scale_mode, *pitch as i32);
        let root_row = !drum && (*pitch as i32 % 12) == root;
        let mut row_bg = if black {
            Color32::from_rgb(13, 15, 19)
        } else {
            Color32::from_rgb(18, 20, 25)
        };
        if in_scale {
            row_bg = mix(row_bg, ACCENT, if root_row { 0.10 } else { 0.035 });
        }
        let row_r = Rect::from_min_size(
            Pos2::new(rect.left(), y),
            Vec2::new(rect.right() - rect.left() - 8.0, row_h),
        );
        p.rect_filled(row_r, 0.0, row_bg);

        let key = Rect::from_min_size(
            Pos2::new(rect.left() + 8.0, y),
            Vec2::new(keyboard_w - 14.0, row_h),
        );
        let key_bg = if drum {
            if ri % 2 == 0 {
                Color32::from_rgb(36, 40, 48)
            } else {
                Color32::from_rgb(30, 33, 40)
            }
        } else if black {
            Color32::from_rgb(28, 31, 38)
        } else {
            Color32::from_rgb(64, 68, 78)
        };
        p.rect_filled(key, 0.0, key_bg);

        if drum {
            let lane = devices::drum_lane(*pitch);
            let label = samples::drum_lane_label(kit, lane);
            txt(
                &p,
                Pos2::new(key.left() + 4.0, key.center().y),
                Align2::LEFT_CENTER,
                fit(
                    &format!("{:02} {}", lane + 1, label),
                    key.width() - 4.0,
                    7.5,
                ),
                7.5,
                if matches!(lane, 0..=4) { TEXT } else { DIM },
            );
        } else {
            let name = format!(
                "{}{}",
                NOTE_NAMES[(pitch % 12) as usize],
                *pitch as i32 / 12 - 1
            );
            txt(
                &p,
                Pos2::new(key.left() + 4.0, key.center().y),
                Align2::LEFT_CENTER,
                name,
                7.0,
                if root_row {
                    ACCENT_DARK
                } else if black {
                    FAINT
                } else {
                    BG
                },
            );
        }
        hline(
            &p,
            rect.left() + 8.0,
            x0 + grid_w,
            y,
            Color32::from_rgb(8, 9, 12),
        );
    }

    // Vertical grid and bar numbers.
    let steps = (clip.length / 0.25).ceil() as i32;
    for s in 0..=steps {
        let beat = s as f64 * 0.25;
        let x = x0 + (beat * ppb) as f32;
        if x > x0 + grid_w {
            break;
        }
        let is_bar = (s % 16) == 0;
        let is_beat = (s % 4) == 0;
        vline(
            &p,
            x,
            top_y,
            top_y + grid_h,
            if is_bar {
                Color32::from_rgb(58, 63, 74)
            } else if is_beat {
                Color32::from_rgb(38, 42, 50)
            } else {
                Color32::from_rgb(28, 31, 38)
            },
        );
        if is_bar {
            mono(
                &p,
                Pos2::new(x + 4.0, top_y - 2.0),
                Align2::LEFT_BOTTOM,
                format!("{}", s / 16 + 1),
                7.5,
                FAINT,
            );
        }
    }
    vline(&p, x0 + grid_w, top_y, top_y + grid_h, alpha(col, 160));

    // Click empty grid to draw; pitch follows the row under the pointer.
    let grid_rect = Rect::from_min_size(Pos2::new(x0, top_y), Vec2::new(grid_w, grid_h));
    let gresp = ui.interact(
        grid_rect,
        Id::unique(("pgrid", cr.track, cr.scene)),
        Sense::click(),
    );
    if gresp.clicked() {
        if let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
            let beat = ((pos.x - x0) as f64 / ppb).max(0.0);
            let start = snap(beat, 0.25).min((clip.length - 0.25).max(0.0));
            let row = ((pos.y - top_y) / row_h).floor().clamp(0.0, rows - 1.0) as usize;
            if let Some(mut pitch) = row_pitches.get(row).copied() {
                if scale_snap {
                    pitch = midi_fx::nearest_scale_pitch(root, scale_mode, pitch as i32)
                        .clamp(0, 127) as u8;
                }
                add_note = Some(Note::new(
                    start,
                    app.note_len.min(clip.length.max(0.125)),
                    pitch,
                    app.note_vel,
                ));
                cursor_set = Some(start);
                audition = Some((pitch, app.note_vel));
            }
        }
    }

    // Playhead and paste cursor.
    let playing = app.shared.is_playing();
    let beat = app.shared.beat();
    let local = if cr.scene >= 0 {
        beat % clip.length.max(0.25)
    } else {
        beat - clip.start
    };
    if playing && local >= 0.0 && local <= clip.length {
        let x = x0 + (local * ppb) as f32;
        p.line_segment(
            [Pos2::new(x, top_y), Pos2::new(x, top_y + grid_h)],
            egui::Stroke::new(1.0, ACCENT),
        );
    }
    let cursor_x = x0 + (app.piano_cursor.max(0.0) * ppb) as f32;
    if cursor_x >= x0 && cursor_x <= x0 + grid_w && !app.note_clipboard.is_empty() {
        p.line_segment(
            [
                Pos2::new(cursor_x, top_y),
                Pos2::new(cursor_x, top_y + grid_h),
            ],
            egui::Stroke::new(1.0, alpha(MINT, 120)),
        );
    }

    // Notes
    for (i, note) in clip.notes.iter().enumerate() {
        let Some(row) = row_pitches.iter().position(|pitch| *pitch == note.pitch) else {
            continue;
        };
        let nx = x0 + (note.start * ppb) as f32;
        let nw = ((note.len * ppb) as f32).max(4.0);
        let ny = top_y + row as f32 * row_h;
        let nr = Rect::from_min_size(
            Pos2::new(nx, ny + 0.5),
            Vec2::new(nw, (row_h - 1.5).max(3.0)),
        );
        let is_sel = app.sel_notes.contains(&i);
        let is_anchor = app.sel_anchor == Some(i);
        let mut fillc = mix(col, BG, 0.25 + 0.45 * (1.0 - note.vel));
        if scale_snap && !midi_fx::scale_contains(root, scale_mode, note.pitch as i32) {
            fillc = mix(fillc, BG, 0.45);
        }
        rounded(&p, nr, 2.0, fillc);
        if is_sel {
            outline(
                &p,
                nr,
                2.0,
                if is_anchor { Color32::WHITE } else { MINT },
                if is_anchor { 1.8 } else { 1.2 },
            );
        } else {
            outline(&p, nr, 2.0, alpha(BG, 120), 1.0);
        }
        if nw > 26.0 && row_h > 9.0 {
            mono(
                &p,
                Pos2::new(nr.left() + 4.0, nr.center().y),
                Align2::LEFT_CENTER,
                format!("{}", note.pitch),
                7.0,
                alpha(BG, 210),
            );
        }
        let resp = ui.interact(
            nr,
            Id::unique(("pnote", cr.track, cr.scene, cr.start as i64, i)),
            Sense::click_and_drag(),
        );
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            if ui.ctx().input(|i| i.modifiers.shift) {
                toggle_sel = Some(i);
            } else {
                select_only = Some(i);
            }
            cursor_set = Some(note.start);
            audition = Some((note.pitch, note.vel));
        }
        if resp.double_clicked() {
            delete_notes.insert(i);
        }
        if resp.secondary_clicked() {
            if app.sel_notes.contains(&i) {
                delete_notes.extend(app.sel_notes.iter().copied());
            } else {
                delete_notes.insert(i);
            }
        }
        if resp.drag_started() {
            if !app.sel_notes.contains(&i) {
                app.select_only_note(i);
            }
            app.sel_anchor = Some(i);
            app.note_drag_origins = app
                .sel_notes
                .iter()
                .filter_map(|idx| clip.notes.get(*idx).map(|note| (*idx, *note)))
                .collect();
            let mode = if let Some(pos) = ui.ctx().input(|i| i.pointer.interact_pos()) {
                if pos.x > nr.right() - 6.0 {
                    NoteDrag::Resize
                } else {
                    NoteDrag::Move
                }
            } else {
                NoteDrag::Move
            };
            app.drag = DragState::Note {
                clip: cr,
                index: i,
                mode,
                start_beat: note.start,
                start_len: note.len,
                start_pitch: note.pitch,
            };
        }
        if resp.dragged() {
            if let DragState::Note {
                clip: active,
                index,
                mode,
                start_beat,
                start_len,
                start_pitch,
            } = app.drag
            {
                if active == cr && index == i {
                    let delta = resp.drag_delta();
                    let dx = delta.x as f64 / ppb;
                    let dy = (delta.y / row_h).round() as i32;
                    match mode {
                        NoteDrag::Move => {
                            let new_start = snap(start_beat + dx, 0.25)
                                .clamp(0.0, (clip.length - 0.125).max(0.0));
                            let start_delta = new_start - start_beat;
                            let start_row = row_pitches
                                .iter()
                                .position(|pitch| *pitch == start_pitch)
                                .unwrap_or(0) as i32;
                            let new_row = (start_row + dy).clamp(0, rows as i32 - 1);
                            let row_shift = new_row - start_row;
                            for (idx, origin) in &app.note_drag_origins {
                                let mut moved = *origin;
                                moved.start = (origin.start + start_delta)
                                    .clamp(0.0, (clip.length - 0.125).max(0.0));
                                let origin_row = row_pitches
                                    .iter()
                                    .position(|pitch| *pitch == origin.pitch)
                                    .unwrap_or(start_row as usize)
                                    as i32;
                                let dest_row =
                                    (origin_row + row_shift).clamp(0, rows as i32 - 1) as usize;
                                moved.pitch = row_pitches[dest_row];
                                if scale_snap {
                                    moved.pitch = midi_fx::nearest_scale_pitch(
                                        root,
                                        scale_mode,
                                        moved.pitch as i32,
                                    )
                                    .clamp(0, 127)
                                        as u8;
                                }
                                edit_notes.push((*idx, moved));
                            }
                        }
                        NoteDrag::Resize => {
                            let mut resized = *note;
                            resized.len = snap(start_len + dx, 0.25)
                                .clamp(0.125, (clip.length - note.start).max(0.125));
                            edit_notes.push((i, resized));
                        }
                    }
                }
            }
        }
        if resp.drag_stopped() {
            app.drag = DragState::None;
            app.note_drag_origins.clear();
        }
    }

    // Bottom info bar.
    let bottom = Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - bottom_h), rect.max);
    fill(&p, bottom, PANEL);
    hline(&p, bottom.left(), bottom.right(), bottom.top(), LINE);
    txt(
        &p,
        Pos2::new(bottom.left() + 12.0, bottom.center().y),
        Align2::LEFT_CENTER,
        if app.sel_notes.len() > 1 {
            format!(
                "{} NOTES SELECTED · drag moves the group · Ctrl+C/V/D · DEL erases",
                app.sel_notes.len()
            )
        } else {
            "Click to draw · Shift+click multi-select · drag edge to resize · Ctrl+C/V/D · DEL erases"
                .to_string()
        },
        8.0,
        FAINT,
    );
    if let Some(i) = app.sel_anchor {
        if let Some(n) = clip.notes.get(i) {
            mono(
                &p,
                Pos2::new(bottom.right() - 14.0, bottom.center().y),
                Align2::RIGHT_CENTER,
                format!(
                    "NOTE {} · VEL {:.0}% · LEN {:.2}",
                    n.pitch,
                    n.vel * 100.0,
                    n.len
                ),
                8.5,
                alpha(col, 230),
            );
        }
    }

    // Apply edits.
    if let Some(note) = add_note {
        if let Some(c) = clip_at_mut(&mut app.song, &cr) {
            let mut note = note;
            note.len = note.len.min(c.length.max(0.125));
            c.notes.push(note);
            let idx = c.notes.len() - 1;
            app.select_only_note(idx);
            app.dirty = true;
        }
    }
    if !edit_notes.is_empty() {
        if let Some(c) = clip_at_mut(&mut app.song, &cr) {
            for (index, note) in edit_notes {
                if index < c.notes.len() {
                    c.notes[index] = note;
                }
            }
            app.dirty = true;
        }
    }
    if set_len.is_some() || set_vel.is_some() {
        let indices: Vec<usize> = app
            .sel_notes
            .iter()
            .copied()
            .filter(|i| *i < clip.notes.len())
            .collect();
        if !indices.is_empty() {
            if let Some(c) = clip_at_mut(&mut app.song, &cr) {
                for i in indices {
                    if let Some(n) = c.notes.get_mut(i) {
                        if let Some(len) = set_len {
                            n.len = len.min((c.length - n.start).max(0.125));
                        }
                        if let Some(vel) = set_vel {
                            n.vel = vel.clamp(0.05, 1.0);
                        }
                    }
                }
                app.dirty = true;
            }
        }
    }
    if quantize {
        let indices: Vec<usize> = if app.sel_notes.is_empty() {
            (0..clip.notes.len()).collect()
        } else {
            app.sel_notes
                .iter()
                .copied()
                .filter(|i| *i < clip.notes.len())
                .collect()
        };
        if let Some(c) = clip_at_mut(&mut app.song, &cr) {
            let len = c.length;
            for i in &indices {
                if let Some(n) = c.notes.get_mut(*i) {
                    n.start = snap(n.start, 0.25).clamp(0.0, (len - 0.125).max(0.0));
                    n.len = snap(n.len, 0.25).max(0.125).min(len - n.start);
                }
            }
            app.dirty = true;
        }
        app.flash(format!(
            "Quantized {} note{} to 1/16.",
            indices.len(),
            if indices.len() == 1 { "" } else { "s" }
        ));
    }
    if copy && app.copy_selection(false) {
        app.flash("Notes copied.");
    }
    if cut && app.copy_selection(true) {
        app.publish_now();
        app.flash("Notes cut.");
    }
    if paste && app.paste_clipboard() {
        app.publish_now();
        app.flash("Notes pasted at the cursor.");
    }
    if duplicate && app.duplicate_selection() {
        app.publish_now();
        app.flash("Notes duplicated.");
    }
    if let Some(i) = select_only {
        app.select_only_note(i);
    }
    if let Some(i) = toggle_sel {
        app.toggle_note_selection(i);
    }
    if !delete_notes.is_empty() {
        if let Some(c) = clip_at_mut(&mut app.song, &cr) {
            let mut indices: Vec<usize> = delete_notes.into_iter().collect();
            indices.sort_unstable();
            indices.retain(|i| *i < c.notes.len());
            for i in indices.iter().rev() {
                c.notes.remove(*i);
            }
            if !indices.is_empty() {
                app.clear_note_selection();
                app.dirty = true;
            }
        }
    }
    if let Some(cursor) = cursor_set {
        app.piano_cursor = cursor;
    }
    if let Some((pitch, vel)) = audition {
        app.audition_note(pitch, vel);
    }
    if back {
        app.close_clip();
    }
    if app.dirty {
        app.publish_now();
    }
}
