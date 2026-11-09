//! Small, fast DSP primitives shared by every instrument and effect:
//! oscillators, filters, noise, saturation helpers and delay lines.

use std::f32::consts::{PI, TAU};

pub const MAX_VOICES: usize = 16;

#[inline]
pub fn midi_hz(note: f32) -> f32 {
    440.0 * ((note - 69.0) / 12.0).exp2()
}

#[inline]
pub fn wrap01(x: f32) -> f32 {
    x - x.floor()
}

#[inline]
pub fn sine(phase: f32) -> f32 {
    (phase * TAU).sin()
}

/// PolyBLEP band-limited sawtooth.
#[inline]
pub fn saw(phase: f32, dt: f32) -> f32 {
    let mut s = 2.0 * phase - 1.0;
    if dt > 0.0 {
        if phase < dt {
            let t = phase / dt;
            s -= t + t - t * t - 1.0;
        } else if phase > 1.0 - dt {
            let t = (phase - 1.0) / dt;
            s -= t * t + t + t + 1.0;
        }
    }
    s
}

#[inline]
pub fn square(phase: f32, dt: f32) -> f32 {
    saw(phase, dt) - saw(wrap01(phase + 0.5), dt)
}

#[derive(Clone)]
pub struct Rng(pub u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed.wrapping_mul(2654435761).wrapping_add(1) | 1)
    }
    #[inline]
    pub fn noise(&mut self) -> f32 {
        // xorshift32
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
    #[inline]
    pub fn uni(&mut self) -> f32 {
        (self.noise() + 1.0) * 0.5
    }
}

/// Four-pole ladder lowpass with resonance; writes through `st` (4 floats).
#[inline]
pub fn ladder(input: f32, st: &mut [f32], cutoff: f32, reso: f32, sr: f32) -> f32 {
    let f = cutoff.clamp(20.0, sr * 0.45);
    let g = (PI * f / sr).tan().min(1.4);
    let a = g / (1.0 + g);
    let fb = reso.clamp(0.0, 0.99) * 3.8 * st[3];
    let x = (input - fb).tanh();
    st[0] += a * (x - st[0]);
    st[1] += a * (st[0] - st[1]);
    st[2] += a * (st[1] - st[2]);
    st[3] += a * (st[2] - st[3]);
    st[3]
}

/// State-variable filter with selectable response (0=LP, 1=BP, 2=HP).
/// `st` holds two floats of state.
#[inline]
pub fn svf(x: f32, st: &mut [f32], cutoff: f32, reso: f32, sr: f32, mode: u8) -> f32 {
    let f = cutoff.clamp(20.0, sr * 0.42);
    let g = (PI * f / sr).tan().min(1.2);
    let k = 2.0 - 1.96 * reso.clamp(0.0, 0.99);
    let a1 = 1.0 / (1.0 + g * (g + k));
    let a2 = g * a1;
    let a3 = g * a2;
    let v3 = x - st[1];
    let v1 = a1 * st[0] + a2 * v3;
    let v2 = st[1] + a2 * st[0] + a3 * v3;
    st[0] = 2.0 * v1 - st[0];
    st[1] = 2.0 * v2 - st[1];
    match mode {
        0 => v2,
        1 => v1,
        _ => x - k * v1 - v2,
    }
}

#[inline]
pub fn onepole(x: f32, z: &mut f32, hz: f32, sr: f32) -> f32 {
    let a = (1.0 - (-TAU * hz.clamp(1.0, sr * 0.49) / sr).exp()).clamp(0.0, 1.0);
    *z += a * (x - *z);
    *z
}

#[inline]
pub fn highpass(x: f32, z: &mut f32, hz: f32, sr: f32) -> f32 {
    x - onepole(x, z, hz, sr)
}

/// Soft saturation that stays loud but never clips hard.
#[inline]
pub fn sat(x: f32) -> f32 {
    let a = x * 1.3;
    a.tanh()
}

#[inline]
pub fn drive(x: f32, amount: f32) -> f32 {
    let g = 1.0 + amount * 18.0;
    (x * g).tanh() / (1.0 + amount * 4.0).sqrt()
}

#[inline]
pub fn fold(x: f32) -> f32 {
    let mut v = x;
    for _ in 0..2 {
        if v > 1.0 {
            v = 2.0 - v;
        } else if v < -1.0 {
            v = -2.0 - v;
        }
    }
    v.clamp(-1.0, 1.0)
}

#[inline]
pub fn db_to_gain(db: f32) -> f32 {
    (db / 20.0 * std::f32::consts::LN_10).exp()
}

#[inline]
pub fn gain_to_db(g: f32) -> f32 {
    20.0 * (g.max(1e-6)).log10()
}

/// Simple fractional delay line. Buffer is pre-allocated at construction.
pub struct Delay {
    pub buf: Vec<f32>,
    pub idx: usize,
}

impl Delay {
    pub fn new(len: usize) -> Self {
        Self {
            buf: vec![0.0; len.max(4)],
            idx: 0,
        }
    }

    #[inline]
    pub fn write(&mut self, x: f32) {
        self.idx = (self.idx + 1) % self.buf.len();
        self.buf[self.idx] = x;
    }

    #[inline]
    pub fn read(&self, delay_samples: f32) -> f32 {
        let len = self.buf.len() as f32;
        let d = delay_samples.clamp(1.0, len - 2.0);
        let pos = self.idx as f32 - d;
        let pos = if pos < 0.0 { pos + len } else { pos };
        let i0 = pos.floor() as usize;
        let frac = pos - i0 as f32;
        let i1 = (i0 + 1) % self.buf.len();
        self.buf[i0] * (1.0 - frac) + self.buf[i1] * frac
    }
}

/// Equal-power pan; returns (left, right).
#[inline]
pub fn pan_gains(pan: f32) -> (f32, f32) {
    let p = (pan.clamp(-1.0, 1.0) + 1.0) * 0.25 * PI;
    (p.cos(), p.sin())
}

/// Attack / decay / sustain / release envelope, evaluated against voice age
/// and gate. Returns the amplitude and reports whether the voice is finished.
#[inline]
pub fn amp_env(
    age: f32,
    released_at: f32,
    gate: bool,
    a: f32,
    d: f32,
    s: f32,
    r: f32,
) -> (f32, bool) {
    let a = a.max(0.0005);
    let mut env;
    if age < a {
        env = age / a;
    } else {
        let k = 1.0 - (-(age - a) / d.max(0.005) * 3.0).exp();
        env = 1.0 + (s - 1.0) * k;
    }
    let mut done = false;
    if !gate {
        let rt = (age - released_at).max(0.0);
        let fade = (-rt / r.max(0.005) * 4.0).exp();
        env *= fade;
        if rt > r.max(0.005) * 1.6 {
            done = true;
        }
    }
    (env, done)
}

/// Smooth parameter ramp helper.
#[inline]
pub fn smooth(current: &mut f32, target: f32, coef: f32) -> f32 {
    *current += (target - *current) * coef;
    *current
}
