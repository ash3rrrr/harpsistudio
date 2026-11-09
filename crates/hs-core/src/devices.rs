//! Instruments and effects. Everything renders sample-by-sample so the
//! engine can sequence notes at sample accuracy and apply per-voice filtering.

use crate::audio_take::AudioTake;
use crate::dsp::*;
use crate::params::{Device, DeviceKind};
use crate::samples::{self, Sample};
use std::f32::consts::TAU;

#[derive(Clone, Copy)]
pub struct Voice {
    pub active: bool,
    pub note: u8,
    pub vel: f32,
    pub amp: f32,
    pub age: f32,
    pub rel_at: f32,
    pub gate: bool,
    pub sustained: bool,
    pub end_beat: f64,
    pub ph: [f32; 4],
    pub st: [f32; 8],
    pub freq: f32,
    pub pan: f32,
    pub seed: u32,
    pub sample: Option<&'static Sample>,
    pub sample_pos: f32,
    pub sample_step: f32,
    pub take_started: bool,
    /// Two-pole DC servo for sample-based voices (and future asymmetric
    /// instruments). Kept separate from the per-instrument filter states.
    pub dc: [f32; 2],
}

impl Default for Voice {
    fn default() -> Self {
        Self::new()
    }
}

impl Voice {
    pub fn new() -> Self {
        Self {
            active: false,
            note: 60,
            vel: 1.0,
            amp: 1.0,
            age: 0.0,
            rel_at: 0.0,
            gate: false,
            sustained: false,
            end_beat: 0.0,
            ph: [0.0; 4],
            st: [0.0; 8],
            freq: 440.0,
            pan: 0.0,
            seed: 0x1234_5678,
            sample: None,
            sample_pos: 0.0,
            sample_step: 1.0,
            take_started: false,
            dc: [0.0; 2],
        }
    }

    pub fn note_on(&mut self, note: u8, vel: f32, end_beat: f64, seed: u32) {
        self.active = true;
        self.note = note;
        self.vel = vel;
        self.amp = 0.3 + 0.7 * vel;
        self.age = 0.0;
        self.rel_at = 0.0;
        self.gate = true;
        self.sustained = false;
        self.end_beat = end_beat;
        self.ph = [0.0; 4];
        self.st = [0.0; 8];
        self.freq = midi_hz(note as f32);
        self.pan = 0.0;
        self.seed = seed | 1;
        self.sample = None;
        self.sample_pos = 0.0;
        self.sample_step = 1.0;
        self.take_started = false;
        self.dc = [0.0; 2];
    }

    pub fn release(&mut self) {
        if self.gate {
            self.gate = false;
            self.rel_at = self.age;
        }
    }

