//! Device view: inspect and shape the selected track's instrument and
//! effect chain, including presets and quick-add effects.

use crate::app::{Act, DevSel, HarpsiApp};
use crate::theme::*;
use crate::widgets::*;
use eframe::egui::{self, Align2, Id, Pos2, Rect, Sense, Ui, Vec2};
use hs_core::devices::DRUM_LANES;
use hs_core::params::{Device, DeviceKind};
use hs_core::samples::{drum_kit_count, drum_lane_label, drum_sample_count};

fn tip(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::DrumRack => {
            "Thirteen real sample kits, sixteen voices: acoustic, world, orchestral, mallet and TR-808. Pick a Kit, set Tune/Decay, and play the pad map below."
        }
        DeviceKind::SubBass => "Pure low end. Raise Drive for harmonics the club can feel, Glide for slides.",
        DeviceKind::ReeseBass => "Detuned saw stack with a moving filter — the sound of dark DnB and techno.",
        DeviceKind::GrowlBass => "Dubstep workhorse: Wobble sets the LFO rate, Wob Amt opens the formant sweep.",
        DeviceKind::Supersaw => "Wide detuned lead. More Detune = wider, more Cutoff = brighter.",
        DeviceKind::Pluck => "Short house stabs. Punch boosts the filter snap; Decay shapes the tail.",
        DeviceKind::Keys => "Two-operator FM keys. Ratio sets the metallic character, FM Amt the bite.",
        DeviceKind::Organ => "Tonewheel drawbar organ with key click and rotary tremolo. Pull Drawbars up for gospel bite, or back it off for smoky house chords.",
        DeviceKind::Pad => "Slow evolving bed. Movement animates the filter over time.",
        DeviceKind::Riser => "Noise and tone sweeping upward. Length is in beats, so it always fits the drop.",
        DeviceKind::Salamander => "Real Salamander Grand Piano samples with soft/hard velocity layers, tone, release and octave. The Bass preset turns it into a warm sample bass.",
        DeviceKind::VoxBass => "A vocal-formant synth bass: vowel shapes, sub, growl and tempo-free wobble. Use Vocal Take when you have a real recorded vocal.",
        DeviceKind::VocalTake => "The sampler source for a real vocal. Import a WAV or record the mic from START → VOICE → BASS; the take is stored inside the project. Put Bassify and Wobble after it for a talking dubstep bass.",
        DeviceKind::Arpeggiator => "MIDI effect: turns held notes into a musical arpeggio. Rate locks to the beat, Mode covers Up/Down/Up-Down/Random, Octaves stacks the figure and Gate keeps it from smearing.",
        DeviceKind::ChordGenerator => "MIDI effect: turns single notes into scale-locked chords. Pick Key and Scale, then Diatonic, Major, Minor, 7th, Sus4 or Add9; Inversion spreads the voicing.",
        DeviceKind::Sidechain => "Tempo-locked ducking. Fixes clashing kick/bass and gives the classic pump.",
        DeviceKind::Delay => "Time is musical (beats). PingPong bounces repeats left and right.",
        DeviceKind::Reverb => "Space and depth. Size for the room, Damp to tame the top.",
        DeviceKind::Compressor => "Tames peaks. Lower Threshold and higher Ratio for more control.",
        DeviceKind::Eq3 => "Three-band tone shaping. Cut mids for width, lift lows for weight.",
        DeviceKind::Bitcrush => "Digital destruction for texture. Bits controls the grit.",
        DeviceKind::Drive => "Saturation and colour. Mix keeps the clean signal in parallel.",
        DeviceKind::Chorus => "Thickens and widens. Depth sets the wobble amount.",
        DeviceKind::Phaser => "Sweeping notches for movement. Center sets the focus frequency.",
        DeviceKind::AutoPan => "Rhythmic width. Depth and Rate keep a sound alive.",
        DeviceKind::Utility => "Gain, width and pan utility — the clean problem solver.",
        DeviceKind::Filter => "Sculpt with cutoff and resonance. Try a bandpass for vocal FX.",
        DeviceKind::Bassify => "Vocal-to-bass shaper: formant shifting, sub weight, growl and a wobble LFO. Put it on a vocal track or Vox Bass for talking-dubstep movement.",
        DeviceKind::Formant => "Morphing vowel filter with A·E·I·O·U motion. Use it on pads, organs or vocals for talking movement without the full Bassify sub layer.",
        DeviceKind::Wobble => "Tempo-free LFO filter for instant wobble. Low Rate = deep dubstep, high Rate = riddim chatter.",
        DeviceKind::Tape => "Warm tape saturation with flutter and a soft top end. Great on drums, keys and the master.",
        DeviceKind::Transient => "Adds attack snap and body weight without simply raising the level. Try it on sampled drums.",
        DeviceKind::Multiband => "Three-band saturation for thick low end, present mids and controlled highs.",
    }
}

