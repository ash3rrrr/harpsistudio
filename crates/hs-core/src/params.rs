//! Device catalogue: every instrument and effect Harpsistudio ships with,
//! their parameter definitions, defaults, colours and factory presets.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum DeviceKind {
    // instruments
    DrumRack,
    SubBass,
    ReeseBass,
    GrowlBass,
    Supersaw,
    Pluck,
    Keys,
    Organ,
    Pad,
    Riser,
    Salamander,
    VoxBass,
    VocalTake,
    // MIDI-generating devices
    Arpeggiator,
    ChordGenerator,
    // effects
    Filter,
    Drive,
    Bitcrush,
    Reverb,
    Delay,
    Chorus,
    Compressor,
    Eq3,
    Phaser,
    Sidechain,
    AutoPan,
    Utility,
    Bassify,
    Formant,
    Wobble,
    Tape,
    Transient,
    Multiband,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Fmt {
    Float(&'static str),
    Percent,
    Db,
    Hz,
    Sec,
    Semis,
    Beats,
    Enum(&'static [&'static str]),
    Int(&'static str),
}

#[derive(Clone, Copy)]
pub struct ParamDef {
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    pub def: f32,
    pub step: f32,
    pub fmt: Fmt,
    pub bipolar: bool,
}

impl ParamDef {
    pub const fn new(
        name: &'static str,
        min: f32,
        max: f32,
        def: f32,
        step: f32,
        fmt: Fmt,
    ) -> Self {
        Self {
            name,
            min,
            max,
            def,
            step,
            fmt,
            bipolar: false,
        }
    }
    pub const fn bp(mut self) -> Self {
        self.bipolar = true;
        self
    }

    pub fn format(&self, v: f32) -> String {
        match self.fmt {
            Fmt::Float(u) => {
                if u.is_empty() {
                    format!("{:.2}", v)
                } else {
                    format!("{:.2}{}", v, u)
                }
            }
            Fmt::Percent => format!("{:.0}%", v * 100.0),
            Fmt::Db => format!("{:+.1} dB", v),
            Fmt::Hz => {
                if v >= 1000.0 {
                    format!("{:.2} kHz", v / 1000.0)
                } else if v < 1.0 {
                    // Slow LFO rates need decimals; rounding 0.35 Hz to "0 Hz"
                    // makes modulation look switched off in the device view.
                    format!("{:.2} Hz", v)
                } else {
                    format!("{:.0} Hz", v)
                }
            }
            Fmt::Sec => {
                if v < 1.0 {
                    format!("{:.1} ms", v * 1000.0)
                } else {
                    format!("{:.2} s", v)
                }
            }
            Fmt::Semis => format!("{:+.1} st", v),
            Fmt::Beats => format!("{:.3} bt", v),
            Fmt::Enum(names) => {
                let i = (v.round() as usize).min(names.len().saturating_sub(1));
                names.get(i).copied().unwrap_or("").to_string()
            }
            Fmt::Int(u) => {
                if u.is_empty() {
                    format!("{:.0}", v)
                } else {
                    format!("{:.0}{}", v, u)
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct Preset {
    pub name: &'static str,
    pub values: &'static [f32],
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Device {
    pub kind: DeviceKind,
    pub params: Vec<f32>,
    pub on: bool,
}

impl Device {
    pub fn new(kind: DeviceKind) -> Self {
        let params = kind.defs().iter().map(|d| d.def).collect();
        Self {
            kind,
            params,
            on: true,
        }
    }

    pub fn from_preset(kind: DeviceKind, preset: &Preset) -> Self {
        let mut d = Device::new(kind);
        d.load(preset);
        d
    }

    pub fn defs(&self) -> &'static [ParamDef] {
        self.kind.defs()
    }

    pub fn p(&self, i: usize) -> f32 {
        let def = &self.defs()[i];
        self.params.get(i).copied().unwrap_or(def.def)
    }

    pub fn set(&mut self, i: usize, v: f32) {
        if let Some(def) = self.defs().get(i) {
            if i >= self.params.len() {
                // Older projects may predate newly added controls (for
                // example the Drum Rack Kit selector). Grow in place so the
                // control still works after loading one.
                self.params.resize(i + 1, def.def);
            }
            self.params[i] = v.clamp(def.min, def.max);
        }
    }

    pub fn norm(&self, i: usize) -> f32 {
        let def = &self.defs()[i];
        let v = self.p(i);
        if def.max - def.min < 1e-9 {
            0.0
        } else {
            ((v - def.min) / (def.max - def.min)).clamp(0.0, 1.0)
        }
    }

    pub fn set_norm(&mut self, i: usize, n: f32) {
        let def = &self.defs()[i];
        let mut v = def.min + (def.max - def.min) * n.clamp(0.0, 1.0);
        if def.step > 0.0 {
            v = (v / def.step).round() * def.step;
        }
        if matches!(def.fmt, Fmt::Enum(_) | Fmt::Int(_)) {
            v = v.round();
        }
        self.set(i, v);
    }

    pub fn load(&mut self, preset: &Preset) {
        for (i, v) in preset.values.iter().enumerate() {
            self.set(i, *v);
        }
    }

    pub fn label(&self) -> String {
        self.kind.name().to_string()
    }
}

const fn p(name: &'static str, min: f32, max: f32, def: f32, step: f32, fmt: Fmt) -> ParamDef {
    ParamDef::new(name, min, max, def, step, fmt)
}

const LP_MODES: &[&str] = &["Lowpass", "Bandpass", "Highpass"];
const DRIVE_MODES: &[&str] = &["Soft", "Hard", "Fold"];
const DIVISIONS: &[&str] = &["1/4", "1/8", "1/2", "1 Bar"];
const ARP_RATES: &[&str] = &["1/4", "1/8", "1/16", "1/32"];
const ARP_MODES: &[&str] = &["Up", "Down", "Up-Down", "Random"];
const MIDI_KEYS: &[&str] = &[
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
const MIDI_SCALES: &[&str] = &["Major", "Minor", "Dorian", "Mixolydian", "Phrygian"];
const CHORD_QUALITIES: &[&str] = &["Diatonic", "Major", "Minor", "7th", "Sus4", "Add9"];
const VOWELS: &[&str] = &["A", "E", "I", "O", "U"];
pub const DRUM_KIT_NAMES: &[&str] = &crate::samples::KIT_NAMES;

static DEF_DRUM_RACK: [ParamDef; 9] = [
    p("Level", 0.0, 3.0, 0.85, 0.0, Fmt::Percent),
    p("Tune", -24.0, 24.0, 0.0, 0.0, Fmt::Semis).bp(),
    p("Decay", 0.05, 2.0, 0.6, 0.0, Fmt::Sec),
    p("Punch", 0.0, 1.0, 0.55, 0.0, Fmt::Percent),
    p("Snap", 0.0, 1.0, 0.55, 0.0, Fmt::Percent),
    p("Drive", 0.0, 1.0, 0.2, 0.0, Fmt::Percent),
    p("Width", 0.0, 1.0, 0.6, 0.0, Fmt::Percent),
    p("Tone", 0.0, 1.0, 0.5, 0.0, Fmt::Percent),
    p(
        "Kit",
        0.0,
        (crate::samples::KIT_NAMES.len() - 1) as f32,
        0.0,
        1.0,
        Fmt::Enum(DRUM_KIT_NAMES),
    ),
];
static DEF_SALAMANDER: [ParamDef; 8] = [
    p("Level", 0.0, 3.0, 0.95, 0.0, Fmt::Percent),
    p("Tone", 0.0, 1.0, 0.62, 0.0, Fmt::Percent),
    p("Release", 0.02, 4.0, 0.55, 0.0, Fmt::Sec),
    p("Detune", -0.25, 0.25, 0.0, 0.0, Fmt::Semis).bp(),
    p("Octave", -2.0, 2.0, 0.0, 1.0, Fmt::Int(" oct")),
    p("Vel Sense", 0.0, 1.0, 0.8, 0.0, Fmt::Percent),
    p("Width", 0.0, 1.0, 0.5, 0.0, Fmt::Percent),
    p("Attack", 0.0, 0.08, 0.002, 0.0, Fmt::Sec),
];
static DEF_VOX_BASS: [ParamDef; 8] = [
    p("Level", 0.0, 3.0, 0.85, 0.0, Fmt::Percent),
    p("Vowel", 0.0, 4.0, 2.0, 1.0, Fmt::Enum(VOWELS)),
    p("Sub", 0.0, 1.0, 0.55, 0.0, Fmt::Percent),
    p("Growl", 0.0, 1.0, 0.5, 0.0, Fmt::Percent),
    p("Wobble", 0.25, 16.0, 4.0, 0.0, Fmt::Hz),
    p("Wob Amt", 0.0, 1.0, 0.55, 0.0, Fmt::Percent),
    p("Drive", 0.0, 1.0, 0.35, 0.0, Fmt::Percent),
    p("Release", 0.01, 2.0, 0.16, 0.0, Fmt::Sec),
];
static DEF_VOCAL_TAKE: [ParamDef; 6] = [
    p("Level", 0.0, 3.0, 0.95, 0.0, Fmt::Percent),
    p("Pitch", -24.0, 12.0, -12.0, 0.0, Fmt::Semis).bp(),
    p("Start", 0.0, 10.0, 0.0, 0.0, Fmt::Sec),
    p("Tone", 0.0, 1.0, 0.72, 0.0, Fmt::Percent),
    p("Release", 0.01, 2.0, 0.18, 0.0, Fmt::Sec),
    p("Reverse", 0.0, 1.0, 0.0, 1.0, Fmt::Enum(&["Off", "On"])),
];
static DEF_ARPEGGIATOR: [ParamDef; 5] = [
    p("Rate", 0.0, 3.0, 2.0, 1.0, Fmt::Enum(ARP_RATES)),
    p("Mode", 0.0, 3.0, 0.0, 1.0, Fmt::Enum(ARP_MODES)),
    p("Octaves", 1.0, 4.0, 2.0, 1.0, Fmt::Int(" oct")),
    p("Gate", 0.05, 1.0, 0.55, 0.0, Fmt::Percent),
    p("Swing", 0.0, 0.6, 0.0, 0.0, Fmt::Percent),
];
static DEF_CHORD_GENERATOR: [ParamDef; 5] = [
    p("Key", 0.0, 11.0, 0.0, 1.0, Fmt::Enum(MIDI_KEYS)),
    p("Scale", 0.0, 4.0, 0.0, 1.0, Fmt::Enum(MIDI_SCALES)),
    p("Quality", 0.0, 5.0, 0.0, 1.0, Fmt::Enum(CHORD_QUALITIES)),
    p("Inversion", 0.0, 3.0, 0.0, 1.0, Fmt::Int("")),
    p("Octave", -2.0, 2.0, 0.0, 1.0, Fmt::Int(" oct")),
];
static DEF_BASSIFY: [ParamDef; 8] = [
    p("Amount", 0.0, 1.0, 0.72, 0.0, Fmt::Percent),
    p("Vowel", 0.0, 4.0, 2.0, 1.0, Fmt::Enum(VOWELS)),
    p("Sub", 0.0, 1.0, 0.45, 0.0, Fmt::Percent),
    p("Growl", 0.0, 1.0, 0.5, 0.0, Fmt::Percent),
    p("Wobble", 0.0, 1.0, 0.55, 0.0, Fmt::Percent),
    p("Formant", 0.25, 2.0, 1.0, 0.0, Fmt::Float("x")),
    p("Mix", 0.0, 1.0, 0.85, 0.0, Fmt::Percent),
    p("Rate", 0.25, 12.0, 4.0, 0.0, Fmt::Hz),
];
static DEF_FORMANT: [ParamDef; 6] = [
    p("Vowel", 0.0, 4.0, 2.0, 1.0, Fmt::Enum(VOWELS)),
    p("Motion", 0.0, 1.0, 0.35, 0.0, Fmt::Percent),
    p("Rate", 0.05, 8.0, 0.6, 0.0, Fmt::Hz),
    p("Reso", 0.0, 1.0, 0.55, 0.0, Fmt::Percent),
    p("Mix", 0.0, 1.0, 0.85, 0.0, Fmt::Percent),
    p("Output", 0.0, 2.0, 0.9, 0.0, Fmt::Percent),
];
static DEF_WOBBLE: [ParamDef; 6] = [
    p("Rate", 0.25, 16.0, 4.0, 0.0, Fmt::Hz),
    p("Depth", 0.0, 1.0, 0.7, 0.0, Fmt::Percent),
    p("Shape", 0.0, 1.0, 0.3, 0.0, Fmt::Percent),
    p("Drive", 0.0, 1.0, 0.25, 0.0, Fmt::Percent),
    p("Mix", 0.0, 1.0, 0.8, 0.0, Fmt::Percent),
    p("Output", 0.0, 2.0, 0.9, 0.0, Fmt::Percent),
];
static DEF_TAPE: [ParamDef; 5] = [
    p("Drive", 0.0, 1.0, 0.35, 0.0, Fmt::Percent),
    p("Warmth", 0.0, 1.0, 0.55, 0.0, Fmt::Percent),
    p("Wow", 0.0, 1.0, 0.2, 0.0, Fmt::Percent),
    p("Mix", 0.0, 1.0, 0.7, 0.0, Fmt::Percent),
    p("Output", 0.0, 2.0, 0.9, 0.0, Fmt::Percent),
];
static DEF_TRANSIENT: [ParamDef; 4] = [
    p("Attack", -1.0, 1.0, 0.35, 0.0, Fmt::Percent).bp(),
    p("Body", -1.0, 1.0, 0.2, 0.0, Fmt::Percent).bp(),
    p("Mix", 0.0, 1.0, 0.65, 0.0, Fmt::Percent),
    p("Output", 0.0, 2.0, 0.9, 0.0, Fmt::Percent),
];
static DEF_MULTIBAND: [ParamDef; 6] = [
    p("Low Drive", 0.0, 1.0, 0.25, 0.0, Fmt::Percent),
    p("Mid Drive", 0.0, 1.0, 0.4, 0.0, Fmt::Percent),
    p("High Drive", 0.0, 1.0, 0.2, 0.0, Fmt::Percent),
    p("Crossover", 200.0, 5000.0, 1200.0, 0.0, Fmt::Hz),
    p("Mix", 0.0, 1.0, 0.8, 0.0, Fmt::Percent),
    p("Output", 0.0, 2.0, 0.9, 0.0, Fmt::Percent),
];
static DEF_SUB_BASS: [ParamDef; 8] = [
    p("Level", 0.0, 3.0, 0.9, 0.0, Fmt::Percent),
    p("Tune", -24.0, 24.0, 0.0, 0.0, Fmt::Semis).bp(),
    p("Attack", 0.001, 0.5, 0.005, 0.0, Fmt::Sec),
    p("Decay", 0.01, 2.0, 0.4, 0.0, Fmt::Sec),
    p("Sustain", 0.0, 1.0, 0.8, 0.0, Fmt::Percent),
    p("Release", 0.01, 3.0, 0.12, 0.0, Fmt::Sec),
    p("Drive", 0.0, 1.0, 0.15, 0.0, Fmt::Percent),
    p("Glide", 0.0, 0.3, 0.0, 0.0, Fmt::Sec),
];
static DEF_REESE_BASS: [ParamDef; 8] = [
    p("Level", 0.0, 3.0, 0.85, 0.0, Fmt::Percent),
    p("Cutoff", 20.0, 16000.0, 700.0, 0.0, Fmt::Hz),
    p("Reso", 0.0, 1.0, 0.35, 0.0, Fmt::Percent),
    p("Detune", 0.0, 0.08, 0.018, 0.0, Fmt::Float("")),
    p("LFO Rate", 0.05, 8.0, 1.2, 0.0, Fmt::Hz),
    p("LFO Amt", 0.0, 1.0, 0.35, 0.0, Fmt::Percent),
    p("Drive", 0.0, 1.0, 0.3, 0.0, Fmt::Percent),
    p("Attack", 0.001, 0.5, 0.01, 0.0, Fmt::Sec),
];
static DEF_GROWL_BASS: [ParamDef; 8] = [
    p("Level", 0.0, 3.0, 0.85, 0.0, Fmt::Percent),
    p("Cutoff", 80.0, 8000.0, 900.0, 0.0, Fmt::Hz),
    p("Reso", 0.0, 1.0, 0.45, 0.0, Fmt::Percent),
    p("Wobble", 0.25, 16.0, 4.0, 0.0, Fmt::Hz),
    p("Wob Amt", 0.0, 1.0, 0.8, 0.0, Fmt::Percent),
    p("FM Amt", 0.0, 1.0, 0.6, 0.0, Fmt::Percent),
    p("Drive", 0.0, 1.0, 0.35, 0.0, Fmt::Percent),
    p("Attack", 0.001, 0.5, 0.005, 0.0, Fmt::Sec),
];
static DEF_SUPERSAW: [ParamDef; 8] = [
    p("Level", 0.0, 3.0, 0.8, 0.0, Fmt::Percent),
    p("Detune", 0.0, 0.1, 0.025, 0.0, Fmt::Float("")),
    p("Cutoff", 80.0, 18000.0, 6000.0, 0.0, Fmt::Hz),
    p("Reso", 0.0, 1.0, 0.15, 0.0, Fmt::Percent),
    p("Attack", 0.001, 1.0, 0.01, 0.0, Fmt::Sec),
    p("Release", 0.01, 3.0, 0.35, 0.0, Fmt::Sec),
    p("Octave", -2.0, 2.0, 0.0, 1.0, Fmt::Int(" oct")),
    p("Width", 0.0, 1.0, 0.7, 0.0, Fmt::Percent),
];
static DEF_PLUCK: [ParamDef; 8] = [
    p("Level", 0.0, 3.0, 0.8, 0.0, Fmt::Percent),
    p("Cutoff", 200.0, 18000.0, 4200.0, 0.0, Fmt::Hz),
    p("Reso", 0.0, 1.0, 0.2, 0.0, Fmt::Percent),
    p("Decay", 0.03, 1.5, 0.28, 0.0, Fmt::Sec),
    p("Detune", 0.0, 0.06, 0.014, 0.0, Fmt::Float("")),
    p("Punch", 0.0, 1.0, 0.6, 0.0, Fmt::Percent),
    p("Attack", 0.0, 0.1, 0.002, 0.0, Fmt::Sec),
    p("Octave", -1.0, 2.0, 0.0, 1.0, Fmt::Int(" oct")),
];
static DEF_KEYS: [ParamDef; 8] = [
    p("Level", 0.0, 3.0, 0.8, 0.0, Fmt::Percent),
    p("Ratio", 0.5, 12.0, 2.0, 0.0, Fmt::Float(":1")),
    p("FM Amt", 0.0, 1.0, 0.35, 0.0, Fmt::Percent),
    p("Attack", 0.001, 1.0, 0.006, 0.0, Fmt::Sec),
    p("Decay", 0.02, 3.0, 1.1, 0.0, Fmt::Sec),
    p("Sustain", 0.0, 1.0, 0.2, 0.0, Fmt::Percent),
    p("Release", 0.01, 3.0, 0.4, 0.0, Fmt::Sec),
    p("Tone", 0.0, 1.0, 0.55, 0.0, Fmt::Percent),
];
static DEF_ORGAN: [ParamDef; 8] = [
    p("Level", 0.0, 3.0, 0.8, 0.0, Fmt::Percent),
    p("Drawbars", 0.0, 1.0, 0.55, 0.0, Fmt::Percent),
    p("Click", 0.0, 1.0, 0.3, 0.0, Fmt::Percent),
    p("Rotary", 0.0, 1.0, 0.3, 0.0, Fmt::Percent),
    p("Drive", 0.0, 1.0, 0.15, 0.0, Fmt::Percent),
    p("Attack", 0.001, 0.1, 0.004, 0.0, Fmt::Sec),
    p("Release", 0.02, 2.0, 0.18, 0.0, Fmt::Sec),
    p("Octave", -2.0, 2.0, 0.0, 1.0, Fmt::Int(" oct")),
];
static DEF_PAD: [ParamDef; 8] = [
    p("Level", 0.0, 3.0, 0.7, 0.0, Fmt::Percent),
    p("Attack", 0.01, 5.0, 1.0, 0.0, Fmt::Sec),
    p("Release", 0.05, 8.0, 2.2, 0.0, Fmt::Sec),
    p("Cutoff", 80.0, 14000.0, 1800.0, 0.0, Fmt::Hz),
    p("Reso", 0.0, 1.0, 0.18, 0.0, Fmt::Percent),
    p("Detune", 0.0, 0.08, 0.03, 0.0, Fmt::Float("")),
    p("Octave", -2.0, 2.0, 0.0, 1.0, Fmt::Int(" oct")),
    p("Movement", 0.0, 1.0, 0.4, 0.0, Fmt::Percent),
];
static DEF_RISER: [ParamDef; 8] = [
    p("Level", 0.0, 3.0, 0.65, 0.0, Fmt::Percent),
    p("Length", 0.25, 16.0, 8.0, 0.0, Fmt::Beats),
    p("Rise", 0.0, 1.0, 0.8, 0.0, Fmt::Percent),
    p("Cutoff", 100.0, 16000.0, 600.0, 0.0, Fmt::Hz),
    p("Reso", 0.0, 1.0, 0.3, 0.0, Fmt::Percent),
    p("Noise", 0.0, 1.0, 0.6, 0.0, Fmt::Percent),
    p("Attack", 0.01, 8.0, 1.0, 0.0, Fmt::Sec),
    p("Release", 0.01, 4.0, 0.6, 0.0, Fmt::Sec),
];
static DEF_FILTER: [ParamDef; 5] = [
    p("Cutoff", 20.0, 20000.0, 1200.0, 0.0, Fmt::Hz),
    p("Reso", 0.0, 1.0, 0.2, 0.0, Fmt::Percent),
    p("Type", 0.0, 2.0, 0.0, 1.0, Fmt::Enum(LP_MODES)),
    p("Drive", 0.0, 1.0, 0.1, 0.0, Fmt::Percent),
    p("Mix", 0.0, 1.0, 1.0, 0.0, Fmt::Percent),
];
static DEF_DRIVE: [ParamDef; 5] = [
    p("Drive", 0.0, 1.0, 0.4, 0.0, Fmt::Percent),
    p("Mode", 0.0, 2.0, 0.0, 1.0, Fmt::Enum(DRIVE_MODES)),
    p("Tone", 0.0, 1.0, 0.6, 0.0, Fmt::Percent),
    p("Mix", 0.0, 1.0, 1.0, 0.0, Fmt::Percent),
    p("Output", 0.0, 2.0, 0.8, 0.0, Fmt::Percent),
];
static DEF_BITCRUSH: [ParamDef; 4] = [
    p("Bits", 1.0, 16.0, 10.0, 1.0, Fmt::Int(" bit")),
    p("Rate", 1.0, 64.0, 4.0, 1.0, Fmt::Int("x")),
    p("Mix", 0.0, 1.0, 0.8, 0.0, Fmt::Percent),
    p("Output", 0.0, 2.0, 0.8, 0.0, Fmt::Percent),
];
static DEF_REVERB: [ParamDef; 4] = [
    p("Size", 0.0, 1.0, 0.6, 0.0, Fmt::Percent),
    p("Damp", 0.0, 1.0, 0.4, 0.0, Fmt::Percent),
    p("Width", 0.0, 1.0, 0.8, 0.0, Fmt::Percent),
    p("Mix", 0.0, 1.0, 0.3, 0.0, Fmt::Percent),
];
static DEF_DELAY: [ParamDef; 5] = [
    p("Time", 0.0625, 2.0, 0.5, 0.0, Fmt::Beats),
    p("Feedback", 0.0, 0.95, 0.4, 0.0, Fmt::Percent),
    p("Tone", 0.0, 1.0, 0.5, 0.0, Fmt::Percent),
    p("Mix", 0.0, 1.0, 0.25, 0.0, Fmt::Percent),
    p("PingPong", 0.0, 1.0, 1.0, 1.0, Fmt::Enum(&["Off", "On"])),
];
static DEF_CHORUS: [ParamDef; 4] = [
    p("Rate", 0.05, 8.0, 0.5, 0.0, Fmt::Hz),
    p("Depth", 0.0, 1.0, 0.35, 0.0, Fmt::Percent),
    p("Mix", 0.0, 1.0, 0.35, 0.0, Fmt::Percent),
    p("Feedback", 0.0, 0.9, 0.15, 0.0, Fmt::Percent),
];
static DEF_COMPRESSOR: [ParamDef; 6] = [
    p("Threshold", -60.0, 0.0, -18.0, 0.0, Fmt::Db),
    p("Ratio", 1.0, 20.0, 4.0, 0.0, Fmt::Float(":1")),
    p("Attack", 0.0005, 0.2, 0.01, 0.0, Fmt::Sec),
    p("Release", 0.01, 1.0, 0.12, 0.0, Fmt::Sec),
    p("Makeup", 0.0, 24.0, 3.0, 0.0, Fmt::Db),
    p("Mix", 0.0, 1.0, 1.0, 0.0, Fmt::Percent),
];
static DEF_EQ3: [ParamDef; 4] = [
    p("Low", -18.0, 18.0, 0.0, 0.0, Fmt::Db).bp(),
    p("Mid", -18.0, 18.0, 0.0, 0.0, Fmt::Db).bp(),
    p("High", -18.0, 18.0, 0.0, 0.0, Fmt::Db).bp(),
    p("Freq", 200.0, 6000.0, 1000.0, 0.0, Fmt::Hz),
];
static DEF_PHASER: [ParamDef; 5] = [
    p("Rate", 0.05, 5.0, 0.5, 0.0, Fmt::Hz),
    p("Depth", 0.0, 1.0, 0.7, 0.0, Fmt::Percent),
    p("Feedback", 0.0, 0.9, 0.4, 0.0, Fmt::Percent),
    p("Mix", 0.0, 1.0, 0.5, 0.0, Fmt::Percent),
    p("Center", 200.0, 5000.0, 900.0, 0.0, Fmt::Hz),
];
static DEF_SIDECHAIN: [ParamDef; 5] = [
    p("Rate", 0.0, 3.0, 0.0, 1.0, Fmt::Enum(DIVISIONS)),
    p("Depth", 0.0, 1.0, 0.7, 0.0, Fmt::Percent),
    p("Attack", 0.001, 0.2, 0.01, 0.0, Fmt::Sec),
    p("Release", 0.02, 1.0, 0.2, 0.0, Fmt::Sec),
    p("Mix", 0.0, 1.0, 1.0, 0.0, Fmt::Percent),
];
static DEF_AUTO_PAN: [ParamDef; 4] = [
    p("Rate", 0.02, 8.0, 0.25, 0.0, Fmt::Hz),
    p("Depth", 0.0, 1.0, 0.8, 0.0, Fmt::Percent),
    p("Phase", 0.0, 1.0, 0.0, 0.0, Fmt::Percent),
    p("Shape", 0.0, 1.0, 0.5, 0.0, Fmt::Percent),
];
static DEF_UTILITY: [ParamDef; 3] = [
    p("Gain", -24.0, 24.0, 0.0, 0.0, Fmt::Db).bp(),
    p("Width", 0.0, 2.0, 1.0, 0.0, Fmt::Percent),
    p("Pan", -1.0, 1.0, 0.0, 0.0, Fmt::Percent).bp(),
];

impl DeviceKind {
    pub const ALL: [DeviceKind; 33] = [
        DeviceKind::DrumRack,
        DeviceKind::SubBass,
        DeviceKind::ReeseBass,
        DeviceKind::GrowlBass,
        DeviceKind::Supersaw,
        DeviceKind::Pluck,
        DeviceKind::Keys,
        DeviceKind::Organ,
        DeviceKind::Pad,
        DeviceKind::Riser,
        DeviceKind::Salamander,
        DeviceKind::VoxBass,
        DeviceKind::VocalTake,
        DeviceKind::Arpeggiator,
        DeviceKind::ChordGenerator,
        DeviceKind::Filter,
        DeviceKind::Drive,
        DeviceKind::Bitcrush,
        DeviceKind::Reverb,
        DeviceKind::Delay,
        DeviceKind::Chorus,
        DeviceKind::Compressor,
        DeviceKind::Eq3,
        DeviceKind::Phaser,
        DeviceKind::Sidechain,
        DeviceKind::AutoPan,
        DeviceKind::Utility,
        DeviceKind::Bassify,
        DeviceKind::Formant,
        DeviceKind::Wobble,
        DeviceKind::Tape,
        DeviceKind::Transient,
        DeviceKind::Multiband,
    ];

    pub fn is_instrument(self) -> bool {
        matches!(
            self,
            DeviceKind::DrumRack
                | DeviceKind::SubBass
                | DeviceKind::ReeseBass
                | DeviceKind::GrowlBass
                | DeviceKind::Supersaw
                | DeviceKind::Pluck
                | DeviceKind::Keys
                | DeviceKind::Organ
                | DeviceKind::Pad
                | DeviceKind::Riser
                | DeviceKind::Salamander
                | DeviceKind::VoxBass
                | DeviceKind::VocalTake
        )
    }

    /// Note-generating devices live in the track chain but do not render audio.
    pub fn is_midi(self) -> bool {
        matches!(self, DeviceKind::Arpeggiator | DeviceKind::ChordGenerator)
    }

    pub fn is_audio_effect(self) -> bool {
        !self.is_instrument() && !self.is_midi()
    }

    pub fn category(self) -> &'static str {
        if self.is_instrument() {
            "INSTRUMENTS"
        } else if self.is_midi() {
            "MIDI FX"
        } else {
            "EFFECTS"
        }
    }

    pub fn name(self) -> &'static str {
        use DeviceKind::*;
        match self {
            DrumRack => "Drum Rack",
            SubBass => "Sub Bass",
            ReeseBass => "Reese Bass",
            GrowlBass => "Growl Bass",
            Supersaw => "Supersaw",
            Pluck => "House Pluck",
            Keys => "FM Keys",
            Organ => "Tonewheel Organ",
            Pad => "Wide Pad",
            Riser => "Riser FX",
            Salamander => "Salamander Grand",
            VoxBass => "Vox Bass",
            VocalTake => "Vocal Take",
            Arpeggiator => "Arpeggiator",
            ChordGenerator => "Chord Generator",
            Filter => "Filter",
            Drive => "Drive",
            Bitcrush => "Bitcrush",
            Reverb => "Reverb",
            Delay => "Delay",
            Chorus => "Chorus",
            Compressor => "Compressor",
            Eq3 => "EQ Three",
            Phaser => "Phaser",
            Sidechain => "Sidechain",
            AutoPan => "Auto Pan",
            Utility => "Utility",
            Bassify => "Bassify",
            Formant => "Formant Filter",
            Wobble => "Wobble",
            Tape => "Tape",
            Transient => "Transient",
            Multiband => "Multiband",
        }
    }

    pub fn short(self) -> &'static str {
        use DeviceKind::*;
        match self {
            DrumRack => "DRUM",
            SubBass => "SUB",
            ReeseBass => "REESE",
            GrowlBass => "GROWL",
            Supersaw => "SUPER",
            Pluck => "PLUCK",
            Keys => "KEYS",
            Organ => "ORGAN",
            Pad => "PAD",
            Riser => "RISER",
            Salamander => "GRAND",
            VoxBass => "VOX",
            VocalTake => "TAKE",
            Arpeggiator => "ARP",
            ChordGenerator => "CHORD",
            Filter => "FILT",
            Drive => "DRIVE",
            Bitcrush => "CRUSH",
            Reverb => "REVERB",
            Delay => "DELAY",
            Chorus => "CHORUS",
            Compressor => "COMP",
            Eq3 => "EQ3",
            Phaser => "PHASER",
            Sidechain => "PUMP",
            AutoPan => "PAN",
            Utility => "UTIL",
            Bassify => "BASSIFY",
            Formant => "FORMANT",
            Wobble => "WOBBLE",
            Tape => "TAPE",
            Transient => "TRANS",
            Multiband => "MBAND",
        }
    }

    pub fn color(self) -> [u8; 3] {
        use DeviceKind::*;
        match self {
            DrumRack => [255, 122, 74],
            SubBass => [108, 146, 255],
            ReeseBass => [136, 96, 255],
            GrowlBass => [225, 66, 138],
            Supersaw => [255, 190, 60],
            Pluck => [90, 220, 170],
            Keys => [120, 200, 255],
            Organ => [244, 184, 108],
            Pad => [150, 130, 255],
            Riser => [255, 100, 200],
            Salamander => [245, 206, 130],
            VoxBass => [255, 126, 176],
            VocalTake => [255, 166, 120],
            Arpeggiator => [120, 235, 210],
            ChordGenerator => [196, 160, 255],
            Filter => [90, 200, 230],
            Drive => [255, 150, 70],
            Bitcrush => [210, 110, 255],
            Reverb => [110, 170, 255],
            Delay => [90, 210, 200],
            Chorus => [130, 190, 255],
            Compressor => [200, 200, 120],
            Eq3 => [140, 220, 160],
            Phaser => [180, 140, 255],
            Sidechain => [255, 120, 120],
            AutoPan => [120, 220, 190],
            Utility => [170, 170, 180],
            Bassify => [255, 120, 176],
            Formant => [118, 235, 205],
            Wobble => [255, 170, 82],
            Tape => [206, 166, 112],
            Transient => [130, 232, 180],
            Multiband => [126, 186, 255],
        }
    }

    pub fn defs(self) -> &'static [ParamDef] {
        use DeviceKind::*;
        match self {
            DrumRack => &DEF_DRUM_RACK,
            SubBass => &DEF_SUB_BASS,
            ReeseBass => &DEF_REESE_BASS,
            GrowlBass => &DEF_GROWL_BASS,
            Supersaw => &DEF_SUPERSAW,
            Pluck => &DEF_PLUCK,
            Keys => &DEF_KEYS,
            Organ => &DEF_ORGAN,
            Pad => &DEF_PAD,
            Riser => &DEF_RISER,
            Salamander => &DEF_SALAMANDER,
            VoxBass => &DEF_VOX_BASS,
            VocalTake => &DEF_VOCAL_TAKE,
            Arpeggiator => &DEF_ARPEGGIATOR,
            ChordGenerator => &DEF_CHORD_GENERATOR,
            Filter => &DEF_FILTER,
            Drive => &DEF_DRIVE,
            Bitcrush => &DEF_BITCRUSH,
            Reverb => &DEF_REVERB,
            Delay => &DEF_DELAY,
            Chorus => &DEF_CHORUS,
            Compressor => &DEF_COMPRESSOR,
            Eq3 => &DEF_EQ3,
            Phaser => &DEF_PHASER,
            Sidechain => &DEF_SIDECHAIN,
            AutoPan => &DEF_AUTO_PAN,
            Utility => &DEF_UTILITY,
            Bassify => &DEF_BASSIFY,
            Formant => &DEF_FORMANT,
            Wobble => &DEF_WOBBLE,
            Tape => &DEF_TAPE,
            Transient => &DEF_TRANSIENT,
            Multiband => &DEF_MULTIBAND,
        }
    }

    pub fn presets(self) -> &'static [Preset] {
        use DeviceKind::*;
        match self {
            DrumRack => &[
                Preset {
                    name: "Acoustic Studio",
                    values: &[0.88, 0.0, 0.8, 0.45, 0.55, 0.12, 0.6, 0.55, 0.0],
                },
                Preset {
                    name: "House Club",
                    values: &[0.86, -1.0, 0.45, 0.6, 0.6, 0.18, 0.65, 0.6, 1.0],
                },
                Preset {
                    name: "Trap 808",
                    values: &[0.9, -2.0, 0.6, 0.7, 0.7, 0.25, 0.55, 0.6, 2.0],
                },
                Preset {
                    name: "Dubstep",
                    values: &[0.9, -1.0, 0.9, 0.65, 0.65, 0.35, 0.7, 0.55, 3.0],
                },
                Preset {
                    name: "Boom Bap",
                    values: &[0.86, -2.0, 0.35, 0.5, 0.5, 0.3, 0.5, 0.45, 4.0],
                },
                Preset {
                    name: "Bounce",
                    values: &[0.88, -1.0, 0.5, 0.55, 0.6, 0.2, 0.6, 0.55, 5.0],
                },
                Preset {
                    name: "Salamander Studio",
                    values: &[0.88, 0.0, 0.8, 0.45, 0.55, 0.12, 0.6, 0.55, 6.0],
                },
                Preset {
                    name: "Salamander Hard",
                    values: &[0.9, 0.0, 0.55, 0.65, 0.65, 0.2, 0.65, 0.65, 7.0],
                },
                Preset {
                    name: "Salamander Soft",
                    values: &[0.85, 0.0, 0.4, 0.35, 0.45, 0.08, 0.55, 0.4, 8.0],
                },
                Preset {
                    name: "World Percussion",
                    values: &[0.86, -1.0, 0.5, 0.5, 0.55, 0.15, 0.7, 0.6, 9.0],
                },
                Preset {
                    name: "Orchestral Percussion",
                    values: &[0.86, -2.0, 0.7, 0.45, 0.55, 0.1, 0.6, 0.55, 10.0],
                },
                Preset {
                    name: "Mallet Lab",
                    values: &[0.9, 0.0, 0.9, 0.35, 0.5, 0.15, 0.7, 0.7, 11.0],
                },
                Preset {
                    name: "TR-808 Studio",
                    values: &[0.9, -2.0, 0.5, 0.6, 0.6, 0.25, 0.6, 0.55, 12.0],
                },
                Preset {
                    name: "Tape Crunch",
                    values: &[0.86, 0.0, 0.8, 0.5, 0.5, 0.35, 0.55, 0.45, 13.0],
                },
                Preset {
                    name: "Vinyl Dust",
                    values: &[0.84, -1.0, 0.7, 0.45, 0.5, 0.25, 0.5, 0.35, 14.0],
                },
                Preset {
                    name: "Sub Pressure",
                    values: &[0.9, -4.0, 1.0, 0.7, 0.6, 0.3, 0.45, 0.4, 15.0],
                },
                Preset {
                    name: "Glass Room",
                    values: &[0.88, 2.0, 1.1, 0.35, 0.5, 0.1, 0.7, 0.75, 16.0],
                },
                Preset {
                    name: "Warehouse Kit",
                    values: &[0.88, -1.0, 1.0, 0.6, 0.6, 0.35, 0.65, 0.5, 17.0],
                },
                Preset {
                    name: "Dub Chamber",
                    values: &[0.85, -3.0, 1.2, 0.5, 0.55, 0.2, 0.75, 0.3, 18.0],
                },
                Preset {
                    name: "Pocket Drums",
                    values: &[0.88, 0.0, 0.4, 0.55, 0.6, 0.12, 0.5, 0.55, 19.0],
                },
                Preset {
                    name: "Neon Electro",
                    values: &[0.9, 1.0, 0.6, 0.65, 0.65, 0.3, 0.6, 0.65, 20.0],
                },
                Preset {
                    name: "Broken Beats",
                    values: &[0.86, -1.0, 0.55, 0.6, 0.55, 0.4, 0.55, 0.35, 21.0],
                },
                Preset {
                    name: "Deep 909",
                    values: &[0.9, -4.0, 0.9, 0.6, 0.6, 0.25, 0.55, 0.4, 22.0],
                },
                Preset {
                    name: "Hard Techno",
                    values: &[0.9, -2.0, 0.5, 0.7, 0.7, 0.45, 0.6, 0.55, 23.0],
                },
                Preset {
                    name: "Cinematic Hits",
                    values: &[0.88, -4.0, 1.3, 0.45, 0.55, 0.15, 0.7, 0.5, 24.0],
                },
            ],
            SubBass => &[
                Preset {
                    name: "Clean Sub",
                    values: &[0.9, 0.0, 0.005, 0.4, 0.85, 0.12, 0.1, 0.0],
                },
                Preset {
                    name: "808 Slide",
                    values: &[0.95, -5.0, 0.004, 0.9, 0.7, 0.35, 0.25, 0.06],
                },
                Preset {
                    name: "Growl Sub",
                    values: &[0.9, -7.0, 0.01, 0.6, 0.8, 0.2, 0.5, 0.0],
                },
            ],
            ReeseBass => &[
                Preset {
                    name: "Dark Reese",
                    values: &[0.85, 500.0, 0.4, 0.02, 0.8, 0.3, 0.3, 0.01],
                },
                Preset {
                    name: "Screamer",
                    values: &[0.8, 1800.0, 0.6, 0.04, 3.5, 0.6, 0.55, 0.005],
                },
                Preset {
                    name: "Warehouse",
                    values: &[0.8, 900.0, 0.25, 0.012, 0.35, 0.2, 0.2, 0.02],
                },
            ],
            GrowlBass => &[
                Preset {
                    name: "Yoi Growl",
                    values: &[0.85, 900.0, 0.5, 4.0, 0.85, 0.7, 0.4, 0.004],
                },
                Preset {
                    name: "Riddim Wobble",
                    values: &[0.9, 600.0, 0.55, 2.0, 0.95, 0.9, 0.5, 0.003],
                },
                Preset {
                    name: "Death Growl",
                    values: &[0.8, 2200.0, 0.7, 6.5, 0.7, 0.5, 0.65, 0.006],
                },
            ],
            Supersaw => &[
                Preset {
                    name: "Festival Lead",
                    values: &[0.75, 0.03, 9000.0, 0.12, 0.005, 0.3, 0.0, 0.8],
                },
                Preset {
                    name: "Wide Chords",
                    values: &[0.7, 0.045, 5000.0, 0.2, 0.02, 0.6, 0.0, 0.95],
                },
                Preset {
                    name: "Trance Pluck",
                    values: &[0.7, 0.02, 7000.0, 0.25, 0.002, 0.18, 1.0, 0.7],
                },
            ],
            Pluck => &[
                Preset {
                    name: "House Stab",
                    values: &[0.8, 3200.0, 0.3, 0.22, 0.016, 0.7, 0.002, 0.0],
                },
                Preset {
                    name: "Deep Chord",
                    values: &[0.8, 1600.0, 0.15, 0.45, 0.008, 0.4, 0.004, -1.0],
                },
                Preset {
                    name: "Acid Pluck",
                    values: &[0.75, 5500.0, 0.65, 0.16, 0.02, 0.9, 0.001, 0.0],
                },
            ],
            Keys => &[
                Preset {
                    name: "FM Piano",
                    values: &[0.8, 2.0, 0.3, 0.004, 1.4, 0.15, 0.35, 0.6],
                },
                Preset {
                    name: "Bell Keys",
                    values: &[0.7, 7.0, 0.55, 0.002, 2.2, 0.0, 1.2, 0.8],
                },
                Preset {
                    name: "Warm Rhodes",
                    values: &[0.8, 1.0, 0.18, 0.01, 1.0, 0.3, 0.5, 0.4],
                },
            ],
            Organ => &[
                Preset {
                    name: "Gospel Drawbars",
                    values: &[0.8, 0.55, 0.35, 0.2, 0.08, 0.003, 0.25, 0.0],
                },
                Preset {
                    name: "House Organ",
                    values: &[0.72, 0.72, 0.25, 0.6, 0.12, 0.002, 0.12, 0.0],
                },
                Preset {
                    name: "Dub Organ",
                    values: &[0.8, 0.35, 0.45, 0.75, 0.4, 0.006, 0.4, -1.0],
                },
            ],
            Pad => &[
                Preset {
                    name: "Warm Pad",
                    values: &[0.65, 1.4, 2.5, 1400.0, 0.15, 0.03, 0.0, 0.35],
                },
                Preset {
                    name: "Dark Atmosphere",
                    values: &[0.65, 2.5, 3.5, 700.0, 0.3, 0.05, -1.0, 0.6],
                },
                Preset {
                    name: "Supersaw Pad",
                    values: &[0.6, 0.8, 2.0, 3500.0, 0.1, 0.04, 0.0, 0.2],
                },
            ],
            Riser => &[
                Preset {
                    name: "White Riser",
                    values: &[0.6, 8.0, 0.85, 800.0, 0.35, 0.9, 1.5, 0.5],
                },
                Preset {
                    name: "Tonal Riser",
                    values: &[0.65, 4.0, 0.7, 1200.0, 0.5, 0.25, 0.8, 0.4],
                },
                Preset {
                    name: "Short Sweep",
                    values: &[0.65, 2.0, 0.6, 1500.0, 0.3, 0.5, 0.3, 0.2],
                },
            ],
            Salamander => &[
                Preset {
                    name: "Salamander Grand",
                    values: &[0.95, 0.62, 0.55, 0.0, 0.0, 0.8, 0.5, 0.002],
                },
                Preset {
                    name: "Felt Piano",
                    values: &[0.9, 0.35, 0.7, 0.0, 0.0, 0.6, 0.3, 0.02],
                },
                Preset {
                    name: "Salamander Bass",
                    values: &[1.0, 0.28, 0.25, 0.05, 0.0, 0.4, 0.2, 0.001],
                },
                Preset {
                    name: "Wide Grand",
                    values: &[1.0, 0.7, 0.8, 0.02, 0.0, 0.9, 0.9, 0.001],
                },
            ],
            VoxBass => &[
                Preset {
                    name: "Bassify Vox",
                    values: &[0.9, 2.0, 0.6, 0.55, 4.0, 0.65, 0.4, 0.16],
                },
                Preset {
                    name: "Deep Vowel",
                    values: &[0.9, 3.0, 0.8, 0.35, 2.0, 0.3, 0.25, 0.3],
                },
                Preset {
                    name: "Riddim Mouth",
                    values: &[0.85, 0.0, 0.5, 0.8, 8.0, 0.9, 0.6, 0.1],
                },
                Preset {
                    name: "Liquid Vox",
                    values: &[0.85, 1.0, 0.4, 0.35, 0.5, 0.2, 0.2, 0.6],
                },
            ],
            Arpeggiator => &[
                Preset {
                    name: "Trance Gate",
                    values: &[2.0, 0.0, 2.0, 0.55, 0.0],
                },
                Preset {
                    name: "Pluck Up",
                    values: &[1.0, 0.0, 2.0, 0.35, 0.0],
                },
                Preset {
                    name: "Riddim Chatter",
                    values: &[3.0, 3.0, 1.0, 0.45, 0.18],
                },
                Preset {
                    name: "Wide Cascade",
                    values: &[2.0, 2.0, 3.0, 0.7, 0.0],
                },
            ],
            ChordGenerator => &[
                Preset {
                    name: "Diatonic Triads",
                    values: &[0.0, 0.0, 0.0, 0.0, 0.0],
                },
                Preset {
                    name: "Seventh Stack",
                    values: &[0.0, 0.0, 3.0, 0.0, 0.0],
                },
                Preset {
                    name: "Sus Lift",
                    values: &[0.0, 0.0, 4.0, 1.0, 0.0],
                },
                Preset {
                    name: "Minor Mood",
                    values: &[9.0, 1.0, 2.0, 0.0, 0.0],
                },
                Preset {
                    name: "Major Anthem",
                    values: &[0.0, 0.0, 1.0, 0.0, 1.0],
                },
            ],
            VocalTake => &[
                Preset {
                    name: "Vocal Bass",
                    values: &[0.95, -12.0, 0.0, 0.72, 0.18, 0.0],
                },
                Preset {
                    name: "Rap Bass",
                    values: &[0.92, -7.0, 0.0, 0.6, 0.12, 0.0],
                },
                Preset {
                    name: "Octave Down",
                    values: &[1.0, -24.0, 0.0, 0.75, 0.22, 0.0],
                },
                Preset {
                    name: "Reverse Chop",
                    values: &[0.9, -7.0, 0.0, 0.82, 0.08, 1.0],
                },
            ],
            Filter => &[
                Preset {
                    name: "Init",
                    values: &[1200.0, 0.2, 0.0, 0.1, 1.0],
                },
                Preset {
                    name: "Dubstep Sweep",
                    values: &[450.0, 0.6, 0.0, 0.35, 1.0],
                },
                Preset {
                    name: "Telephone",
                    values: &[1800.0, 0.4, 1.0, 0.2, 1.0],
                },
            ],
            Drive => &[
                Preset {
                    name: "Warm",
                    values: &[0.25, 0.0, 0.55, 0.6, 0.85],
                },
                Preset {
                    name: "Rage",
                    values: &[0.75, 1.0, 0.7, 1.0, 0.7],
                },
                Preset {
                    name: "Fold",
                    values: &[0.5, 2.0, 0.5, 0.8, 0.75],
                },
            ],
            Bitcrush => &[
                Preset {
                    name: "Lo-Fi",
                    values: &[8.0, 6.0, 0.6, 0.8],
                },
                Preset {
                    name: "Destroy",
                    values: &[3.0, 24.0, 1.0, 0.7],
                },
            ],
            Reverb => &[
                Preset {
                    name: "Club Hall",
                    values: &[0.7, 0.4, 0.9, 0.35],
                },
                Preset {
                    name: "Tight Room",
                    values: &[0.3, 0.6, 0.6, 0.22],
                },
                Preset {
                    name: "Infinite",
                    values: &[1.0, 0.2, 1.0, 0.6],
                },
            ],
            Delay => &[
                Preset {
                    name: "Ping Pong",
                    values: &[0.5, 0.4, 0.5, 0.3, 1.0],
                },
                Preset {
                    name: "Dub Echo",
                    values: &[0.75, 0.65, 0.3, 0.4, 1.0],
                },
                Preset {
                    name: "Slap",
                    values: &[0.125, 0.15, 0.6, 0.2, 0.0],
                },
            ],
            Chorus => &[
                Preset {
                    name: "Width",
                    values: &[0.4, 0.4, 0.4, 0.1],
                },
                Preset {
                    name: "Juno",
                    values: &[0.7, 0.55, 0.5, 0.25],
                },
            ],
            Compressor => &[
                Preset {
                    name: "Glue",
                    values: &[-18.0, 3.0, 0.02, 0.2, 3.0, 1.0],
                },
                Preset {
                    name: "Punch",
                    values: &[-24.0, 6.0, 0.005, 0.09, 6.0, 1.0],
                },
            ],
            Eq3 => &[
                Preset {
                    name: "Smile",
                    values: &[3.0, -2.0, 3.5, 1000.0],
                },
                Preset {
                    name: "Telephone",
                    values: &[-12.0, 6.0, -9.0, 1500.0],
                },
            ],
            Phaser => &[
                Preset {
                    name: "Jet",
                    values: &[0.35, 0.8, 0.55, 0.6, 800.0],
                },
                Preset {
                    name: "Slow Sweep",
                    values: &[0.12, 0.6, 0.35, 0.4, 1200.0],
                },
            ],
            Sidechain => &[
                Preset {
                    name: "House Pump",
                    values: &[0.0, 0.75, 0.005, 0.16, 1.0],
                },
                Preset {
                    name: "Hard Duck",
                    values: &[0.0, 1.0, 0.002, 0.09, 1.0],
                },
                Preset {
                    name: "Offbeat",
                    values: &[1.0, 0.7, 0.01, 0.12, 1.0],
                },
            ],
            AutoPan => &[
                Preset {
                    name: "Slow Drift",
                    values: &[0.15, 0.6, 0.0, 0.2],
                },
                Preset {
                    name: "Trance Gate",
                    values: &[2.0, 0.9, 0.0, 0.9],
                },
            ],
            Utility => &[
                Preset {
                    name: "Reset",
                    values: &[0.0, 1.0, 0.0],
                },
                Preset {
                    name: "Wide",
                    values: &[0.0, 1.6, 0.0],
                },
            ],
            Bassify => &[
                Preset {
                    name: "Vocal Wub",
                    values: &[0.75, 2.0, 0.45, 0.55, 0.55, 1.0, 0.85, 4.0],
                },
                Preset {
                    name: "Crunchy Bass",
                    values: &[0.95, 0.0, 0.3, 0.8, 0.7, 1.2, 1.0, 8.0],
                },
                Preset {
                    name: "Liquid Formant",
                    values: &[0.55, 4.0, 0.25, 0.25, 0.25, 0.8, 0.75, 0.6],
                },
            ],
            Formant => &[
                Preset {
                    name: "Talking Bass",
                    values: &[2.0, 0.5, 1.1, 0.7, 0.9, 0.9],
                },
                Preset {
                    name: "Vowel Sweep",
                    values: &[0.0, 0.85, 0.3, 0.6, 0.8, 0.9],
                },
                Preset {
                    name: "Robot Choir",
                    values: &[1.0, 0.45, 4.5, 0.4, 0.65, 0.9],
                },
            ],
            Wobble => &[
                Preset {
                    name: "Classic Wub",
                    values: &[4.0, 0.7, 0.2, 0.25, 0.8, 0.9],
                },
                Preset {
                    name: "Riddim Chop",
                    values: &[8.0, 0.9, 0.8, 0.4, 0.9, 0.9],
                },
                Preset {
                    name: "Slow Sweep",
                    values: &[0.75, 0.5, 0.1, 0.1, 0.6, 0.9],
                },
            ],
            Tape => &[
                Preset {
                    name: "Warm Tape",
                    values: &[0.35, 0.55, 0.2, 0.7, 0.9],
                },
                Preset {
                    name: "Dusty Loop",
                    values: &[0.55, 0.8, 0.55, 0.8, 0.9],
                },
                Preset {
                    name: "Master Glue",
                    values: &[0.2, 0.4, 0.05, 0.5, 1.0],
                },
            ],
            Transient => &[
                Preset {
                    name: "Punch",
                    values: &[0.5, 0.1, 0.8, 0.9],
                },
                Preset {
                    name: "Soft Touch",
                    values: &[-0.2, 0.3, 0.6, 0.9],
                },
            ],
            Multiband => &[
                Preset {
                    name: "Glue",
                    values: &[0.2, 0.35, 0.15, 1200.0, 0.8, 0.9],
                },
                Preset {
                    name: "Crush",
                    values: &[0.5, 0.8, 0.5, 900.0, 1.0, 0.8],
                },
            ],
        }
    }

    pub fn default_preset(self) -> Preset {
        self.presets()[0]
    }
}

/// Track colours used by the session grid, mixer strips and arrangement.
pub const TRACK_COLORS: [[u8; 3]; 10] = [
    [255, 122, 74],
    [108, 146, 255],
    [225, 66, 138],
    [255, 190, 60],
    [90, 220, 170],
    [136, 96, 255],
    [255, 100, 200],
    [120, 200, 255],
    [200, 200, 120],
    [150, 220, 120],
];

pub fn device_index(kind: DeviceKind) -> usize {
    DeviceKind::ALL.iter().position(|k| *k == kind).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drum_rack_presets_cover_every_kit() {
        let presets = DeviceKind::DrumRack.presets();
        assert_eq!(presets.len(), DRUM_KIT_NAMES.len());
        assert_eq!(DRUM_KIT_NAMES.len(), 25);
        assert_eq!(DEF_DRUM_RACK[8].max, 24.0);
        assert_eq!(DRUM_KIT_NAMES, &crate::samples::KIT_NAMES[..]);
        for (i, preset) in presets.iter().enumerate() {
            assert_eq!(preset.name, DRUM_KIT_NAMES[i]);
            assert_eq!(preset.values.len(), 9);
            assert_eq!(preset.values[8] as usize, i);
        }
        assert_eq!(DEF_DRUM_RACK[8].format(12.0), "TR-808 Studio");
        assert_eq!(DEF_DRUM_RACK[8].format(24.0), "Cinematic Hits");
    }

    #[test]
    fn midi_devices_are_catalogue_members() {
        assert_eq!(DeviceKind::ALL.len(), 33);
        for kind in [DeviceKind::Arpeggiator, DeviceKind::ChordGenerator] {
            assert!(kind.is_midi());
            assert!(!kind.is_instrument());
            assert!(!kind.is_audio_effect());
            assert_eq!(kind.category(), "MIDI FX");
            assert!(DeviceKind::ALL.contains(&kind));
            assert!(!kind.presets().is_empty());
            assert_eq!(kind.defs().len(), 5);
        }
        let arp = Device::new(DeviceKind::Arpeggiator);
        assert_eq!(arp.defs()[0].format(arp.p(0)), "1/16");
        let chord = Device::new(DeviceKind::ChordGenerator);
        assert_eq!(chord.defs()[2].format(chord.p(2)), "Diatonic");
    }

    #[test]
    fn hz_format_keeps_slow_lfo_precision() {
        let def = ParamDef::new("Rate", 0.0, 8.0, 0.35, 0.0, Fmt::Hz);
        assert_eq!(def.format(0.35), "0.35 Hz");
        assert_eq!(def.format(4.0), "4 Hz");
        assert_eq!(def.format(1200.0), "1.20 kHz");
    }
}