    #[inline]
    pub fn rnd(&mut self) -> f32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Per-track instrument state shared by every voice (LFOs, glide).
pub struct InstrRt {
    pub lfo: f32,
    pub lfo2: f32,
    pub glide: f32,
    pub last_note: u8,
}

impl Default for InstrRt {
    fn default() -> Self {
        Self::new()
    }
}

impl InstrRt {
    pub fn new() -> Self {
        Self {
            lfo: 0.0,
            lfo2: 0.0,
            glide: 440.0,
            last_note: 60,
        }
    }
}

pub const DRUM_LANES: [(&str, u8); 16] = [
    ("Kick", 36),
    ("Snare", 38),
    ("Clap", 39),
    ("Hat", 42),
    ("Open Hat", 46),
    ("Low Tom", 45),
    ("Crash", 49),
    ("Rim", 37),
    ("Sub 808", 35),
    ("Perc", 43),
    ("Shaker", 70),
    ("Ride", 51),
    ("Cowbell", 56),
    ("Snap", 50),
    ("Floor Tom", 41),
    ("High Tom", 48),
];

pub fn drum_lane(note: u8) -> usize {
    match note {
        36 => 0,
        38 => 1,
        39 => 2,
        42 => 3,
        46 => 4,
        45 => 5,
        49 => 6,
        37 => 7,
        35 => 8,
        43 => 9,
        70 => 10,
        51 => 11,
        56 => 12,
        50 => 13,
        41 => 14,
        48 => 15,
        _ => (note as usize) % 16,
    }
}

pub fn render_instrument(
    dev: &Device,
    v: &mut Voice,
    rt: &mut InstrRt,
    sr: f32,
    bpm: f64,
    take: Option<&AudioTake>,
) -> f32 {
    if !v.active {
        return 0.0;
    }
    let out = match dev.kind {
        DeviceKind::DrumRack => drum(dev, v, sr),
        DeviceKind::SubBass => sub_bass(dev, v, rt, sr),
        DeviceKind::ReeseBass => reese(dev, v, rt, sr),
        DeviceKind::GrowlBass => growl(dev, v, rt, sr),
        DeviceKind::Supersaw => supersaw(dev, v, sr),
        DeviceKind::Pluck => pluck(dev, v, sr),
        DeviceKind::Keys => keys(dev, v, sr),
        DeviceKind::Organ => organ(dev, v, rt, sr),
        DeviceKind::Pad => pad(dev, v, rt, sr),
        DeviceKind::Riser => riser(dev, v, rt, sr, bpm),
        DeviceKind::Salamander => salamander(dev, v, sr),
        DeviceKind::VoxBass => vox_bass(dev, v, rt, sr),
        DeviceKind::VocalTake => vocal_take(dev, v, take, sr),
        _ => 0.0,
    };
    v.age += 1.0 / sr;
    out
}

fn drum(dev: &Device, v: &mut Voice, sr: f32) -> f32 {
    let level = dev.p(0);
    let tune = (dev.p(1) / 12.0).exp2();
    let decay = dev.p(2);
    let punch = dev.p(3);
    let snap = dev.p(4);
    let drv = dev.p(5);
    let tone = dev.p(7);
    let lane = drum_lane(v.note);
    let age = v.age;
    let kit = (dev.p(8).round() as usize).min(samples::drum_kit_count() - 1);
    if v.sample.is_none() {
        v.sample = samples::drum(kit, lane);
        v.sample_pos = 0.0;
        v.sample_step = tune * v.sample.map(|s| s.rate / sr).unwrap_or(1.0);
        v.pan = match lane {
            4 | 6 | 11 | 12 => 0.28,
            5 | 14 => -0.22,
            9 | 10 => 0.16,
            _ => 0.0,
        };
    }
    let Some(sample) = v.sample else {
        v.active = false;
        return 0.0;
    };
    let len = sample.data.len();
    let idx = v.sample_pos as usize;
    if idx + 1 >= len {
        v.active = false;
        return 0.0;
    }
    let frac = v.sample_pos - idx as f32;
    let mut s = sample.data[idx] * (1.0 - frac) + sample.data[idx + 1] * frac;
    v.sample_pos += v.sample_step;

    // A short fade makes chopped tails click-free without robbing the sample
    // of its natural body. Decay doubles as the maximum tail length.
    let tail = (sample.data.len() as f32 / sample.rate).max(0.08);
    let fade_start = (decay * 2.2).min(tail * 0.92);
    let fade = if age > fade_start {
        (-(age - fade_start) / (tail - fade_start).max(0.02) * 5.0).exp()
    } else {
        1.0
    };
    let transient = 1.0 + (punch * 0.35 + snap * 0.25) * (-age * 55.0).exp();
    s *= fade * transient * v.vel.max(0.05) * 1.25;

    let cut = 700.0 + tone * 17000.0;
    s = onepole(s, &mut v.st[6], cut, sr);
    s = drive(s, drv * 0.7);
    // A recorded one-shot can have a different mean over the short voiced
    // prefix than over the whole file. Remove that residue at the voice, not
    // at the kit bus or master bus.
    s = highpass(highpass(s, &mut v.dc[0], 10.0, sr), &mut v.dc[1], 10.0, sr);
    if !v.gate && age > decay * 3.5 + 0.25 {
        v.active = false;
    }
    s * level * samples::drum_kit_gain(kit)
}

fn sub_bass(dev: &Device, v: &mut Voice, rt: &mut InstrRt, sr: f32) -> f32 {
    let level = dev.p(0);
    let tune = dev.p(1);
    let (env, done) = amp_env(
        v.age,
        v.rel_at,
        v.gate,
        dev.p(2),
        dev.p(3),
        dev.p(4),
        dev.p(5),
    );
    if done {
        v.active = false;
    }
    let target = midi_hz(v.note as f32 + tune);
    let glide = dev.p(7);
    if glide > 0.001 {
        let coef = 1.0 - (-1.0 / (glide * sr)).exp();
        v.st[7] += (target - v.st[7]) * coef;
    } else {
        v.st[7] = target;
    }
    let f = if v.st[7] <= 0.0 { target } else { v.st[7] };
    rt.last_note = v.note;
    let dt = f / sr;
    v.ph[0] = wrap01(v.ph[0] + dt);
    v.ph[1] = wrap01(v.ph[1] + dt * 0.5);
    v.ph[2] = wrap01(v.ph[2] + dt);
    let body = sine(v.ph[0]);
    let sub = sine(v.ph[1]) * 0.55;
    let grit = saw(v.ph[2], dt) * 0.12;
    let s = drive(body + sub + grit, dev.p(6)) * env * v.amp;
    s * level * 0.8
}

fn reese(dev: &Device, v: &mut Voice, rt: &mut InstrRt, sr: f32) -> f32 {
    let level = dev.p(0);
    let cutoff = dev.p(1);
    let reso = dev.p(2);
    let detune = dev.p(3);
    let rate = dev.p(4);
    let depth = dev.p(5);
    let drv = dev.p(6);
    let (env, done) = amp_env(v.age, v.rel_at, v.gate, dev.p(7), 0.4, 0.95, 0.09);
    if done {
        v.active = false;
    }
    rt.lfo = wrap01(rt.lfo + rate / sr);
    let lfo = sine(rt.lfo);
    let base = midi_hz(v.note as f32);
    let dt = base / sr;
    v.ph[0] = wrap01(v.ph[0] + dt * (1.0 - detune));
    v.ph[1] = wrap01(v.ph[1] + dt);
    v.ph[2] = wrap01(v.ph[2] + dt * (1.0 + detune));
    v.ph[3] = wrap01(v.ph[3] + dt * 0.5);
    let mut raw = saw(v.ph[0], dt) + saw(v.ph[1], dt) + saw(v.ph[2], dt) * 0.9;
    raw += sine(v.ph[3]) * 0.5;
    let cut = cutoff * 2f32.powf(depth * 2.0 * lfo);
    let lp = ladder(raw, &mut v.st[0..4], cut, reso, sr);
    let s = drive(lp * 0.4, drv) * env * v.amp;
    // Symmetric saws through a resonant ladder can accumulate a small DC
    // component at note boundaries. Remove it at the voice output so it never
    // reaches the track fader or the shared reverb bus.
    let s = highpass(highpass(s, &mut v.st[4], 20.0, sr), &mut v.st[5], 20.0, sr);
    s * level
}

fn growl(dev: &Device, v: &mut Voice, rt: &mut InstrRt, sr: f32) -> f32 {
    let level = dev.p(0);
    let cutoff = dev.p(1);
    let reso = dev.p(2);
    let rate = dev.p(3);
    let depth = dev.p(4);
    let fm = dev.p(5);
    let drv = dev.p(6);
    let (env, done) = amp_env(v.age, v.rel_at, v.gate, dev.p(7), 0.5, 1.0, 0.06);
    if done {
        v.active = false;
    }
    rt.lfo = wrap01(rt.lfo + rate / sr);
    let wob = sine(rt.lfo);
    let base = midi_hz(v.note as f32) * (1.0 + 0.012 * fm * wob);
    let dt = base / sr;
    v.ph[0] = wrap01(v.ph[0] + dt);
    v.ph[1] = wrap01(v.ph[1] + dt * 1.007);
    v.ph[2] = wrap01(v.ph[2] + dt * 2.0);
    let raw = saw(v.ph[0], dt) * 0.55 + square(v.ph[1], dt) * 0.28;
    let shaped = fold(raw * 1.4);
    let cut = cutoff * 2f32.powf(depth * 3.0 * wob);
    let lp = ladder(shaped, &mut v.st[0..4], cut, reso, sr);
    let bp = svf(
        shaped,
        &mut v.st[4..6],
        cut * 1.6,
        (reso * 0.8).min(0.9),
        sr,
        1,
    );
    let mut s = lp + bp * fm * 0.9;
    s = drive(s, drv);
    let growl_am = 0.75 + 0.25 * sine(v.ph[2]);
    // The wavefolder is intentionally asymmetric around zero. Its musically
    // useful grit also creates a DC term; block it at the instrument output.
    let s = highpass(highpass(s, &mut v.st[6], 20.0, sr), &mut v.st[7], 20.0, sr);
    s * growl_am * env * v.amp * level
}

fn supersaw(dev: &Device, v: &mut Voice, sr: f32) -> f32 {
    let level = dev.p(0);
    let detune = dev.p(1);
    let cutoff = dev.p(2);
    let reso = dev.p(3);
    let (env, done) = amp_env(v.age, v.rel_at, v.gate, dev.p(4), 0.3, 0.85, dev.p(5));
    if done {
        v.active = false;
    }
    let base = midi_hz(v.note as f32 + dev.p(6).round() * 12.0);
    let dt = base / sr;
    let mut raw = 0.0;
    for i in 0..4 {
        let m = 1.0 + detune * (i as f32 - 1.5);
        v.ph[i] = wrap01(v.ph[i] + dt * m);
        raw += saw(v.ph[i], dt * m);
    }
    raw *= 0.28;
    let fenv = 0.45 + 0.55 * (-v.age * 3.0).exp();
    let lp = ladder(raw, &mut v.st[0..4], cutoff * fenv, reso, sr);
    lp * env * v.amp * level
}

fn pluck(dev: &Device, v: &mut Voice, sr: f32) -> f32 {
    let level = dev.p(0);
    let cutoff = dev.p(1);
    let reso = dev.p(2);
    let decay = dev.p(3);
    let detune = dev.p(4);
    let punch = dev.p(5);
    let (env, done) = amp_env(v.age, v.rel_at, v.gate, dev.p(6), decay * 0.7, 0.0, 0.09);
    if done {
        v.active = false;
    }
    let base = midi_hz(v.note as f32 + dev.p(7).round() * 12.0);
    let dt = base / sr;
    v.ph[0] = wrap01(v.ph[0] + dt * (1.0 - detune));
    v.ph[1] = wrap01(v.ph[1] + dt * (1.0 + detune));
    v.ph[2] = wrap01(v.ph[2] + dt * 2.0);
    let raw = saw(v.ph[0], dt) * 0.5 + saw(v.ph[1], dt) * 0.5 + saw(v.ph[2], dt) * 0.15;
    let fenv = 1.0 + punch * 6.0 * (-v.age * (18.0 + punch * 30.0)).exp();
    let lp = ladder(raw, &mut v.st[0..4], cutoff * fenv, reso, sr);
    // The three-saw pluck can leave a small boundary DC term after the ladder.
    let lp = highpass(highpass(lp, &mut v.st[4], 20.0, sr), &mut v.st[5], 20.0, sr);
    lp * env * v.amp * level * 0.85
}

fn keys(dev: &Device, v: &mut Voice, sr: f32) -> f32 {
    let level = dev.p(0);
    let ratio = dev.p(1).max(0.25);
    let fm = dev.p(2);
    let (env, done) = amp_env(
        v.age,
        v.rel_at,
        v.gate,
        dev.p(3),
        dev.p(4),
        dev.p(5),
        dev.p(6),
    );
    if done {
        v.active = false;
    }
    let base = midi_hz(v.note as f32);
    let dt = base / sr;
    v.ph[0] = wrap01(v.ph[0] + dt);
    v.ph[1] = wrap01(v.ph[1] + dt * ratio);
    v.ph[2] = wrap01(v.ph[2] + dt * 0.5);
    let mod_sig = sine(v.ph[1]) * fm * 4.0;
    let car = sine(wrap01(v.ph[0] + mod_sig * 0.1));
    let sub = sine(v.ph[2]) * 0.2;
    let t = 300.0 + dev.p(7) * 9000.0;
    let s = onepole(car + sub, &mut v.st[0], t, sr);
    s * env * v.amp * level * 0.8
}

fn pad(dev: &Device, v: &mut Voice, rt: &mut InstrRt, sr: f32) -> f32 {
    let level = dev.p(0);
    let (env, done) = amp_env(v.age, v.rel_at, v.gate, dev.p(1), 1.0, 0.9, dev.p(2));
    if done {
        v.active = false;
    }
    let base = midi_hz(v.note as f32 + dev.p(6).round() * 12.0);
    let dt = base / sr;
    let detune = dev.p(5);
    let mut raw = 0.0;
    for i in 0..4 {
        let m = 1.0 + detune * (i as f32 - 1.5);
        v.ph[i] = wrap01(v.ph[i] + dt * m);
        raw += saw(v.ph[i], dt * m);
    }
    rt.lfo2 = wrap01(rt.lfo2 + 0.12 / sr);
    let move_amt = dev.p(7);
    let cut = dev.p(3) * (1.0 + move_amt * 0.8 * sine(rt.lfo2));
    let lp = ladder(raw * 0.26, &mut v.st[0..4], cut, dev.p(4), sr);
    // Four detuned saws can leave a small low-frequency residue after the
    // ladder; keep the sustained chord DC-free at the voice source.
    let s = highpass(highpass(lp, &mut v.st[4], 20.0, sr), &mut v.st[5], 20.0, sr);
    s * env * v.amp * level
}

fn riser(dev: &Device, v: &mut Voice, rt: &mut InstrRt, sr: f32, bpm: f64) -> f32 {
    let level = dev.p(0);
    let length_s = (dev.p(1) as f64 * 60.0 / bpm.max(30.0)) as f32;
    let rise = dev.p(2);
    let cutoff = dev.p(3);
    let reso = dev.p(4);
    let noise_mix = dev.p(5);
    let (env, done) = amp_env(
        v.age,
        v.rel_at,
        v.gate,
        dev.p(6),
        length_s.max(0.2),
        1.0,
        dev.p(7),
    );
    if done || v.age > length_s * 2.5 {
        v.active = false;
    }
    let prog = (v.age / length_s.max(0.05)).clamp(0.0, 1.5);
    let base = midi_hz(v.note as f32) * (1.0 + rise * 2.0 * prog + rise * prog * prog);
    let dt = base / sr;
    v.ph[0] = wrap01(v.ph[0] + dt);
    v.ph[1] = wrap01(v.ph[1] + dt * 1.5);
    rt.lfo = wrap01(rt.lfo + (0.5 + prog * 6.0) / sr);
    let osc = saw(v.ph[0], dt) * 0.5 + saw(v.ph[1], dt) * 0.25;
    let n = v.rnd() * 0.8;
    let mixed = osc * (1.0 - noise_mix) + n * noise_mix;
    let cut = (cutoff * (1.0 + 9.0 * prog * rise)).min(sr * 0.45);
    let lp = ladder(mixed, &mut v.st[0..4], cut, reso, sr);
    lp * env * v.amp * level * (0.6 + prog.min(1.0) * 0.5)
}

const FORMANT_TABLE: [[f32; 3]; 5] = [
    [800.0, 1150.0, 2900.0], // A
    [500.0, 1800.0, 2500.0], // E
    [300.0, 2300.0, 3000.0], // I
    [500.0, 900.0, 2400.0],  // O
    [350.0, 800.0, 2200.0],  // U
];

fn organ(dev: &Device, v: &mut Voice, rt: &mut InstrRt, sr: f32) -> f32 {
    let level = dev.p(0);
    let bars = dev.p(1).clamp(0.0, 1.0);
    let click = dev.p(2);
    let rotary = dev.p(3);
    let drv = dev.p(4);
    let (env, done) = amp_env(v.age, v.rel_at, v.gate, dev.p(5), 0.02, 1.0, dev.p(6));
    if done {
        v.active = false;
    }

    let f = midi_hz(v.note as f32 + dev.p(7).round() * 12.0);
    let base = f * 0.5;
    rt.lfo = wrap01(rt.lfo + 5.6 / sr);
    let rot = sine(rt.lfo);
    // Rotary speakers add a little pitch shimmer plus a gentle tremolo.
    let bend = 1.0 + rotary * 0.0025 * rot;
    v.ph[0] = wrap01(v.ph[0] + base * bend / sr);

    // Nine drawbar partials, measured from the 16' fundamental. The Drawbars
    // control tilts the balance from hollow and round to bright and vocal.
    let weights = [
        1.0 - 0.12 * bars,
        0.9,
        0.42 + 0.38 * bars,
        0.52 - 0.08 * bars,
        0.26 + 0.44 * bars,
        0.16 + 0.46 * bars,
        0.08 + 0.36 * bars,
        0.05 + 0.28 * bars,
        0.03 + 0.2 * bars,
    ];
    let ratios = [1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 10.0, 12.0, 16.0];
    let mut sum = 0.0;
    let mut norm = 0.0;
    for (w, ratio) in weights.iter().zip(ratios) {
        let partial_hz = base * ratio;
        if partial_hz < sr * 0.45 {
            sum += sine(v.ph[0] * ratio) * *w;
            norm += *w;
        }
    }
    if norm > 0.0 {
        sum /= norm;
    }

    // Key click is a very short band-limited noise burst. It is what makes a
    // tonewheel organ read as an instrument rather than a bank of sine waves.
    let tick = v.rnd() * click * (-v.age / 0.004).exp();
    sum += highpass(tick, &mut v.st[0], 2200.0, sr) * 0.55;

    let trem = 1.0 - rotary * 0.1 * (0.5 + 0.5 * rot);
    v.pan = rotary * 0.22 * rot;
    drive(sum * env * v.amp * level * 1.5 * trem, drv)
}

fn salamander(dev: &Device, v: &mut Voice, sr: f32) -> f32 {
    let level = dev.p(0);
    let tone = dev.p(1);
    let release = dev.p(2);
    let detune = dev.p(3);
    let octave = dev.p(4).round();
    let vel_sense = dev.p(5);
    let width = dev.p(6);
    let attack = dev.p(7).max(0.0005);

    if v.sample.is_none() {
        let threshold = 0.58 - vel_sense * 0.16;
        let hard = v.vel >= threshold;
        if let Some(s) = samples::piano(v.note, hard) {
            let semis = v.note as f32 - s.root as f32 + detune + octave * 12.0;
            v.sample_step = semis.exp2() * s.rate / sr;
            v.sample = Some(s);
            v.sample_pos = 0.0;
        } else {
            v.active = false;
            return 0.0;
        }
    }
    let Some(sample) = v.sample else {
        v.active = false;
        return 0.0;
    };
    if v.sample_pos >= sample.data.len().saturating_sub(2) as f32 {
        v.active = false;
        return 0.0;
    }
    let idx = v.sample_pos as usize;
    let frac = v.sample_pos - idx as f32;
    let mut s = sample.data[idx] * (1.0 - frac) + sample.data[idx + 1] * frac;
    v.sample_pos += v.sample_step;

    let attack_amp = (v.age / attack).min(1.0);
    let amp = if v.gate {
        attack_amp
    } else {
        let rt = (v.age - v.rel_at).max(0.0);
        if rt > release * 2.5 {
            v.active = false;
        }
        (-rt / release.max(0.005) * 4.5).exp()
    };

    // Keep the recorded body but tame the top end when Tone is lowered.
    let cut = 450.0 + tone.powf(1.35) * 17500.0;
    s = onepole(s, &mut v.st[0], cut, sr);
    v.pan = (width * 0.32) * (((v.note % 12) as f32 / 11.0) - 0.5);
    s * amp * v.vel.max(0.08) * level * 0.9
}

fn vox_bass(dev: &Device, v: &mut Voice, rt: &mut InstrRt, sr: f32) -> f32 {
    let level = dev.p(0);
    let vowel = (dev.p(1).round() as usize).min(FORMANT_TABLE.len() - 1);
    let sub_amt = dev.p(2);
    let growl = dev.p(3);
    let rate = dev.p(4);
    let wob_amt = dev.p(5);
    let drv = dev.p(6);
    let (env, done) = amp_env(v.age, v.rel_at, v.gate, 0.006, 0.3, 0.9, dev.p(7));
    if done {
        v.active = false;
    }

    rt.lfo = wrap01(rt.lfo + rate / sr);
    let wob = sine(rt.lfo);
    let base = midi_hz(v.note as f32) * (1.0 + 0.018 * wob * wob_amt);
    let dt = base / sr;
    v.ph[0] = wrap01(v.ph[0] + dt);
    v.ph[1] = wrap01(v.ph[1] + dt * 0.5);
    v.ph[2] = wrap01(v.ph[2] + dt * 2.0);

    // A buzzy glottal source feeds three fixed vowel formants. The wobble
    // moves the formants and adds a little amplitude tremolo, which is the
    // recognisable "talking bass" motion.
    let raw = saw(v.ph[0], dt) * 0.8 + square(v.ph[2], dt * 2.0) * 0.18;
    let f = FORMANT_TABLE[vowel];
    let bend = 1.0 + 0.18 * wob_amt * wob;
    let b1 = svf(raw, &mut v.st[0..2], f[0] * bend, 0.72, sr, 1);
    let b2 = svf(raw, &mut v.st[2..4], f[1] * bend, 0.68, sr, 1);
    let b3 = svf(raw, &mut v.st[4..6], f[2] * bend, 0.55, sr, 1);
    let voiced = (b1 * 1.45 + b2 * 0.85 + b3 * 0.45) * (1.0 + growl * 1.6);
    let sub = sine(v.ph[1]) * sub_amt * 0.9;
    let mut s = voiced + sub;
    s = drive(s, drv);
    let am = 1.0 - wob_amt * 0.22 * (0.5 + 0.5 * wob);
    // Formant filtering and folding a glottal pulse produce a moving DC term.
    // Remove it before the voice is summed into the track.
    let s = highpass(highpass(s, &mut v.st[6], 20.0, sr), &mut v.st[7], 20.0, sr);
    s * am * env * v.amp * level * 0.6
}

fn vocal_take(dev: &Device, v: &mut Voice, take: Option<&AudioTake>, sr: f32) -> f32 {
    let level = dev.p(0);
    let Some(take) = take else {
        v.active = false;
        return 0.0;
    };
    if take.samples.len() < 4 {
        v.active = false;
        return 0.0;
    }

    let len = take.samples.len();
    let pitch = dev.p(1);
    let start = dev.p(2).max(0.0);
    let tone = dev.p(3);
    let release = dev.p(4).max(0.005);
    let reverse = dev.p(5) >= 0.5;

    if !v.take_started {
        v.take_started = true;
        let step = (pitch / 12.0).exp2() * take.sample_rate as f32 / sr;
        let head = (start * take.sample_rate as f32).min((len - 2) as f32);
        v.sample_step = if reverse { -step } else { step };
        v.sample_pos = if reverse {
            (len - 2) as f32 - head
        } else {
            head
        };
    }

    if v.sample_pos < 0.0 || v.sample_pos >= (len - 1) as f32 {
        v.active = false;
        return 0.0;
    }
    let idx = v.sample_pos.floor() as usize;
    let frac = v.sample_pos - idx as f32;
    let a = take.samples[idx] as f32 / 32768.0;
    let b = take.samples[(idx + 1).min(len - 1)] as f32 / 32768.0;
    let mut s = a * (1.0 - frac) + b * frac;
    v.sample_pos += v.sample_step;

    // Preserve the performed pitch contour; the octave-down control is what
    // puts a natural vocal into a musical bass register before Bassify.
    let attack = (v.age / 0.004).min(1.0);
    let amp = if v.gate {
        attack
    } else {
        let rt = (v.age - v.rel_at).max(0.0);
        if rt > release * 2.5 {
            v.active = false;
        }
        (-rt / release * 4.5).exp()
    };
    let cut = 320.0 + tone.powf(1.25) * 13000.0;
    s = onepole(s, &mut v.st[0], cut, sr);
    s * amp * v.amp * level * 0.9
}

// ---------------------------------------------------------------------------
// Effects
// ---------------------------------------------------------------------------

pub struct FxState {
    pub kind: DeviceKind,
    pub buf_l: Vec<f32>,
    pub buf_r: Vec<f32>,
    pub idx: usize,
    pub z: [f32; 32],
    pub ph: f32,
    pub env: f32,
}

fn read_buf(buf: &[f32], idx: usize, delay: f32) -> f32 {
    let len = buf.len();
    if len < 8 {
        return 0.0;
    }
    let d = delay.clamp(1.0, len as f32 - 2.0);
    let mut pos = idx as f32 - d;
    if pos < 0.0 {
        pos += len as f32;
    }
    let i0 = pos.floor() as usize % len;
    let frac = pos - pos.floor();
    let i1 = (i0 + 1) % len;
    buf[i0] * (1.0 - frac) + buf[i1] * frac
}

impl FxState {
    pub fn new(kind: DeviceKind, sr: f32) -> Self {
        let seconds = match kind {
            DeviceKind::Reverb => 1.8,
            DeviceKind::Delay => 2.4,
            DeviceKind::Chorus => 0.12,
            _ => 0.0,
        };
        let n = (sr * seconds) as usize;
        Self {
            kind,
            buf_l: vec![0.0; n],
            buf_r: vec![0.0; n],
            idx: 0,
            z: [0.0; 32],
            ph: 0.0,
            env: 0.0,
        }
    }
}

pub fn process_effect(
    dev: &Device,
    st: &mut FxState,
    l: f32,
    r: f32,
    sr: f32,
    beat: f64,
    bpm: f64,
) -> (f32, f32) {
    match dev.kind {
        DeviceKind::Filter => {
            let cut = dev.p(0);
            let reso = dev.p(1);
            let mode = dev.p(2).round() as u8;
            let drv = dev.p(3);
            let mix = dev.p(4);
            let dl = svf(drive(l, drv), &mut st.z[0..2], cut, reso, sr, mode);
            let dr = svf(drive(r, drv), &mut st.z[2..4], cut, reso, sr, mode);
            (l + (dl - l) * mix, r + (dr - r) * mix)
        }
        DeviceKind::Drive => {
            let amt = dev.p(0);
            let mode = dev.p(1).round() as u8;
            let tone = dev.p(2);
            let mix = dev.p(3);
            let out_gain = dev.p(4);
            let sh = |x: f32| -> f32 {
                let g = 1.0 + amt * 14.0;
                match mode {
                    0 => (x * g).tanh(),
                    1 => (x * g).clamp(-1.0, 1.0),
                    _ => fold(x * g),
                }
            };
            let a = sh(l);
            let b = sh(r);
            let a = onepole(a, &mut st.z[0], 500.0 + tone * 15000.0, sr);
            let b = onepole(b, &mut st.z[1], 500.0 + tone * 15000.0, sr);
            let norm = 1.0 / (1.0 + amt * 2.5);
            (
                (l + (a * norm - l) * mix) * out_gain,
                (r + (b * norm - r) * mix) * out_gain,
            )
        }
        DeviceKind::Bitcrush => {
            let bits = dev.p(0).round().clamp(1.0, 16.0);
            let rate = dev.p(1).round().clamp(1.0, 64.0);
            let mix = dev.p(2);
            let out_gain = dev.p(3);
            let steps = 2f32.powf(bits) * 0.5;
            st.z[2] += 1.0;
            if st.z[2] >= rate {
                st.z[2] = 0.0;
                st.z[0] = l;
                st.z[1] = r;
            }
            let ql = (st.z[0] * steps).round() / steps;
            let qr = (st.z[1] * steps).round() / steps;
            (
                (l + (ql - l) * mix) * out_gain,
                (r + (qr - r) * mix) * out_gain,
            )
        }
        DeviceKind::Reverb => {
            let size = dev.p(0);
            let damp = dev.p(1);
            let width = dev.p(2);
            let mix = dev.p(3);
            if st.buf_l.len() < 256 {
                return (l, r);
            }
            let scale = 0.55 + size * 0.8;
            let taps = [0.0297f32, 0.0371, 0.0411, 0.0437];
            let fb = 0.72 + size * 0.24;
            let damp_hz = 900.0 + (1.0 - damp) * 10000.0;
            let mut wl = 0.0;
            let mut wr = 0.0;
            for (i, t) in taps.iter().enumerate() {
                let d = t * sr * scale;
                let al = read_buf(&st.buf_l, st.idx, d);
                let ar = read_buf(&st.buf_r, st.idx, d * 1.013);
                let dl = onepole(al, &mut st.z[i], damp_hz, sr);
                let dr = onepole(ar, &mut st.z[4 + i], damp_hz, sr);
                wl += dl;
                wr += dr;
            }
            // pre-diffusion taps
            let pl = read_buf(&st.buf_l, st.idx, 0.0071 * sr);
            let pr = read_buf(&st.buf_r, st.idx, 0.0093 * sr);
            let inl = (l * 0.35 + wl * 0.24 + pl * 0.3) * fb;
            let inr = (r * 0.35 + wr * 0.24 + pr * 0.3) * fb;
            // The four-tap feedback path has a DC loop gain above unity
            // (about 1.09 with the factory size). A tiny DC term in a track
            // therefore builds up in the delay buffers and can dominate the
            // reverb output. Keep DC out of the feedback path at its source;
            // this is deliberately not a master-bus high-pass.
            let inl = highpass(
                highpass(inl, &mut st.z[8], 20.0, sr),
                &mut st.z[9],
                20.0,
                sr,
            );
            let inr = highpass(
                highpass(inr, &mut st.z[10], 20.0, sr),
                &mut st.z[11],
                20.0,
                sr,
            );
            st.buf_l[st.idx] = inl.tanh();
            st.buf_r[st.idx] = inr.tanh();
            st.idx = (st.idx + 1) % st.buf_l.len();
            let wet_l = (wl * 0.22 + pl * 0.3) * (0.5 + width * 0.5) + wr * 0.05 * (1.0 - width);
            let wet_r = (wr * 0.22 + pr * 0.3) * (0.5 + width * 0.5) + wl * 0.05 * (1.0 - width);
            (l + (wet_l - l) * mix, r + (wet_r - r) * mix)
        }
        DeviceKind::Delay => {
            let time_beats = dev.p(0);
            let fb = dev.p(1);
            let tone = dev.p(2);
            let mix = dev.p(3);
            let ping = dev.p(4) > 0.5;
            if st.buf_l.len() < 256 {
                return (l, r);
            }
            let ds = (time_beats as f64 * 60.0 / bpm.max(20.0) * sr as f64) as f32;
            let dl = read_buf(&st.buf_l, st.idx, ds);
            let dr = read_buf(&st.buf_r, st.idx, ds * 1.005);
            let tl = onepole(dl, &mut st.z[0], 400.0 + tone * 12000.0, sr);
            let tr = onepole(dr, &mut st.z[1], 400.0 + tone * 12000.0, sr);
            if ping {
                st.buf_l[st.idx] = l + tr * fb;
                st.buf_r[st.idx] = r + tl * fb;
            } else {
                st.buf_l[st.idx] = l + tl * fb;
                st.buf_r[st.idx] = r + tr * fb;
            }
            st.idx = (st.idx + 1) % st.buf_l.len();
            (l + (dl - l) * mix, r + (dr - r) * mix)
        }
        DeviceKind::Chorus => {
            let rate = dev.p(0);
            let depth = dev.p(1);
            let mix = dev.p(2);
            let fb = dev.p(3);
            if st.buf_l.len() < 256 {
                return (l, r);
            }
            st.ph = wrap01(st.ph + rate / sr);
            let lfo_l = sine(st.ph);
            let lfo_r = sine(st.ph + 0.25);
            let base = 0.009 * sr;
            let sweep = (0.002 + depth * 0.009) * sr;
            let dl = read_buf(&st.buf_l, st.idx, base + sweep * (0.5 + 0.5 * lfo_l));
            let dr = read_buf(&st.buf_r, st.idx, base + sweep * (0.5 + 0.5 * lfo_r));
            st.buf_l[st.idx] = l + dl * fb;
            st.buf_r[st.idx] = r + dr * fb;
            st.idx = (st.idx + 1) % st.buf_l.len();
            (l + (dl - l) * mix, r + (dr - r) * mix)
        }
        DeviceKind::Compressor => {
            let thresh = dev.p(0);
            let ratio = dev.p(1).max(1.0);
            let atk = dev.p(2);
            let rel = dev.p(3);
            let makeup = db_to_gain(dev.p(4));
            let mix = dev.p(5);
            let peak = l.abs().max(r.abs());
            let coef = if peak > st.env {
                1.0 - (-1.0 / (atk * sr).max(1.0)).exp()
            } else {
                1.0 - (-1.0 / (rel * sr).max(1.0)).exp()
            };
            st.env += (peak - st.env) * coef;
            let db = gain_to_db(st.env.max(1e-5));
            let over = (db - thresh).max(0.0);
            let reduce_db = over - over / ratio;
            let g = db_to_gain(-reduce_db) * makeup;
            (l + (l * g - l) * mix, r + (r * g - r) * mix)
        }
        DeviceKind::Eq3 => {
            let low_g = db_to_gain(dev.p(0));
            let mid_g = db_to_gain(dev.p(1));
            let high_g = db_to_gain(dev.p(2));
            let f = dev.p(3);
            let ll = onepole(l, &mut st.z[0], f, sr);
            let lr = onepole(r, &mut st.z[1], f, sr);
            let hl = highpass(l, &mut st.z[2], f * 3.0, sr);
            let hr = highpass(r, &mut st.z[3], f * 3.0, sr);
            let ml = l - ll - hl;
            let mr = r - lr - hr;
            (
                ll * low_g + ml * mid_g + hl * high_g,
                lr * low_g + mr * mid_g + hr * high_g,
            )
        }
        DeviceKind::Phaser => {
            let rate = dev.p(0);
            let depth = dev.p(1);
            let fb = dev.p(2);
            let mix = dev.p(3);
            let center = dev.p(4);
            st.ph = wrap01(st.ph + rate / sr);
            let lfo = sine(st.ph) * 0.5 + 0.5;
            let f = center * (1.0 + depth * 6.0 * lfo);
            let a = ((TAU * f.min(sr * 0.45) / sr).tan() - 1.0)
                / ((TAU * f.min(sr * 0.45) / sr).tan() + 1.0);
            let mut xl = l + st.z[8] * fb;
            let mut xr = r + st.z[9] * fb;
            for i in 0..4 {
                let yl = -a * xl + st.z[i];
                st.z[i] = xl + a * yl;
                xl = yl;
                let yr = -a * xr + st.z[4 + i];
                st.z[4 + i] = xr + a * yr;
                xr = yr;
            }
            st.z[8] = xl;
            st.z[9] = xr;
            (l + (xl - l) * mix, r + (xr - r) * mix)
        }
        DeviceKind::Sidechain => {
            let div = match dev.p(0).round() as usize {
                0 => 1.0,
                1 => 0.5,
                2 => 2.0,
                _ => 4.0,
            };
            let depth = dev.p(1);
            let atk = dev.p(2).max(0.001);
            let rel = dev.p(3).max(0.01);
            let mix = dev.p(4);
            let phase = ((beat / div).fract() as f32).abs();
            let t = phase * div as f32 * (60.0 / bpm.max(20.0)) as f32;
            let shape = if t < atk {
                t / atk
            } else {
                (-(t - atk) / rel * 3.0).exp()
            };
            let g = 1.0 - depth * (1.0 - shape);
            let g = 1.0 + (g - 1.0) * mix;
            (l * g, r * g)
        }
        DeviceKind::AutoPan => {
            let rate = dev.p(0);
            let depth = dev.p(1);
            let phase_off = dev.p(2);
            let shape = dev.p(3);
            st.ph = wrap01(st.ph + rate / sr);
            let mut lfo = sine(wrap01(st.ph + phase_off));
            if shape > 0.01 {
                let sq = if lfo >= 0.0 { 1.0 } else { -1.0 };
                lfo = lfo * (1.0 - shape) + sq * shape;
            }
            let pan = lfo * depth;
            let (gl, gr) = pan_gains(pan);
            (l * gl * 1.414, r * gr * 1.414)
        }
        DeviceKind::Utility => {
            let gain = db_to_gain(dev.p(0));
            let width = dev.p(1);
            let pan = dev.p(2);
            let mid = (l + r) * 0.5;
            let side = (l - r) * 0.5 * width;
            let (pl, pr) = pan_gains(pan);
            (
                (mid + side) * gain * pl * 1.414,
                (mid - side) * gain * pr * 1.414,
            )
        }
        DeviceKind::Bassify => {
            let amount = dev.p(0);
            let vowel = (dev.p(1).round() as usize).min(FORMANT_TABLE.len() - 1);
            let sub_amt = dev.p(2);
            let growl = dev.p(3);
            let wobble = dev.p(4);
            let formant_scale = dev.p(5);
            let mix = dev.p(6);
            let rate = dev.p(7);

            st.ph = wrap01(st.ph + rate / sr);
            let wob = sine(st.ph);
            let f = FORMANT_TABLE[vowel];
            let bend = formant_scale * (1.0 + 0.16 * wobble * wob);

            let l1 = svf(l, &mut st.z[0..2], f[0] * bend, 0.7, sr, 1);
            let l2 = svf(l, &mut st.z[2..4], f[1] * bend, 0.65, sr, 1);
            let l3 = svf(l, &mut st.z[4..6], f[2] * bend, 0.55, sr, 1);
            let r1 = svf(r, &mut st.z[6..8], f[0] * bend, 0.7, sr, 1);
            let r2 = svf(r, &mut st.z[8..10], f[1] * bend, 0.65, sr, 1);
            let r3 = svf(r, &mut st.z[10..12], f[2] * bend, 0.55, sr, 1);
            let vl = l1 * 1.5 + l2 * 0.9 + l3 * 0.45;
            let vr = r1 * 1.5 + r2 * 0.9 + r3 * 0.45;

            // Take the sub layer from the raw take, not from the formant
            // bands. The formant filters intentionally reject the fundamental;
            // deriving the sub from their output made the effect mid-forward.
            let low_l = onepole(l, &mut st.z[12], 160.0, sr);
            let low_r = onepole(r, &mut st.z[13], 160.0, sr);
            let g = growl * (1.0 + amount * 2.2);
            let sub_gain = 1.0 + sub_amt * 8.0;
            let body_gain = 0.10 + amount * 0.20;
            // Keep a clean sub layer alongside the folded formant body. The
            // fold is what gives the vowel its snarl, but on its own it puts
            // too much energy into the 300–500 Hz harmonics.
            let body_l = fold((vl + low_l * sub_amt * 1.2) * (1.0 + g));
            let body_r = fold((vr + low_r * sub_amt * 1.2) * (1.0 + g));
            let am = 1.0 - wobble * 0.28 * (0.5 + 0.5 * wob);
            let wet_l = low_l * sub_gain + drive(body_l * am, growl * 0.55) * body_gain;
            let wet_r = low_r * sub_gain + drive(body_r * am, growl * 0.55) * body_gain;
            let out_l = l + (wet_l - l) * mix;
            let out_r = r + (wet_r - r) * mix;
            // Folding a vocal produces a slowly moving DC term. A two-pole
            // 30 Hz high-pass removes it without touching the musical sub.
            let out_l = highpass(out_l, &mut st.z[14], 30.0, sr);
            let out_r = highpass(out_r, &mut st.z[15], 30.0, sr);
            (
                highpass(out_l, &mut st.z[16], 30.0, sr),
                highpass(out_r, &mut st.z[17], 30.0, sr),
            )
        }
        DeviceKind::Formant => {
            let vowel = (dev.p(0).round() as usize).min(FORMANT_TABLE.len() - 1);
            let motion = dev.p(1);
            let rate = dev.p(2);
            let reso = dev.p(3).clamp(0.0, 0.95);
            let mix = dev.p(4);
            let output = dev.p(5);

            st.ph = wrap01(st.ph + rate / sr);
            let morph = motion * (0.5 + 0.5 * sine(st.ph));
            let a = FORMANT_TABLE[vowel];
            let b = FORMANT_TABLE[(vowel + 1) % FORMANT_TABLE.len()];
            let f0 = a[0] + (b[0] - a[0]) * morph;
            let f1 = a[1] + (b[1] - a[1]) * morph;
            let f2 = a[2] + (b[2] - a[2]) * morph;
            let q = 0.35 + reso * 0.45;

            let l1 = svf(l, &mut st.z[0..2], f0, q, sr, 1);
            let l2 = svf(l, &mut st.z[2..4], f1, q * 0.9, sr, 1);
            let l3 = svf(l, &mut st.z[4..6], f2, q * 0.8, sr, 1);
            let r1 = svf(r, &mut st.z[6..8], f0, q, sr, 1);
            let r2 = svf(r, &mut st.z[8..10], f1, q * 0.9, sr, 1);
            let r3 = svf(r, &mut st.z[10..12], f2, q * 0.8, sr, 1);

            let wl = drive((l1 + l2 * 0.6 + l3 * 0.35) * 0.55, 0.12);
            let wr = drive((r1 + r2 * 0.6 + r3 * 0.35) * 0.55, 0.12);
            ((l + (wl - l) * mix) * output, (r + (wr - r) * mix) * output)
        }
        DeviceKind::Wobble => {
            let rate = dev.p(0);
            let depth = dev.p(1);
            let shape = dev.p(2);
            let drv = dev.p(3);
            let mix = dev.p(4);
            let output = dev.p(5);
            st.ph = wrap01(st.ph + rate / sr);
            let mut lfo = sine(st.ph);
            if shape > 0.01 {
                let sq = if lfo >= 0.0 { 1.0 } else { -1.0 };
                lfo = lfo * (1.0 - shape) + sq * shape;
            }
            let cut = 220.0 * 2f32.powf(depth * 4.0 * (0.5 + 0.5 * lfo));
            let wl = svf(l, &mut st.z[0..2], cut, 0.55, sr, 0);
            let wr = svf(r, &mut st.z[2..4], cut, 0.55, sr, 0);
            let wl = drive(wl, drv) * output;
            let wr = drive(wr, drv) * output;
            let out_l = highpass(
                highpass(l + (wl - l) * mix, &mut st.z[4], 30.0, sr),
                &mut st.z[6],
                30.0,
                sr,
            );
            let out_r = highpass(
                highpass(r + (wr - r) * mix, &mut st.z[5], 30.0, sr),
                &mut st.z[7],
                30.0,
                sr,
            );
            (out_l, out_r)
        }
        DeviceKind::Tape => {
            let drv = dev.p(0);
            let warmth = dev.p(1);
            let wow = dev.p(2);
            let mix = dev.p(3);
            let output = dev.p(4);
            st.ph = wrap01(st.ph + 0.65 / sr);
            let flutter = 1.0 + wow * 0.06 * sine(st.ph);
            let cut = (12500.0 - warmth * 9800.0) * flutter;
            let a = drive(l, drv * 0.8);
            let b = drive(r, drv * 0.8);
            let a = onepole(a, &mut st.z[0], cut, sr);
            let b = onepole(b, &mut st.z[1], cut, sr);
            let a = a * (1.0 + drv * 0.35) * output;
            let b = b * (1.0 + drv * 0.35) * output;
            (l + (a - l) * mix, r + (b - r) * mix)
        }
        DeviceKind::Transient => {
            let attack = dev.p(0);
            let body = dev.p(1);
            let mix = dev.p(2);
            let output = dev.p(3);
            let peak = l.abs().max(r.abs());
            st.z[1] += (peak - st.z[1]) * 0.018;
            st.z[0] += (peak - st.z[0]) * 0.0007;
            let transient = (st.z[1] - st.z[0]).max(0.0);
            let g = (1.0 + attack * transient * 5.0 + body * st.z[0] * 1.8).max(0.05);
            let g = 1.0 + (g - 1.0) * mix;
            (l * g * output, r * g * output)
        }
        DeviceKind::Multiband => {
            let low_drive = dev.p(0);
            let mid_drive = dev.p(1);
            let high_drive = dev.p(2);
            let crossover = dev.p(3);
            let mix = dev.p(4);
            let output = dev.p(5);
            let low_l = onepole(l, &mut st.z[0], crossover, sr);
            let low_r = onepole(r, &mut st.z[1], crossover, sr);
            let high_l = highpass(l, &mut st.z[2], crossover * 2.2, sr);
            let high_r = highpass(r, &mut st.z[3], crossover * 2.2, sr);
            let mid_l = l - low_l - high_l;
            let mid_r = r - low_r - high_r;
            let wl = drive(low_l, low_drive) + drive(mid_l, mid_drive) + drive(high_l, high_drive);
            let wr = drive(low_r, low_drive) + drive(mid_r, mid_drive) + drive(high_r, high_drive);
            (
                (l + (wl * 0.55 - l) * mix) * output,
                (r + (wr * 0.55 - r) * mix) * output,
            )
        }
        _ => (l, r),
    }
}