fn param_grid(
    ui: &mut Ui,
    area: Rect,
    dev: &mut Device,
    track: usize,
    slot: usize,
    accent: egui::Color32,
) -> bool {
    let defs = dev.defs();
    let count = defs.len().max(1);
    let max_cols = ((area.width() / 76.0).floor() as usize).max(2);
    let max_rows = ((area.height() / 86.0).floor() as usize).max(1);
    let min_cols = count.div_ceil(max_rows);
    let cols = max_cols.max(min_cols).min(count).max(2);
    let cell_w = area.width() / cols as f32;
    let knob_w = (cell_w - 6.0).max(44.0);
    let mut changed = false;
    for (i, def) in defs.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        let kr = Rect::from_min_size(
            Pos2::new(
                area.left() + col as f32 * cell_w,
                area.top() + row as f32 * 86.0,
            ),
            Vec2::new(knob_w, 80.0),
        );
        let mut v = dev.p(i);
        let disp = def.format(v);
        let spec = KnobSpec {
            def: def.def,
            min: def.min,
            max: def.max,
            step: def.step,
            bipolar: def.bipolar,
            label: def.name,
            accent,
        };
        if knob(
            ui,
            kr,
            Id::unique(("param", track, slot, i)),
            &mut v,
            &disp,
            &spec,
        ) {
            dev.set(i, v);
            changed = true;
        }
    }
    changed
}

/// Device rack in the bottom detail dock: keep the full parameter grid, but
/// drop the tour tips and quick-add footer that belong to a full-height page.
pub fn show_detail(app: &mut HarpsiApp, ui: &mut Ui, rect: Rect) {
    show_impl(app, ui, rect, true);
}

