//! Session view: an Ableton-style clip launcher grid.

use crate::app::{Act, ClipRef, HarpsiApp};
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{self, Align2, Color32, Id, Pos2, Rect, Sense, Ui, Vec2};
use hs_core::params::DeviceKind;
use hs_core::song::launch_quantum_label;

pub fn show(app: &mut HarpsiApp, ui: &mut Ui, rect: Rect) {
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, BG);

    let scenes = app.song.scene_names();
    let n_tracks = app.song.tracks.len();
    let left_w = 138.0;
    let header_h = 78.0;
    let bar_h = 36.0;
    let selected_clip = app
        .clip_edit
        .and_then(|cr| crate::app::clip_at(&app.song, &cr).map(|clip| (cr, clip.clone())));
    // When the bottom detail dock is open it already shows the selected clip;
    // keep the grid taller instead of stacking two inspectors.
    let detail_h = if !app.detail_open() && selected_clip.is_some() {
        46.0
    } else {
        0.0
    };

    let cell_area = Rect::from_min_max(
        Pos2::new(rect.left() + left_w, rect.top() + header_h),
        Pos2::new(rect.right(), rect.bottom() - bar_h - detail_h),
    );

    let max_rows = ((cell_area.height() / 48.0).floor() as usize).max(4);
    let rows = scenes.len().min(max_rows).max(4);
    let cell_h = cell_area.height() / rows as f32;
    let avail_w = (rect.width() - left_w - 6.0).max(100.0);
    let col_w = (avail_w / n_tracks.max(1) as f32).clamp(88.0, 260.0);

    let mut acts: Vec<Act> = Vec::new();
    let mut select_track: Option<usize> = None;
    let mut mute_toggle: Option<usize> = None;
    let mut solo_toggle: Option<usize> = None;
    let mut select_clip: Option<ClipRef> = None;

    let launched = app.launched.clone();
    let playing = app.shared.is_playing();
    let session_states: Vec<Option<i32>> = (0..n_tracks)
        .map(|track| app.shared.session_scene(track))
        .collect();
    let pending_states: Vec<Option<i32>> = (0..n_tracks)
        .map(|track| app.shared.pending_scene(track))
        .collect();
    let sel_track = app.sel_track;

    // Header: scene column label
    let head_left = Rect::from_min_size(rect.min, Vec2::new(left_w, header_h));
    fill(&p, head_left, PANEL);
    txt(
        &p,
        Pos2::new(head_left.left() + 14.0, head_left.top() + 14.0),
        Align2::LEFT_TOP,
        "SESSION",
        9.0,
        ACCENT,
    );
    txt(
        &p,
        Pos2::new(head_left.left() + 14.0, head_left.top() + 30.0),
        Align2::LEFT_TOP,
        "SCENES",
        8.0,
        FAINT,
    );
    txt(
        &p,
        Pos2::new(head_left.left() + 14.0, head_left.bottom() - 16.0),
        Align2::LEFT_BOTTOM,
        fit(
            &format!(
                "Q {} · CLICK TO LAUNCH",
                launch_quantum_label(app.song.launch_quantum)
            ),
            head_left.width() - 22.0,
            7.5,
        ),
        7.5,
        DIM,
    );

    // Track headers
    for (ti, track) in app.song.tracks.iter().enumerate() {
        let x = rect.left() + left_w + ti as f32 * col_w;
        if x >= rect.right() {
            break;
        }
        let col = c(track.color);
        let hr = Rect::from_min_size(
            Pos2::new(x, rect.top()),
            Vec2::new((col_w - 6.0).max(60.0), header_h - 4.0),
        );
        let selected = ti == sel_track;
        fill(
            &p,
            hr,
            if selected {
                mix(PANEL, col, 0.10)
            } else {
                PANEL
            },
        );
        p.rect_filled(
            Rect::from_min_size(hr.min, Vec2::new(hr.width(), 3.0)),
            0.0,
            col,
        );
        if selected {
            outline(&p, hr, 0.0, col, 1.0);
        }
        let resp = ui.interact(hr, Id::unique(("trkh", ti)), Sense::click_and_drag());
        if resp.clicked() {
            select_track = Some(ti);
        }
        if resp.double_clicked() {
            select_track = Some(ti);
            app.view = crate::app::View::Device;
            app.device_open = true;
            app.dev_sel = crate::app::DevSel::Instrument;
        }
        mono(
            &p,
            Pos2::new(hr.left() + 9.0, hr.top() + 8.0),
            Align2::LEFT_TOP,
            format!("{:02}", ti + 1),
            8.0,
            alpha(col, 200),
        );
        txt(
            &p,
            Pos2::new(hr.left() + 28.0, hr.top() + 8.0),
            Align2::LEFT_TOP,
            fit(&track.name, hr.width() - 58.0, 12.0),
            12.0,
            TEXT,
        );
        // Live-style per-track stop button, always available in the header.
        let stop_r = Rect::from_min_size(
            Pos2::new(hr.right() - 22.0, hr.top() + 7.0),
            Vec2::new(16.0, 16.0),
        );
        let stop_resp = ui.interact(stop_r, Id::unique(("stoptrack", ti)), Sense::click());
        rounded(
            &p,
            stop_r,
            2.0,
            if stop_resp.hovered() {
                mix(PANEL3, RED, 0.25)
            } else {
                PANEL2
            },
        );
        outline(
            &p,
            stop_r,
            2.0,
            if stop_resp.hovered() { RED } else { LINE },
            1.0,
        );
        p.rect_filled(
            Rect::from_center_size(stop_r.center(), Vec2::splat(6.0)),
            1.0,
            if stop_resp.hovered() { RED } else { FAINT },
        );
        if stop_resp.clicked() {
            acts.push(Act::StopTrack(ti));
        }
        txt(
            &p,
            Pos2::new(hr.left() + 9.0, hr.top() + 27.0),
            Align2::LEFT_TOP,
            track.instrument.kind.name().to_uppercase(),
            7.5,
            alpha(col, 220),
        );
        // mute / solo
        let m = Rect::from_min_size(
            Pos2::new(hr.left() + 9.0, hr.top() + 44.0),
            Vec2::new(24.0, 17.0),
        );
        let s = Rect::from_min_size(
            Pos2::new(hr.left() + 37.0, hr.top() + 44.0),
            Vec2::new(24.0, 17.0),
        );
        if toggle_sq(ui, m, Id::unique(("mute", ti)), "M", track.mute, RED).clicked() {
            mute_toggle = Some(ti);
        }
        if toggle_sq(ui, s, Id::unique(("solo", ti)), "S", track.solo, ACCENT).clicked() {
            solo_toggle = Some(ti);
        }
        let d = Rect::from_min_size(
            Pos2::new(hr.right() - 38.0, hr.top() + 44.0),
            Vec2::new(32.0, 17.0),
        );
        if toggle_sq(ui, d, Id::unique(("dev", ti)), "DEV", false, MINT).clicked() {
            select_track = Some(ti);
            app.view = crate::app::View::Device;
            app.device_open = true;
            app.dev_sel = crate::app::DevSel::Instrument;
        }
    }

    // Scene rows: launch buttons + clip cells
    for row in 0..rows {
        let scene_label = scenes.get(row).map(|s| &s[..]).unwrap_or("");
        let y = cell_area.top() + row as f32 * cell_h;
        let row_r = Rect::from_min_max(
            Pos2::new(rect.left(), y),
            Pos2::new(rect.right(), (y + cell_h).min(cell_area.bottom())),
        );
        if row % 2 == 1 {
            fill(&p, row_r, Color32Ext::row());
        }
        hline(&p, rect.left(), rect.right(), y, LINE_SOFT);

        let scene_r = Rect::from_min_size(
            Pos2::new(rect.left() + 6.0, y + 3.0),
            Vec2::new(left_w - 12.0, (cell_h - 6.0).max(24.0)),
        );
        let scene_resp = ui.interact(scene_r, Id::unique(("scene", row)), Sense::click());
        if scene_resp.hovered() {
            rounded(&p, scene_r, 3.0, PANEL2);
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let triangle = if scene_resp.hovered() { MINT } else { FAINT };
        p.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(scene_r.left() + 10.0, scene_r.center().y - 5.0),
                Pos2::new(scene_r.left() + 17.0, scene_r.center().y),
                Pos2::new(scene_r.left() + 10.0, scene_r.center().y + 5.0),
            ],
            triangle,
            egui::Stroke::new(0.0, Color32::TRANSPARENT),
        ));
        txt(
            &p,
            Pos2::new(scene_r.left() + 23.0, scene_r.center().y),
            Align2::LEFT_CENTER,
            fit(scene_label, scene_r.width() - 30.0, 10.0),
            10.0,
            if scene_resp.hovered() { TEXT } else { DIM },
        );
        if scene_resp.clicked() {
            for (ti, t) in app.song.tracks.iter().enumerate() {
                if t.session_clip(row as i32).is_some() {
                    acts.push(Act::Launch {
                        track: ti,
                        scene: row as i32,
                    });
                }
            }
        }

        for (ti, track) in app.song.tracks.iter().enumerate() {
            let x = rect.left() + left_w + ti as f32 * col_w;
            if x >= rect.right() {
                break;
            }
            let cr = Rect::from_min_size(
                Pos2::new(x + 3.0, y + 3.0),
                Vec2::new((col_w - 9.0).max(50.0), (cell_h - 7.0).min(120.0)),
            );
            let col = c(track.color);
            let clip = track.session_clip(row as i32);
            let cref = ClipRef {
                track: ti,
                scene: row as i32,
                start: 0.0,
            };
            let is_sel = app.clip_edit.map(|s| s == cref).unwrap_or(false);
            let shared_active = session_states.get(ti).copied().flatten();
            let requested = launched.get(ti).copied().flatten();
            // Once the engine is running it is the source of truth, so a
            // follow-action STOP or a track stop clears the grid immediately.
            // The optimistic request is only used for auditioning without audio.
            let active_scene = if app.audio.is_some() {
                shared_active
            } else {
                requested
            };
            let is_playing = playing && active_scene == Some(row as i32);
            let is_queued = pending_states.get(ti).copied().flatten() == Some(row as i32)
                || (playing
                    && requested == Some(row as i32)
                    && shared_active.is_some()
                    && shared_active != Some(row as i32));

            let resp = ui.interact(cr, Id::unique(("cell", ti, row)), Sense::click_and_drag());
            if let Some(clip) = clip {
                let bg = mix(PANEL2, col, if resp.hovered() { 0.30 } else { 0.20 });
                rounded(&p, cr, 3.0, bg);
                if is_queued {
                    p.rect_filled(cr, 3.0, alpha(ACCENT, 14));
                }
                outline(
                    &p,
                    cr,
                    3.0,
                    if is_queued || is_sel {
                        ACCENT
                    } else {
                        mix(PANEL, col, 0.55)
                    },
                    if is_queued || is_sel { 1.5 } else { 1.0 },
                );
                if is_playing || is_queued {
                    p.rect_filled(
                        Rect::from_min_size(cr.min, Vec2::new(3.0, cr.height())),
                        1.0,
                        if is_playing { MINT } else { ACCENT },
                    );
                }
                txt(
                    &p,
                    Pos2::new(cr.left() + 10.0, cr.top() + 6.0),
                    Align2::LEFT_TOP,
                    fit(&clip.name, cr.width() - 26.0, 10.0),
                    10.0,
                    TEXT,
                );
                mono(
                    &p,
                    Pos2::new(cr.right() - 7.0, cr.top() + 6.0),
                    Align2::RIGHT_TOP,
                    format!("{:.0}", clip.length / 4.0),
                    8.0,
                    alpha(col, 220),
                );
                let notes_r = Rect::from_min_max(
                    Pos2::new(cr.left() + 8.0, cr.top() + 24.0),
                    Pos2::new(cr.right() - 7.0, cr.bottom() - 7.0),
                );
                draw_clip_notes(&p, notes_r, clip, alpha(col, 235));
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if is_playing || is_queued {
                    rounded(
                        &p,
                        Rect::from_min_size(
                            Pos2::new(cr.left() + 3.0, cr.bottom() - 15.0),
                            Vec2::new(48.0, 12.0),
                        ),
                        2.0,
                        alpha(BG, 210),
                    );
                    txt(
                        &p,
                        Pos2::new(cr.left() + 8.0, cr.bottom() - 9.0),
                        Align2::LEFT_CENTER,
                        if is_playing {
                            "▶ PLAYING"
                        } else {
                            "◷ QUEUED"
                        },
                        6.5,
                        if is_playing { MINT } else { ACCENT },
                    );
                } else if resp.hovered() {
                    rounded(
                        &p,
                        Rect::from_min_size(
                            Pos2::new(cr.left() + 3.0, cr.bottom() - 15.0),
                            Vec2::new(30.0, 12.0),
                        ),
                        2.0,
                        alpha(BG, 200),
                    );
                    txt(
                        &p,
                        Pos2::new(cr.left() + 8.0, cr.bottom() - 9.0),
                        Align2::LEFT_CENTER,
                        "LAUNCH",
                        6.5,
                        MINT,
                    );
                }
                if resp.double_clicked() {
                    select_clip = Some(cref);
                    acts.push(Act::OpenClip(cref));
                } else if resp.clicked() {
                    select_clip = Some(cref);
                    acts.push(Act::Launch {
                        track: ti,
                        scene: row as i32,
                    });
                }
            } else {
                rounded(
                    &p,
                    cr,
                    3.0,
                    if resp.hovered() {
                        PANEL2
                    } else {
                        Color32Ext::empty()
                    },
                );
                if resp.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    p.add(egui::Shape::convex_polygon(
                        vec![
                            Pos2::new(cr.center().x - 5.0, cr.top() + 12.0),
                            Pos2::new(cr.center().x + 6.0, cr.top() + 18.0),
                            Pos2::new(cr.center().x - 5.0, cr.top() + 24.0),
                        ],
                        alpha(MINT, 180),
                        egui::Stroke::new(0.0, Color32::TRANSPARENT),
                    ));
                    txt(
                        &p,
                        Pos2::new(cr.center().x, cr.top() + 32.0),
                        Align2::CENTER_TOP,
                        "NEW CLIP",
                        8.0,
                        DIM,
                    );
                } else {
                    txt(
                        &p,
                        Pos2::new(cr.left() + 10.0, cr.top() + 7.0),
                        Align2::LEFT_TOP,
                        "＋",
                        10.0,
                        alpha(FAINT, 140),
                    );
                }
                if resp.clicked() && clip.is_none() {
                    acts.push(Act::AddSessionClip {
                        track: ti,
                        scene: row as i32,
                    });
                    acts.push(Act::Launch {
                        track: ti,
                        scene: row as i32,
                    });
                    select_clip = Some(cref);
                }
            }
        }
    }

    // Clip inspector: the selected session clip exposes Live-style follow and
    // loop controls without leaving the grid. It yields to the bottom detail
    // dock when Clip/Device is open.
    if let Some((cr, clip)) = selected_clip.as_ref().filter(|_| !app.detail_open()) {
        let detail_top = rect.bottom() - bar_h - detail_h;
        let detail = Rect::from_min_max(
            Pos2::new(rect.left(), detail_top),
            Pos2::new(rect.right(), rect.bottom() - bar_h),
        );
        let col = app
            .song
            .tracks
            .get(cr.track)
            .map(|t| c(t.color))
            .unwrap_or(ACCENT);
        fill(&p, detail, PANEL);
        hline(&p, detail.left(), detail.right(), detail.top(), LINE);
        p.rect_filled(
            Rect::from_min_size(detail.min, Vec2::new(3.0, detail.height())),
            0.0,
            col,
        );
        txt(
            &p,
            Pos2::new(detail.left() + 12.0, detail.top() + 7.0),
            Align2::LEFT_TOP,
            if cr.scene >= 0 {
                "SESSION CLIP"
            } else {
                "ARRANGEMENT CLIP"
            },
            7.5,
            col,
        );
        txt(
            &p,
            Pos2::new(detail.left() + 12.0, detail.top() + 20.0),
            Align2::LEFT_TOP,
            fit(&clip.name, 188.0, 12.0),
            12.0,
            TEXT,
        );
        mono(
            &p,
            Pos2::new(detail.left() + 210.0, detail.top() + 23.0),
            Align2::LEFT_TOP,
            format!("{:.0} BARS", clip.length / 4.0),
            8.0,
            alpha(col, 230),
        );

        if cr.scene >= 0 {
            let mut dx = detail.left() + 286.0;
            txt(
                &p,
                Pos2::new(dx, detail.center().y),
                Align2::LEFT_CENTER,
                "FOLLOW",
                7.5,
                FAINT,
            );
            dx += 44.0;
            let follow_r =
                Rect::from_min_size(Pos2::new(dx, detail.top() + 12.0), Vec2::new(68.0, 22.0));
            if chip(
                ui,
                follow_r,
                Id::unique("clipfollow"),
                clip.follow_label(),
                clip.follow_action != 0,
                ACCENT,
            )
            .clicked()
            {
                acts.push(Act::CycleClipFollow(*cr));
            }
            dx += 76.0;
            txt(
                &p,
                Pos2::new(dx, detail.center().y),
                Align2::LEFT_CENTER,
                "AFTER",
                7.5,
                FAINT,
            );
            dx += 38.0;
            let after_r =
                Rect::from_min_size(Pos2::new(dx, detail.top() + 12.0), Vec2::new(70.0, 22.0));
            if chip(
                ui,
                after_r,
                Id::unique("clipfollowafter"),
                &hs_core::song::follow_after_label(clip.follow_after),
                clip.follow_action != 0,
                MINT,
            )
            .clicked()
            {
                acts.push(Act::CycleClipFollowAfter(*cr));
            }
            dx += 78.0;
            let loop_r =
                Rect::from_min_size(Pos2::new(dx, detail.top() + 12.0), Vec2::new(56.0, 22.0));
            if chip(
                ui,
                loop_r,
                Id::unique("cliploadingloop"),
                "LOOP",
                clip.looping,
                BLUE,
            )
            .clicked()
            {
                acts.push(Act::ToggleClipLoop(*cr));
            }
            txt(
                &p,
                Pos2::new(detail.right() - 14.0, detail.center().y),
                Align2::RIGHT_CENTER,
                format!("SCENE {} · FOLLOW ACTIONS", cr.scene + 1),
                8.0,
                FAINT,
            );
        } else {
            txt(
                &p,
                Pos2::new(detail.left() + 286.0, detail.center().y),
                Align2::LEFT_CENTER,
                "Drag the clip in Arrangement · double-click to edit notes · follow actions apply to session clips",
                8.0,
                FAINT,
            );
        }
    }

    // Bottom action bar
    let bar = Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - bar_h), rect.max);
    fill(&p, bar, PANEL);
    hline(&p, bar.left(), bar.right(), bar.top(), LINE);
    let by = bar.top() + 5.0;
    let bh = 24.0;
    let mut bx = bar.left() + 12.0;

    let add_scene = Rect::from_min_size(Pos2::new(bx, by), Vec2::new(84.0, bh));
    if chip(
        ui,
        add_scene,
        Id::unique("addscene"),
        "＋ SCENE",
        false,
        ACCENT,
    )
    .clicked()
    {
        acts.push(Act::AddScene);
    }
    bx += 92.0;
    let stop = Rect::from_min_size(Pos2::new(bx, by), Vec2::new(96.0, bh));
    if chip(
        ui,
        stop,
        Id::unique("stopclips"),
        "■ STOP CLIPS",
        false,
        RED,
    )
    .clicked()
    {
        acts.push(Act::StopClips);
    }
    bx += 104.0;
    let add_clip = Rect::from_min_size(Pos2::new(bx, by), Vec2::new(86.0, bh));
    if chip(ui, add_clip, Id::unique("addclip2"), "＋ CLIP", false, MINT).clicked() {
        acts.push(Act::AddSessionClip {
            track: sel_track,
            scene: app.sel_scene,
        });
    }
    bx += 94.0;

    // One-click starting points for original parts. The vocal action opens the
    // same WAV import path as VOICE → BASS so a real take becomes a track.
    let parts = [
        ("＋ DRUMS", DeviceKind::DrumRack, 74.0),
        ("＋ MELODY", DeviceKind::Salamander, 78.0),
        ("＋ BASS", DeviceKind::SubBass, 64.0),
    ];
    for (i, (label, kind, width)) in parts.into_iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(bx, by), Vec2::new(width, bh));
        if chip(ui, r, Id::unique(("newpart", i)), label, false, MINT).clicked() {
            acts.push(Act::CreatePart(kind));
        }
        bx += width + 6.0;
    }
    let vocal = Rect::from_min_size(Pos2::new(bx, by), Vec2::new(62.0, bh));
    if chip(ui, vocal, Id::unique("newvocal"), "＋ VOCAL", false, BLUE).clicked() {
        acts.push(Act::ImportVocal(app.vocal_style));
    }
    bx += 68.0;

    let sel_label = app
        .clip_edit
        .and_then(|cr| crate::app::clip_at(&app.song, &cr).map(|c| (cr, c.name.clone())))
        .map(|(_, n)| n);
    if let Some(name) = &sel_label {
        txt(
            &p,
            Pos2::new(bx + 4.0, bar.center().y),
            Align2::LEFT_CENTER,
            format!("SELECTED: {}", fit(name, 126.0, 9.0)),
            9.0,
            DIM,
        );
        let edit = Rect::from_min_size(Pos2::new(bar.right() - 172.0, by), Vec2::new(76.0, bh));
        if chip(
            ui,
            edit,
            Id::unique("editclip"),
            "EDIT NOTES",
            false,
            ACCENT,
        )
        .clicked()
        {
            if let Some(cr) = app.clip_edit {
                acts.push(Act::OpenClip(cr));
            }
        }
        let del = Rect::from_min_size(Pos2::new(bar.right() - 90.0, by), Vec2::new(78.0, bh));
        if chip(ui, del, Id::unique("delclip"), "DELETE", false, RED).clicked() {
            if let Some(cr) = app.clip_edit {
                acts.push(Act::DeleteClip(cr));
            }
        }
    } else {
        txt(
            &p,
            Pos2::new(bx + 4.0, bar.center().y),
            Align2::LEFT_CENTER,
            "Double-click a clip to edit its notes.",
            8.5,
            FAINT,
        );
    }

    if let Some(ti) = select_track {
        app.sel_track = ti;
        app.sel_scene = app.sel_scene.clamp(0, scenes.len() as i32 - 1);
        if let Some(t) = app.song.tracks.get(ti) {
            app.dev_sel = crate::app::DevSel::Instrument;
            let _ = t;
        }
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
    for a in acts {
        app.act(a);
    }
}

/// Local helper for subtle grid backgrounds.
struct Color32Ext;
impl Color32Ext {
    fn row() -> egui::Color32 {
        egui::Color32::from_rgb(13, 15, 19)
    }
    fn empty() -> egui::Color32 {
        egui::Color32::from_rgb(16, 18, 22)
    }
}
