//! Mixer view: channel strips with pan, send, fader, meter, mute/solo,
//! plus the reverb return and the master section.

use crate::app::HarpsiApp;
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{Align2, Id, Pos2, Rect, Sense, Ui, Vec2};
use hs_core::engine::MAX_TRACKS;

pub fn show(app: &mut HarpsiApp, ui: &mut Ui, rect: Rect) {
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, BG);

    let master_w = 128.0;
    let return_w = 128.0;
    let right_reserve = master_w + return_w + 30.0;
    let strip_w = 94.0;
    let gap = 7.0;
    let top = rect.top() + 12.0;
    let bottom = rect.bottom() - 12.0;

    let mut select: Option<usize> = None;
    let tracks_n = app.song.tracks.len();
    let avail = (rect.width() - right_reserve - 20.0).max(100.0);
    let fits = ((avail + gap) / (strip_w + gap)).floor().max(1.0) as usize;
    let shown = tracks_n.min(fits);

    let playing = app.shared.is_playing();

    for i in 0..shown {
        let x = rect.left() + 14.0 + i as f32 * (strip_w + gap);
        let strip = Rect::from_min_max(Pos2::new(x, top), Pos2::new(x + strip_w, bottom));
        let selected = i == app.sel_track;
        let t = &mut app.song.tracks[i];
        let col = c(t.color);
        fill(&p, strip, PANEL);
        outline(
            &p,
            strip,
            2.0,
            if selected { alpha(col, 180) } else { LINE_SOFT },
            if selected { 1.2 } else { 1.0 },
        );
        p.rect_filled(
            Rect::from_min_size(strip.min, Vec2::new(strip.width(), 3.0)),
            0.0,
            col,
        );
        let click = ui.interact(strip, Id::unique(("mstrip", i)), Sense::click());
        if click.clicked() {
            select = Some(i);
        }

        txt(
            &p,
            Pos2::new(strip.center().x, strip.top() + 12.0),
            Align2::CENTER_TOP,
            fit(&t.name, strip.width() - 10.0, 10.5),
            10.5,
            TEXT,
        );
        txt(
            &p,
            Pos2::new(strip.center().x, strip.top() + 27.0),
            Align2::CENTER_TOP,
            t.instrument.kind.short(),
            7.5,
            alpha(col, 225),
        );

        // Pan
        let pan_r = Rect::from_min_size(
            Pos2::new(strip.left() + 12.0, strip.top() + 41.0),
            Vec2::new(strip.width() - 24.0, 64.0),
        );
        let mut pan = t.pan;
        let pan_disp = if pan.abs() < 0.01 {
            "C".to_string()
        } else {
            format!(
                "{:.0}{}",
                pan.abs() * 100.0,
                if pan < 0.0 { "L" } else { "R" }
            )
        };
        let pan_spec = KnobSpec {
            def: 0.0,
            min: -1.0,
            max: 1.0,
            step: 0.0,
            bipolar: true,
            label: "PAN",
            accent: col,
        };
        if knob(
            ui,
            pan_r,
            Id::unique(("pan", i)),
            &mut pan,
            &pan_disp,
            &pan_spec,
        ) {
            t.pan = pan;
            app.dirty = true;
        }

        // Send
        let send_r = Rect::from_min_size(
            Pos2::new(strip.left() + 12.0, strip.top() + 106.0),
            Vec2::new(strip.width() - 24.0, 60.0),
        );
        let mut send = t.send;
        let send_disp = format!("{:.0}%", send * 100.0);
        let send_spec = KnobSpec {
            def: 0.12,
            min: 0.0,
            max: 1.0,
            step: 0.0,
            bipolar: false,
            label: "SEND",
            accent: MINT,
        };
        if knob(
            ui,
            send_r,
            Id::unique(("send", i)),
            &mut send,
            &send_disp,
            &send_spec,
        ) {
            t.send = send;
            app.dirty = true;
        }

        // Fader + meter
        let fader_top = strip.top() + 176.0;
        let fader_bottom = strip.bottom() - 60.0;
        let fr = Rect::from_min_size(
            Pos2::new(strip.left() + 18.0, fader_top),
            Vec2::new(40.0, (fader_bottom - fader_top).max(40.0)),
        );
        let mut vol = t.volume;
        if fader(ui, fr, Id::unique(("fader", i)), &mut vol, 0.8, 1.3, col) {
            t.volume = vol;
            app.dirty = true;
        }
        let mr = Rect::from_min_size(
            Pos2::new(strip.right() - 22.0, fader_top),
            Vec2::new(8.0, (fader_bottom - fader_top).max(40.0)),
        );
        let level = app.shared.level(i.min(MAX_TRACKS - 1));
        meter(&p, mr, if t.mute { 0.0 } else { level }, true);

        mono(
            &p,
            Pos2::new(strip.center().x, strip.bottom() - 48.0),
            Align2::CENTER_TOP,
            format!("{} dB", db_text(t.volume)),
            8.5,
            DIM,
        );
        let m = Rect::from_min_size(
            Pos2::new(strip.left() + 12.0, strip.bottom() - 30.0),
            Vec2::new(29.0, 19.0),
        );
        let s = Rect::from_min_size(
            Pos2::new(strip.left() + 46.0, strip.bottom() - 30.0),
            Vec2::new(29.0, 19.0),
        );
        if toggle_sq(ui, m, Id::unique(("mm", i)), "MUTE", t.mute, RED).clicked() {
            t.mute = !t.mute;
            app.dirty = true;
        }
        if toggle_sq(ui, s, Id::unique(("ms", i)), "SOLO", t.solo, ACCENT).clicked() {
            t.solo = !t.solo;
            app.dirty = true;
        }
        if playing {
            p.circle_filled(
                Pos2::new(strip.left() + 8.0, strip.top() + 8.0),
                2.0,
                alpha(MINT, 180),
            );
        }
    }

    if tracks_n > shown {
        txt(
            &p,
            Pos2::new(
                rect.left() + 14.0 + shown as f32 * (strip_w + gap) + 4.0,
                rect.center().y,
            ),
            Align2::LEFT_CENTER,
            format!("+{}", tracks_n - shown),
            9.0,
            FAINT,
        );
    }

    // Return and master follow the visible track strips, as they do in Live.
    // They only fall back to the right edge when a wide session fills the view.
    let ret_left = (rect.left() + 14.0 + shown as f32 * (strip_w + gap) + 16.0)
        .min(rect.right() - master_w - return_w - 30.0);
    let ret_r = Rect::from_min_max(
        Pos2::new(ret_left, top),
        Pos2::new(ret_left + return_w, bottom),
    );
    fill(&p, ret_r, PANEL);
    outline(&p, ret_r, 2.0, LINE_SOFT, 1.0);
    p.rect_filled(
        Rect::from_min_size(ret_r.min, Vec2::new(ret_r.width(), 3.0)),
        0.0,
        BLUE,
    );
    txt(
        &p,
        Pos2::new(ret_r.center().x, ret_r.top() + 12.0),
        Align2::CENTER_TOP,
        "RETURN A · REVERB",
        9.0,
        TEXT,
    );
    txt(
        &p,
        Pos2::new(ret_r.center().x, ret_r.top() + 26.0),
        Align2::CENTER_TOP,
        "SEND BUS",
        7.5,
        FAINT,
    );
    let rfx = &mut app.song.return_fx;
    let defs = rfx.defs();
    let cols = 2usize;
    let kw = (ret_r.width() - 24.0) / cols as f32;
    for (i, def) in defs.iter().enumerate().take(4) {
        let cx = ret_r.left() + 10.0 + (i % cols) as f32 * kw;
        let cy = ret_r.top() + 48.0 + (i / cols) as f32 * 82.0;
        let kr = Rect::from_min_size(Pos2::new(cx, cy), Vec2::new(kw - 4.0, 76.0));
        let mut v = rfx.p(i);
        let disp = def.format(v);
        let spec = KnobSpec {
            def: def.def,
            min: def.min,
            max: def.max,
            step: def.step,
            bipolar: def.bipolar,
            label: def.name,
            accent: BLUE,
        };
        if knob(ui, kr, Id::unique(("ret", i)), &mut v, &disp, &spec) {
            rfx.set(i, v);
            app.dirty = true;
        }
    }
    txt(
        &p,
        Pos2::new(ret_r.center().x, ret_r.bottom() - 36.0),
        Align2::CENTER_BOTTOM,
        "Raise a track's SEND",
        8.0,
        FAINT,
    );
    txt(
        &p,
        Pos2::new(ret_r.center().x, ret_r.bottom() - 22.0),
        Align2::CENTER_BOTTOM,
        "to feed this reverb.",
        8.0,
        FAINT,
    );

    // Master strip
    let mast_left = ret_r.right() + 14.0;
    let mast = Rect::from_min_max(
        Pos2::new(mast_left, top),
        Pos2::new(mast_left + master_w, bottom),
    );
    fill(&p, mast, PANEL);
    outline(&p, mast, 2.0, ACCENT_DARK, 1.0);
    p.rect_filled(
        Rect::from_min_size(mast.min, Vec2::new(mast.width(), 3.0)),
        0.0,
        ACCENT,
    );
    txt(
        &p,
        Pos2::new(mast.center().x, mast.top() + 12.0),
        Align2::CENTER_TOP,
        "MASTER",
        10.0,
        TEXT,
    );
    let cpu = app.shared.cpu() * 100.0;
    txt(
        &p,
        Pos2::new(mast.center().x, mast.top() + 27.0),
        Align2::CENTER_TOP,
        format!("CPU {:.0}%", cpu),
        7.5,
        if cpu > 80.0 { RED } else { FAINT },
    );

    let mut mv = app.song.master_volume;
    let mv_disp = format!("{} dB", db_text(mv));
    let mvol = Rect::from_min_size(
        Pos2::new(mast.left() + 14.0, mast.top() + 44.0),
        Vec2::new(mast.width() - 28.0, 64.0),
    );
    let mv_spec = KnobSpec {
        def: 0.85,
        min: 0.0,
        max: 1.3,
        step: 0.0,
        bipolar: false,
        label: "VOLUME",
        accent: ACCENT,
    };
    if knob(ui, mvol, Id::unique("mvol"), &mut mv, &mv_disp, &mv_spec) {
        app.song.master_volume = mv;
        app.dirty = true;
    }

    let fader_top = mast.top() + 124.0;
    let fader_bottom = mast.bottom() - 96.0;
    let fr = Rect::from_min_size(
        Pos2::new(mast.left() + 16.0, fader_top),
        Vec2::new(42.0, (fader_bottom - fader_top).max(50.0)),
    );
    let mut mv2 = app.song.master_volume;
    if fader(ui, fr, Id::unique("mfader"), &mut mv2, 0.85, 1.3, ACCENT) {
        app.song.master_volume = mv2;
        app.dirty = true;
    }
    meter(
        &p,
        Rect::from_min_size(
            Pos2::new(mast.right() - 24.0, fader_top),
            Vec2::new(9.0, (fader_bottom - fader_top).max(50.0)),
        ),
        app.shared.level(MAX_TRACKS),
        true,
    );
    mono(
        &p,
        Pos2::new(mast.center().x, fader_bottom + 4.0),
        Align2::CENTER_TOP,
        format!("{} dB", db_text(app.song.master_volume)),
        9.0,
        TEXT,
    );

    let lim = Rect::from_min_size(
        Pos2::new(mast.left() + 12.0, mast.bottom() - 78.0),
        Vec2::new(mast.width() - 24.0, 22.0),
    );
    if toggle_sq(
        ui,
        lim,
        Id::unique("mlim"),
        "LIMITER",
        app.song.limiter,
        MINT,
    )
    .clicked()
    {
        app.song.limiter = !app.song.limiter;
        app.dirty = true;
    }

    // Master effect toggles
    let mut my = mast.bottom() - 50.0;
    for j in 0..app.song.master_fx.len().min(2) {
        let (name, on) = {
            let d = &app.song.master_fx[j];
            (d.kind.name(), d.on)
        };
        let r = Rect::from_min_size(
            Pos2::new(mast.left() + 12.0, my),
            Vec2::new(mast.width() - 24.0, 17.0),
        );
        let resp = ui.interact(r, Id::unique(("mfx", j)), Sense::click());
        let col = if on { MINT } else { FAINT };
        p.circle_filled(Pos2::new(r.left() + 5.0, r.center().y), 3.0, col);
        txt(
            &p,
            Pos2::new(r.left() + 14.0, r.center().y),
            Align2::LEFT_CENTER,
            fit(name, r.width() - 18.0, 8.0),
            8.0,
            if on { DIM } else { FAINT },
        );
        if resp.clicked() {
            app.song.master_fx[j].on = !on;
            app.dirty = true;
        }
        my += 18.0;
    }
    if app.song.master_fx.is_empty() {
        txt(
            &p,
            Pos2::new(mast.center().x, mast.bottom() - 40.0),
            Align2::CENTER_CENTER,
            "No master FX",
            8.0,
            FAINT,
        );
    }

    if let Some(i) = select {
        app.sel_track = i;
    }
}