fn show_impl(app: &mut HarpsiApp, ui: &mut Ui, rect: Rect, compact: bool) {
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, BG);

    if app.song.tracks.is_empty() {
        txt(
            &p,
            rect.center(),
            Align2::CENTER_CENTER,
            "No tracks. Load an instrument from the browser.",
            12.0,
            DIM,
        );
        return;
    }

    let ti = app.sel_track.min(app.song.tracks.len() - 1);
    let n = app.song.tracks.len();
    let mut acts: Vec<Act> = Vec::new();
    let mut select_track: Option<usize> = None;
    let mut select_dev: Option<DevSel> = None;
    let mut changed = false;
    let mut toast: Option<String> = None;

    // Track selector chips
    let cw = ((rect.width() - 260.0) / (n as f32 + 2.0)).clamp(74.0, 122.0);
    let mut x = rect.left() + 14.0;
    let cy = rect.top() + 8.0;
    for i in 0..n {
        let r = Rect::from_min_size(Pos2::new(x, cy), Vec2::new(cw - 5.0, 26.0));
        let t = &app.song.tracks[i];
        let label = format!(
            "{:02} {}",
            i + 1,
            fit(&t.name.to_uppercase(), cw - 34.0, 8.5)
        );
        let active = i == ti && !matches!(app.dev_sel, DevSel::Master | DevSel::Return);
        if chip(
            ui,
            r,
            Id::unique(("dvtrack", i)),
            &label,
            active,
            c(t.color),
        )
        .clicked()
        {
            select_track = Some(i);
            select_dev = Some(DevSel::Instrument);
        }
        x += cw;
        if x > rect.right() - 220.0 {
            break;
        }
    }
    let mr = Rect::from_min_size(
        Pos2::new(x.max(rect.right() - 210.0), cy),
        Vec2::new(98.0, 26.0),
    );
    if chip(
        ui,
        mr,
        Id::unique("dvmaster"),
        "MASTER",
        matches!(app.dev_sel, DevSel::Master),
        ACCENT,
    )
    .clicked()
    {
        select_dev = Some(DevSel::Master);
    }
    let rr = Rect::from_min_size(Pos2::new(mr.right() + 5.0, cy), Vec2::new(100.0, 26.0));
    if chip(
        ui,
        rr,
        Id::unique("dvreturn"),
        "RETURN A",
        matches!(app.dev_sel, DevSel::Return),
        BLUE,
    )
    .clicked()
    {
        select_dev = Some(DevSel::Return);
    }

    let body_top = cy + 38.0;
    let chain_w = 244.0;
    let chain = Rect::from_min_max(
        Pos2::new(rect.left() + 14.0, body_top),
        Pos2::new(rect.left() + 14.0 + chain_w, rect.bottom() - 12.0),
    );
    fill(&p, chain, PANEL);
    outline(&p, chain, 3.0, LINE_SOFT, 1.0);

    let right = Rect::from_min_max(
        Pos2::new(chain.right() + 14.0, body_top),
        Pos2::new(rect.right() - 14.0, rect.bottom() - 12.0),
    );

    if !matches!(app.dev_sel, DevSel::Master | DevSel::Return) {
        let tcol = c(app.song.tracks[ti].color);
        let inst = app.song.tracks[ti].instrument.kind;
        let fx_info: Vec<(DeviceKind, bool)> = app.song.tracks[ti]
            .effects
            .iter()
            .map(|d| (d.kind, d.on))
            .collect();
        let effect_count = fx_info.len();

        // Instrument slot
        let ir = Rect::from_min_size(
            Pos2::new(chain.left() + 8.0, chain.top() + 10.0),
            Vec2::new(chain.width() - 16.0, 54.0),
        );
        let active = app.dev_sel == DevSel::Instrument;
        rounded(
            &p,
            ir,
            3.0,
            if active {
                mix(PANEL2, tcol, 0.20)
            } else {
                PANEL2
            },
        );
        outline(&p, ir, 3.0, if active { tcol } else { LINE_SOFT }, 1.0);
        p.rect_filled(
            Rect::from_min_size(ir.min, Vec2::new(3.0, ir.height())),
            1.0,
            tcol,
        );
        txt(
            &p,
            Pos2::new(ir.left() + 12.0, ir.top() + 8.0),
            Align2::LEFT_TOP,
            "INSTRUMENT",
            7.0,
            FAINT,
        );
        txt(
            &p,
            Pos2::new(ir.left() + 12.0, ir.top() + 21.0),
            Align2::LEFT_TOP,
            inst.name(),
            13.0,
            TEXT,
        );
        mono(
            &p,
            Pos2::new(ir.right() - 10.0, ir.top() + 24.0),
            Align2::RIGHT_TOP,
            inst.short(),
            8.0,
            alpha(tcol, 230),
        );
        if ui
            .interact(ir, Id::unique("instslot"), Sense::click())
            .clicked()
        {
            select_dev = Some(DevSel::Instrument);
        }

        txt(
            &p,
            Pos2::new(chain.left() + 12.0, chain.top() + 78.0),
            Align2::LEFT_TOP,
            "DEVICE CHAIN",
            7.0,
            FAINT,
        );

        // Effect entries
        let mut ey = chain.top() + 92.0;
        let max_entries = ((chain.bottom() - ey - 96.0) / 36.0).max(1.0) as usize;
        for (ei, (fx_kind, fx_on)) in fx_info.iter().enumerate().take(max_entries) {
            let er = Rect::from_min_size(
                Pos2::new(chain.left() + 8.0, ey),
                Vec2::new(chain.width() - 16.0, 32.0),
            );
            let active = app.dev_sel == DevSel::Effect(ei);
            rounded(&p, er, 3.0, if active { PANEL3 } else { PANEL2 });
            outline(
                &p,
                er,
                3.0,
                if active { ACCENT_DARK } else { LINE_SOFT },
                1.0,
            );
            let dot = Rect::from_min_size(
                Pos2::new(er.left() + 7.0, er.center().y - 5.0),
                Vec2::new(10.0, 10.0),
            );
            let dresp = ui.interact(dot, Id::unique(("fxon", ei)), Sense::click());
            p.circle_filled(dot.center(), 4.0, if *fx_on { MINT } else { FAINT });
            if dresp.clicked() {
                acts.push(Act::ToggleEffect { track: ti, idx: ei });
            }
            txt(
                &p,
                Pos2::new(er.left() + 24.0, er.center().y),
                Align2::LEFT_CENTER,
                fit(fx_kind.name(), er.width() - 82.0, 10.0),
                10.0,
                if *fx_on { TEXT } else { FAINT },
            );
            let up = Rect::from_min_size(
                Pos2::new(er.right() - 52.0, er.center().y - 7.0),
                Vec2::new(15.0, 14.0),
            );
            let dn = Rect::from_min_size(
                Pos2::new(er.right() - 35.0, er.center().y - 7.0),
                Vec2::new(15.0, 14.0),
            );
            let rm = Rect::from_min_size(
                Pos2::new(er.right() - 18.0, er.center().y - 7.0),
                Vec2::new(14.0, 14.0),
            );
            if small_btn(ui, up, Id::unique(("fxup", ei)), "◀", DIM).clicked() {
                acts.push(Act::MoveEffect {
                    track: ti,
                    idx: ei,
                    dir: -1,
                });
            }
            if small_btn(ui, dn, Id::unique(("fxdn", ei)), "▶", DIM).clicked() {
                acts.push(Act::MoveEffect {
                    track: ti,
                    idx: ei,
                    dir: 1,
                });
            }
            if small_btn(ui, rm, Id::unique(("fxrm", ei)), "×", RED).clicked() {
                acts.push(Act::RemoveEffect { track: ti, idx: ei });
            }
            if ui
                .interact(er, Id::unique(("fxslot", ei)), Sense::click())
                .clicked()
            {
                select_dev = Some(DevSel::Effect(ei));
            }
            ey += 36.0;
        }
        if fx_info.is_empty() {
            txt(
                &p,
                Pos2::new(chain.left() + 14.0, chain.top() + 98.0),
                Align2::LEFT_TOP,
                "No effects yet. Add one from the",
                9.0,
                FAINT,
            );
            txt(
                &p,
                Pos2::new(chain.left() + 14.0, chain.top() + 112.0),
                Align2::LEFT_TOP,
                "browser or the quick-add row.",
                9.0,
                FAINT,
            );
        }
        if effect_count > max_entries {
            txt(
                &p,
                Pos2::new(chain.left() + 14.0, ey + 2.0),
                Align2::LEFT_TOP,
                format!("+{} more effects", effect_count - max_entries),
                8.0,
                FAINT,
            );
        }

        // Delete track
        let del = Rect::from_min_size(
            Pos2::new(chain.left() + 8.0, chain.bottom() - 30.0),
            Vec2::new(chain.width() - 16.0, 22.0),
        );
        if chip(ui, del, Id::unique("deltrack"), "DELETE TRACK", false, RED).clicked() {
            acts.push(Act::DeleteTrack(ti));
        }

        // Right: device header
        let (kind, is_inst) = match app.dev_sel {
            DevSel::Effect(ei) => fx_info
                .get(ei)
                .map(|(k, _)| (*k, false))
                .unwrap_or((inst, true)),
            _ => (inst, true),
        };
        let dcol = c(kind.color());

        txt(
            &p,
            Pos2::new(right.left(), right.top() + 4.0),
            Align2::LEFT_TOP,
            kind.name().to_uppercase(),
            20.0,
            TEXT,
        );
        mono(
            &p,
            Pos2::new(right.left(), right.top() + 30.0),
            Align2::LEFT_TOP,
            kind.category(),
            8.0,
            alpha(dcol, 230),
        );
        p.rect_filled(
            Rect::from_min_size(
                Pos2::new(right.left(), right.top() + 44.0),
                Vec2::new(46.0, 2.0),
            ),
            0.0,
            dcol,
        );

        // Preset chips
        let presets = kind.presets();
        let mut px = right.left();
        let mut py = right.top() + 54.0;
        let max_px = right.right() - 60.0;
        let init = Rect::from_min_size(Pos2::new(px, py), Vec2::new(46.0, 22.0));
        if chip(ui, init, Id::unique("initdev"), "INIT", false, DIM).clicked() {
            let target = if is_inst {
                Some(&mut app.song.tracks[ti].instrument)
            } else if let DevSel::Effect(ei) = app.dev_sel {
                app.song.tracks[ti].effects.get_mut(ei)
            } else {
                None
            };
            if let Some(dev) = target {
                *dev = Device::new(kind);
                changed = true;
            }
        }
        px += 52.0;
        if right.width() >= 500.0 {
            for (pi, preset) in presets.iter().enumerate() {
                let w = 8.0 + preset.name.len() as f32 * 5.6;
                if px + w > max_px && px > right.left() {
                    px = right.left();
                    py += 26.0;
                }
                let pr = Rect::from_min_size(Pos2::new(px, py), Vec2::new(w, 22.0));
                if chip(ui, pr, Id::unique(("preset", pi)), preset.name, false, dcol).clicked() {
                    let target: Option<&mut Device> = if is_inst {
                        Some(&mut app.song.tracks[ti].instrument)
                    } else if let DevSel::Effect(ei) = app.dev_sel {
                        app.song.tracks[ti].effects.get_mut(ei)
                    } else {
                        None
                    };
                    if let Some(dev) = target {
                        dev.load(preset);
                        changed = true;
                        toast = Some(format!("Preset: {}", preset.name));
                    }
                }
                px += w + 6.0;
            }
        }
        let preset_bottom = py + 22.0;
        let knob_top = (right.top() + 90.0).max(preset_bottom + 12.0);

        // On/off
        let on_r = Rect::from_min_size(
            Pos2::new(right.right() - 66.0, right.top() + 2.0),
            Vec2::new(66.0, 24.0),
        );
        let is_on = if is_inst {
            true
        } else if let DevSel::Effect(ei) = app.dev_sel {
            fx_info.get(ei).map(|(_, on)| *on).unwrap_or(true)
        } else {
            true
        };
        if !is_inst
            && toggle_sq(
                ui,
                on_r,
                Id::unique("devon"),
                if is_on { "ON" } else { "OFF" },
                is_on,
                MINT,
            )
            .clicked()
        {
            if let DevSel::Effect(ei) = app.dev_sel {
                acts.push(Act::ToggleEffect { track: ti, idx: ei });
            }
        }

        // Knobs. In the dock the footer is removed, so the grid gets the space
        // needed for a second parameter row on narrower windows.
        let knob_bottom = if compact {
            right.bottom() - 12.0
        } else {
            right.bottom() - 92.0
        };
        let grid = Rect::from_min_max(
            Pos2::new(right.left(), knob_top),
            Pos2::new(right.right(), knob_bottom),
        );
        let target: Option<&mut Device> = if is_inst {
            Some(&mut app.song.tracks[ti].instrument)
        } else if let DevSel::Effect(ei) = app.dev_sel {
            app.song.tracks[ti].effects.get_mut(ei)
        } else {
            None
        };
        if let Some(dev) = target {
            if param_grid(ui, grid, dev, ti, if is_inst { 0 } else { 1 }, dcol) {
                changed = true;
            }
        }

        // Drum map
        if is_inst && kind == DeviceKind::DrumRack && !compact {
            let kit = app.song.tracks[ti].instrument.p(8).round().clamp(0.0, 12.0) as usize;
            let param_cols = ((right.width() / 88.0).floor() as usize).max(2);
            let param_rows = kind.defs().len().div_ceil(param_cols);
            let map_y = (right.top() + 274.0).max(knob_top + param_rows as f32 * 86.0 + 8.0);
            txt(
                &p,
                Pos2::new(right.left(), map_y),
                Align2::LEFT_TOP,
                "PAD MAP",
                7.5,
                FAINT,
            );
            txt(
                &p,
                Pos2::new(right.right(), map_y),
                Align2::RIGHT_TOP,
                format!(
                    "{} KITS · {} SAMPLES",
                    drum_kit_count(),
                    drum_sample_count()
                ),
                7.5,
                FAINT,
            );
            let cols = ((right.width() / 118.0).floor() as usize).max(2);
            for (li, (_default_label, note)) in DRUM_LANES.iter().enumerate() {
                let name = drum_lane_label(kit, li);
                let cx = right.left() + (li % cols) as f32 * 116.0;
                let ry = map_y + 16.0 + (li / cols) as f32 * 19.0;
                mono(
                    &p,
                    Pos2::new(cx, ry),
                    Align2::LEFT_TOP,
                    format!("{:>3}", note),
                    8.5,
                    alpha(dcol, 220),
                );
                txt(
                    &p,
                    Pos2::new(cx + 26.0, ry),
                    Align2::LEFT_TOP,
                    name,
                    8.5,
                    DIM,
                );
            }
        }

        if !compact {
            // Tip
            txt(
                &p,
                Pos2::new(right.left(), right.bottom() - 62.0),
                Align2::LEFT_TOP,
                fit(tip(kind), right.width(), 9.0),
                9.0,
                DIM,
            );
            txt(
                &p,
                Pos2::new(right.left(), right.bottom() - 46.0),
                Align2::LEFT_TOP,
                "Drag knobs · SHIFT for fine control · double-click to reset",
                8.0,
                FAINT,
            );

            // Quick add
            let quick = [
                DeviceKind::Arpeggiator,
                DeviceKind::ChordGenerator,
                DeviceKind::Filter,
                DeviceKind::Sidechain,
                DeviceKind::Delay,
                DeviceKind::Reverb,
            ];
            let qy = right.bottom() - 30.0;
            let qw = 76.0;
            let mut qx = right.left();
            txt(
                &p,
                Pos2::new(qx, qy + 3.0),
                Align2::LEFT_TOP,
                "ADD",
                7.0,
                FAINT,
            );
            qx += 30.0;
            for kind in quick {
                if qx + qw > right.right() {
                    break;
                }
                let r = Rect::from_min_size(Pos2::new(qx, qy - 4.0), Vec2::new(qw - 4.0, 22.0));
                if chip(
                    ui,
                    r,
                    Id::unique(("quick", kind.name())),
                    kind.name(),
                    false,
                    c(kind.color()),
                )
                .clicked()
                {
                    acts.push(Act::AddEffect { track: ti, kind });
                }
                qx += qw;
            }
        }
    } else if matches!(app.dev_sel, DevSel::Master) {
        // Master page
        txt(
            &p,
            Pos2::new(chain.left() + 14.0, chain.top() + 14.0),
            Align2::LEFT_TOP,
            "MASTER",
            18.0,
            TEXT,
        );
        txt(
            &p,
            Pos2::new(chain.left() + 14.0, chain.top() + 40.0),
            Align2::LEFT_TOP,
            "Two-bus output with limiter.",
            9.0,
            DIM,
        );
        txt(
            &p,
            Pos2::new(chain.left() + 14.0, chain.top() + 64.0),
            Align2::LEFT_TOP,
            "MASTER CHAIN",
            7.5,
            FAINT,
        );
        let mut my = chain.top() + 80.0;
        for j in 0..app.song.master_fx.len() {
            let dev = &mut app.song.master_fx[j];
            let r = Rect::from_min_size(
                Pos2::new(chain.left() + 12.0, my),
                Vec2::new(chain.width() - 24.0, 28.0),
            );
            let resp = ui.interact(r, Id::unique(("dmfx", j)), Sense::click());
            rounded(&p, r, 3.0, PANEL2);
            p.circle_filled(
                Pos2::new(r.left() + 10.0, r.center().y),
                4.0,
                if dev.on { MINT } else { FAINT },
            );
            txt(
                &p,
                Pos2::new(r.left() + 22.0, r.center().y),
                Align2::LEFT_CENTER,
                dev.kind.name(),
                10.0,
                if dev.on { TEXT } else { FAINT },
            );
            if resp.clicked() {
                dev.on = !dev.on;
                changed = true;
            }
            my += 32.0;
        }

        let mv = &mut app.song.master_volume;
        let mut v = *mv;
        let vdisp = format!("{} dB", db_text(v));
        let kr = Rect::from_min_size(
            Pos2::new(chain.left() + 40.0, chain.top() + 300.0),
            Vec2::new(90.0, 84.0),
        );
        let spec = KnobSpec {
            def: 0.85,
            min: 0.0,
            max: 1.3,
            step: 0.0,
            bipolar: false,
            label: "MASTER VOL",
            accent: ACCENT,
        };
        if knob(ui, kr, Id::unique("dmvol"), &mut v, &vdisp, &spec) {
            *mv = v;
            changed = true;
        }

        // Right: overview of all track levels
        txt(
            &p,
            Pos2::new(right.left(), right.top() + 4.0),
            Align2::LEFT_TOP,
            "SESSION LEVELS",
            9.0,
            DIM,
        );
        let mut yy = right.top() + 26.0;
        for (i, t) in app.song.tracks.iter().enumerate() {
            if yy > right.bottom() - 30.0 {
                break;
            }
            let col = c(t.color);
            let level = app.shared.level(i.min(23));
            txt(
                &p,
                Pos2::new(right.left(), yy),
                Align2::LEFT_TOP,
                fit(&t.name, 130.0, 9.0),
                9.0,
                DIM,
            );
            hbar(
                &p,
                Rect::from_min_size(
                    Pos2::new(right.left() + 140.0, yy + 1.0),
                    Vec2::new(right.width() - 140.0, 7.0),
                ),
                level,
            );
            p.circle_filled(Pos2::new(right.right() - 2.0, yy + 4.0), 2.0, col);
            yy += 22.0;
        }
        txt(
            &p,
            Pos2::new(right.left(), right.bottom() - 22.0),
            Align2::LEFT_TOP,
            "Master effects are shown in the chain on the left · click a dot to bypass.",
            8.0,
            FAINT,
        );
    } else {
        // Return A
        txt(
            &p,
            Pos2::new(chain.left() + 14.0, chain.top() + 14.0),
            Align2::LEFT_TOP,
            "RETURN A",
            18.0,
            TEXT,
        );
        txt(
            &p,
            Pos2::new(chain.left() + 14.0, chain.top() + 40.0),
            Align2::LEFT_TOP,
            "Global reverb fed by track SENDs.",
            9.0,
            DIM,
        );
        txt(
            &p,
            Pos2::new(chain.left() + 14.0, chain.top() + 64.0),
            Align2::LEFT_TOP,
            "FEED WITH THE SEND KNOB IN THE MIXER",
            7.0,
            FAINT,
        );
        txt(
            &p,
            Pos2::new(chain.left() + 14.0, chain.top() + 86.0),
            Align2::LEFT_TOP,
            "Mixer → SEND → more reverb.",
            9.0,
            FAINT,
        );

        let dev = &mut app.song.return_fx;
        let defs = dev.defs();
        let cols = ((right.width() / 100.0).floor() as usize).max(2);
        for (i, def) in defs.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let kr = Rect::from_min_size(
                Pos2::new(
                    right.left() + col as f32 * 96.0,
                    right.top() + 20.0 + row as f32 * 88.0,
                ),
                Vec2::new(90.0, 82.0),
            );
            let mut v = dev.p(i);
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
            if knob(ui, kr, Id::unique(("retdev", i)), &mut v, &disp, &spec) {
                dev.set(i, v);
                changed = true;
            }
        }
        txt(
            &p,
            Pos2::new(right.left(), right.top() + 200.0),
            Align2::LEFT_TOP,
            "A long, dark plate for tails; keep Mix near 100% because",
            9.0,
            DIM,
        );
        txt(
            &p,
            Pos2::new(right.left(), right.top() + 214.0),
            Align2::LEFT_TOP,
            "this is a send bus, not an insert.",
            9.0,
            DIM,
        );
    }

    if let Some(i) = select_track {
        app.sel_track = i;
        app.dev_sel = DevSel::Instrument;
    }
    if let Some(d) = select_dev {
        app.dev_sel = d;
    }
    if changed {
        app.dirty = true;
    }
    for a in acts {
        app.act(a);
    }
    if let Some(t) = toast {
        app.flash(t);
    }
}
