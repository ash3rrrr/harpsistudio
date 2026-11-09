//! Factory content: demo songs and musical default patterns, so Harpsistudio
//! is playable the moment it opens.

use crate::audio_take::AudioTake;
use crate::params::{Device, DeviceKind};
use crate::song::{Clip, Note, Song, Track};

pub struct DemoInfo {
    pub name: &'static str,
    pub tag: &'static str,
    pub bpm: f64,
    pub blurb: &'static str,
}

pub const DEMOS: [DemoInfo; 11] = [
    DemoInfo {
        name: "Midnight House",
        tag: "HOUSE",
        bpm: 124.0,
        blurb: "Am7 house stabs, pumping sub and a festival lead.",
    },
    DemoInfo {
        name: "Riddim Protocol",
        tag: "DUBSTEP",
        bpm: 140.0,
        blurb: "Half-time drums, wobbling growl bass and a riser.",
    },
    DemoInfo {
        name: "Big Room Anthem",
        tag: "EDM",
        bpm: 128.0,
        blurb: "Wide supersaw chords, a drums-out build and a bar-9 drop.",
    },
    DemoInfo {
        name: "Deep Groove",
        tag: "DEEP HOUSE",
        bpm: 122.0,
        blurb: "FM keys, warm pad and a lazy shuffled groove.",
    },
    DemoInfo {
        name: "Warehouse Techno",
        tag: "TECHNO",
        bpm: 132.0,
        blurb: "Driving Reese bass, metallic hats and dark stabs.",
    },
    DemoInfo {
        name: "Neon Cathedral",
        tag: "MELODIC BASS",
        bpm: 150.0,
        blurb: "Half-time drums, talking Vox Bass and a real-piano topline.",
    },
    DemoInfo {
        name: "Velvet Sky",
        tag: "MELODIC",
        bpm: 108.0,
        blurb: "Salamander piano, warm sub and an organic head-nod groove.",
    },
    DemoInfo {
        name: "Cypher Room",
        tag: "RAP",
        bpm: 90.0,
        blurb: "Boom-bap drums, felt piano, sliding 808 and a vocal-forward rap flow.",
    },
    DemoInfo {
        name: "Organ House",
        tag: "HOUSE",
        bpm: 124.0,
        blurb: "Tonewheel organ chords, a vowel-swept formant layer and a gospel lead over a club groove.",
    },
    DemoInfo {
        name: "Live Room",
        tag: "HOUSE",
        bpm: 118.0,
        blurb: "Salamander studio drums, hand percussion and felt piano over a warm club sub.",
    },
    DemoInfo {
        name: "Vocal Riddim",
        tag: "DUBSTEP",
        bpm: 140.0,
        blurb: "Chopped vocal hook, talking vocal bass and a half-time double drop.",
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VocalStyle {
    Dubstep,
    Rap,
}

impl VocalStyle {
    pub fn label(self) -> &'static str {
        match self {
            VocalStyle::Dubstep => "Dubstep",
            VocalStyle::Rap => "Rap",
        }
    }
}

/// The full vocal-to-bass path used by both the Starters panel and the tests:
/// the imported take is the instrument source, then Bassify and Wobble shape it
/// into a talking bass before compression and EQ.
pub fn add_vocal_bass_track(song: &mut Song, take: AudioTake, style: VocalStyle) -> usize {
    let idx = song.add_track(DeviceKind::VocalTake);
    let bpm = song.bpm.max(30.0);
    let beats = (take.duration_seconds() as f64 * bpm / 60.0).max(0.25);
    let length = (beats * 4.0).ceil() / 4.0;
    let o = match style {
        VocalStyle::Dubstep => "Dubstep",
        VocalStyle::Rap => "Rap",
    };

    let t = &mut song.tracks[idx];
    t.name = format!("{o} Vocal Bass");
    t.volume = 0.82;
    t.send = 0.08;
    t.instrument.set(0, 0.95);
    t.instrument.set(
        1,
        if style == VocalStyle::Rap {
            -7.0
        } else {
            -12.0
        },
    );
    t.instrument.set(3, 0.72);
    t.instrument.set(4, 0.18);

    let bassify = dev(
        DeviceKind::Bassify,
        if style == VocalStyle::Rap { 1 } else { 0 },
    );
    t.effects.push(bassify);
    let wobble = dev(
        DeviceKind::Wobble,
        if style == VocalStyle::Rap { 2 } else { 0 },
    );
    t.effects.push(wobble);
    t.effects.push(dev(DeviceKind::Compressor, 0));
    t.effects.push(dev(DeviceKind::Eq3, 0));

    let bass = &mut t.effects[0];
    bass.set(0, 0.82);
    bass.set(2, 0.70);
    bass.set(3, 0.5);
    bass.set(4, 0.48);
    bass.set(6, 0.92);
    bass.set(7, if style == VocalStyle::Rap { 1.2 } else { 3.2 });
    let wob = &mut t.effects[1];
    wob.set(0, if style == VocalStyle::Rap { 1.0 } else { 3.2 });
    wob.set(4, if style == VocalStyle::Rap { 0.25 } else { 0.45 });
    let comp = &mut t.effects[2];
    comp.set(0, -22.0);
    comp.set(1, 3.5);
    comp.set(4, 4.0);
    let eq = &mut t.effects[3];
    eq.set(0, 4.0);
    eq.set(1, -2.0);
    eq.set(2, -4.0);

    let mut c = Clip::new(&format!("{o} Vox Bass"), 0, 0.0, length);
    c.looping = false;
    c.notes.push(Note::new(0.0, beats.min(length), 60, 1.0));
    // The session clip is for instant jamming; the arrangement clip is what
    // Mixdown renders, so importing a vocal into an existing starter is
    // audible in the exported WAV too.
    let mut arrangement = c.clone();
    arrangement.scene = -1;
    t.take = Some(take);
    t.clips.push(c);
    t.clips.push(arrangement);
    idx
}

pub fn demo_song(index: usize) -> Song {
    match index {
        1 => riddim(),
        2 => big_room(),
        3 => deep_house(),
        4 => techno(),
        5 => neon_cathedral(),
        6 => velvet_sky(),
        7 => cypher_room(),
        8 => organ_house(),
        9 => live_room(),
        10 => vocal_riddim(),
        _ => house(),
    }
}

// ---------------------------------------------------------------------------
// Note helpers
// ---------------------------------------------------------------------------

fn nt(start: f64, len: f64, pitch: u8, vel: f32) -> Note {
    Note::new(start, len, pitch, vel)
}

fn chord(out: &mut Vec<Note>, root: u8, ivs: &[i32], start: f64, len: f64, vel: f32) {
    for iv in ivs {
        out.push(nt(start, len, (root as i32 + iv).clamp(0, 127) as u8, vel));
    }
}

const CH_AM7: &[i32] = &[0, 3, 7, 10];
const CH_FMAJ7: &[i32] = &[0, 4, 7, 11];
const CH_CMAJ7: &[i32] = &[0, 4, 7, 12];
const CH_G6: &[i32] = &[0, 4, 7, 9];
const CH_MIN: &[i32] = &[0, 3, 7];

fn progression(bar: usize) -> (u8, &'static [i32]) {
    match bar % 4 {
        0 => (57, CH_AM7),
        1 => (53, CH_FMAJ7),
        2 => (48, CH_CMAJ7),
        _ => (55, CH_G6),
    }
}

fn minor_progression(bar: usize) -> (u8, &'static [i32]) {
    match bar % 4 {
        0 => (45, CH_MIN), // A2
        1 => (41, CH_MIN), // F2
        2 => (48, CH_MIN), // C3
        _ => (43, CH_MIN), // G2
    }
}

// ---------------------------------------------------------------------------
// Drum patterns
// ---------------------------------------------------------------------------

fn house_drums(bars: usize, fill: bool) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        for b in 0..4 {
            v.push(nt(o + b as f64, 0.25, 36, if b == 0 { 1.0 } else { 0.92 }));
        }
        v.push(nt(o + 1.0, 0.25, 39, 0.85));
        v.push(nt(o + 3.0, 0.25, 39, 0.85));
        for step in 0..16 {
            let pos = step as f64 * 0.25;
            if (step % 4) == 2 {
                v.push(nt(o + pos, 0.2, 46, 0.45));
            } else {
                let vel = if step % 2 == 0 { 0.32 } else { 0.18 };
                v.push(nt(o + pos, 0.1, 42, vel));
            }
        }
        if bar == 0 {
            v.push(nt(o, 1.0, 49, 0.5));
        }
        if fill && bar == bars - 1 {
            for i in 0..4 {
                v.push(nt(o + 2.5 + i as f64 * 0.25, 0.2, 38, 0.5 + i as f32 * 0.1));
            }
        }
    }
    v
}

fn half_time_drums(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        v.push(nt(o, 0.3, 36, 1.0));
        v.push(nt(o + 2.75, 0.25, 36, 0.8));
        v.push(nt(o + 2.0, 0.3, 38, 1.0));
        v.push(nt(o + 3.75, 0.2, 38, 0.4));
        for step in 0..16 {
            let pos = step as f64 * 0.25;
            if step % 4 == 2 {
                v.push(nt(o + pos, 0.15, 42, 0.35));
            } else if step % 2 == 0 {
                v.push(nt(o + pos, 0.1, 42, 0.25));
            }
        }
        v.push(nt(o + 1.5, 0.2, 46, 0.4));
        if bar % 4 == 0 {
            v.push(nt(o, 1.0, 49, 0.45));
        }
        if bar % 8 == 7 {
            for i in 0..6 {
                v.push(nt(
                    o + 2.0 + i as f64 * 0.125,
                    0.1,
                    38,
                    0.4 + i as f32 * 0.08,
                ));
            }
        }
    }
    v
}

fn big_room_drums(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        for b in 0..4 {
            v.push(nt(o + b as f64, 0.25, 36, 1.0));
        }
        v.push(nt(o + 1.0, 0.25, 39, 0.9));
        v.push(nt(o + 3.0, 0.25, 39, 0.9));
        for b in 0..4 {
            v.push(nt(o + b as f64 + 0.5, 0.15, 42, 0.3));
        }
        v.push(nt(o + 1.5, 0.2, 46, 0.35));
        v.push(nt(o + 3.5, 0.2, 46, 0.35));
        if bar % 4 == 0 {
            v.push(nt(o, 1.0, 49, 0.45));
        }
    }
    v
}

fn deep_drums(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        v.push(nt(o, 0.25, 36, 0.95));
        v.push(nt(o + 1.5, 0.25, 36, 0.75));
        v.push(nt(o + 2.0, 0.25, 36, 0.9));
        v.push(nt(o + 3.0, 0.25, 36, 0.7));
        v.push(nt(o + 1.0, 0.15, 37, 0.55));
        v.push(nt(o + 3.0, 0.15, 37, 0.45));
        for b in 0..8 {
            let pos = b as f64 * 0.5;
            v.push(nt(
                o + pos + 0.25,
                0.12,
                42,
                if b % 2 == 0 { 0.3 } else { 0.18 },
            ));
        }
        if bar % 4 == 0 {
            v.push(nt(o, 1.5, 49, 0.25));
        }
    }
    v
}

fn techno_drums(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        for b in 0..4 {
            v.push(nt(o + b as f64, 0.25, 36, 1.0));
        }
        v.push(nt(o + 1.0, 0.2, 39, 0.7));
        v.push(nt(o + 3.0, 0.2, 39, 0.7));
        for step in 0..16 {
            let pos = step as f64 * 0.25;
            let vel = if step % 4 == 2 {
                0.45
            } else if step % 2 == 0 {
                0.22
            } else {
                0.12
            };
            let pitch = if step % 4 == 2 { 46 } else { 42 };
            v.push(nt(o + pos, 0.08, pitch, vel));
        }
        if bar % 2 == 1 {
            v.push(nt(o + 3.5, 0.2, 45, 0.35));
        }
    }
    v
}

fn world_percussion(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        v.push(nt(o, 0.3, 36, 0.95));
        v.push(nt(o + 1.5, 0.25, 36, 0.7));
        v.push(nt(o + 2.5, 0.25, 36, 0.82));
        v.push(nt(o + 1.0, 0.2, 38, 0.62));
        v.push(nt(o + 3.25, 0.2, 38, 0.5));
        for step in 0..16 {
            let vel = if step % 4 == 0 {
                0.3
            } else if step % 2 == 0 {
                0.2
            } else {
                0.12
            };
            v.push(nt(o + step as f64 * 0.25, 0.08, 42, vel));
        }
        v.push(nt(o + 1.5, 0.2, 46, 0.32));
        v.push(nt(o + 3.5, 0.2, 46, 0.28));
        v.push(nt(o + 1.75, 0.2, 43, 0.7));
        v.push(nt(o + 3.0, 0.2, 43, 0.6));
        for step in (0..16).step_by(2) {
            v.push(nt(o + step as f64 * 0.25, 0.08, 70, 0.16));
        }
        v.push(nt(o + 3.75, 0.15, 56, 0.42));
    }
    v
}

// ---------------------------------------------------------------------------
// Bass / chord / lead patterns
// ---------------------------------------------------------------------------

fn house_bass(bars: usize, root: u8) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        for b in 0..4 {
            v.push(nt(o + b as f64 + 0.5, 0.4, root, 0.8));
        }
        if bar % 2 == 1 {
            v.push(nt(o + 3.75, 0.2, root + 12, 0.6));
        }
    }
    v
}

fn long_sub(bars: usize, root: u8) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        v.push(nt(o, 2.0, root, 0.9));
        v.push(nt(o + 2.0, 1.75, root, 0.8));
        if bar % 4 == 3 {
            v.push(nt(o + 3.75, 0.2, root + 12, 0.6));
        }
    }
    v
}

fn growl_pattern(bars: usize, root: u8) -> Vec<Note> {
    let mut v = Vec::new();
    let pitches = [
        root,
        root,
        root + 3,
        root,
        root + 5,
        root + 3,
        root,
        root + 7,
    ];
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        for (i, p) in pitches.iter().enumerate() {
            let pos = i as f64 * 0.5;
            let vel = if i % 2 == 0 { 0.9 } else { 0.65 };
            v.push(nt(o + pos, 0.45, *p, vel));
        }
        if bar % 4 == 2 {
            v.push(nt(o + 3.5, 0.4, root + 10, 0.7));
        }
    }
    v
}

fn reese_line(bars: usize, root: u8) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        for i in 0..8 {
            let p = if i % 4 == 3 { root + 12 } else { root };
            v.push(nt(
                o + i as f64 * 0.5,
                0.4,
                p,
                if i % 4 == 0 { 0.9 } else { 0.7 },
            ));
        }
    }
    v
}

fn stab_chords(bars: usize, _root: u8) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        let (root, ivs) = progression(bar);
        for i in 0..4 {
            let pos = i as f64 + 0.5;
            let vel = if i % 2 == 0 { 0.75 } else { 0.55 };
            chord(&mut v, root, ivs, o + pos, 0.28, vel);
        }
    }
    v
}

fn keys_chords(bars: usize, _root: u8) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        let (root, ivs) = progression(bar);
        chord(&mut v, root, ivs, o, 2.0, 0.65);
        chord(&mut v, root + 12, ivs, o + 2.5, 1.2, 0.45);
    }
    v
}

fn pad_chords(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        let (root, ivs) = minor_progression(bar);
        chord(&mut v, root + 12, ivs, o, 4.0, 0.45);
    }
    v
}

fn supersaw_chords(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        let (root, ivs) = progression(bar);
        chord(&mut v, root + 12, ivs, o, 3.5, 0.7);
        if bar % 4 == 3 {
            chord(&mut v, root + 12, ivs, o + 3.5, 0.5, 0.5);
        }
    }
    v
}

fn lead_melody(bars: usize) -> Vec<Note> {
    let motif: [(f64, f64, u8, f32); 8] = [
        (0.0, 0.45, 69, 0.8),
        (1.0, 0.45, 72, 0.75),
        (1.5, 0.45, 76, 0.85),
        (2.0, 0.9, 74, 0.8),
        (3.0, 0.45, 72, 0.7),
        (3.5, 0.45, 69, 0.7),
        (2.5, 0.4, 72, 0.6),
        (3.75, 0.25, 67, 0.6),
    ];
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        for (s, l, p, vel) in motif {
            if bar % 4 == 3 && s >= 3.0 {
                continue;
            }
            v.push(nt(o + s, l, p, vel));
        }
        if bar % 4 == 3 {
            v.push(nt(o + 3.0, 0.9, 64, 0.65));
            v.push(nt(o + 3.0, 0.9, 67, 0.55));
        }
    }
    v
}

fn riser_note(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        v.push(nt(bar as f64 * 4.0, 4.0, 60, 0.8));
    }
    v
}

// ---------------------------------------------------------------------------
// Track helpers
// ---------------------------------------------------------------------------

fn dev(kind: DeviceKind, preset: usize) -> Device {
    let ps = kind.presets();
    Device::from_preset(kind, &ps[preset.min(ps.len() - 1)])
}

/// Per-track level, pan and send settings used when building demo songs.
struct TrackMix {
    volume: f32,
    pan: f32,
    send: f32,
}

impl TrackMix {
    const fn new(volume: f32, pan: f32, send: f32) -> Self {
        Self { volume, pan, send }
    }
}

fn add_track(
    song: &mut Song,
    name: &str,
    kind: DeviceKind,
    preset: usize,
    mix: TrackMix,
    fx: &[(DeviceKind, usize)],
) -> usize {
    let ps = kind.presets();
    let idx = song.add_track_preset(kind, &ps[preset.min(ps.len() - 1)]);
    let t = &mut song.tracks[idx];
    t.name = name.to_string();
    t.volume = mix.volume;
    t.pan = mix.pan;
    t.send = mix.send;
    for (k, p) in fx {
        t.effects.push(dev(*k, *p));
    }
    idx
}

fn clip(track: &mut Track, name: &str, scene: i32, start: f64, bars: f64, notes: Vec<Note>) {
    let mut c = Clip::new(name, scene, start, bars * 4.0);
    c.notes = notes;
    track.clips.push(c);
}

fn session_and_arrangement(
    song: &mut Song,
    track: usize,
    clip_name: &str,
    pattern_bars: usize,
    pattern: &[Note],
    arrangement: &[(f64, usize)],
) {
    let t = &mut song.tracks[track];
    // A looping session clip to jam with.
    clip(t, clip_name, 0, 0.0, pattern_bars as f64, pattern.to_vec());
    // Arrangement sections.
    for (start_bar, bars) in arrangement {
        let mut notes = Vec::new();
        let reps = (*bars).div_ceil(pattern_bars);
        for r in 0..reps {
            for n in pattern {
                let mut nn = *n;
                nn.start += (r * pattern_bars) as f64 * 4.0;
                if nn.start < *bars as f64 * 4.0 {
                    notes.push(nn);
                }
            }
        }
        clip(t, clip_name, -1, *start_bar * 4.0, *bars as f64, notes);
    }
}

fn finish(mut s: Song) -> Song {
    s.loop_enabled = true;
    s.loop_start = 0.0;
    s.loop_end = 64.0;
    s.normalize();
    s
}

// ---------------------------------------------------------------------------
// Songs
// ---------------------------------------------------------------------------

fn house() -> Song {
    let mut s = Song::empty("Midnight House", 124.0);
    let drums = add_track(
        &mut s,
        "Drums",
        DeviceKind::DrumRack,
        0,
        TrackMix::new(0.85, 0.0, 0.1),
        &[(DeviceKind::Compressor, 0), (DeviceKind::Eq3, 0)],
    );
    let bass = add_track(
        &mut s,
        "Sub Bass",
        DeviceKind::SubBass,
        0,
        TrackMix::new(0.8, 0.0, 0.0),
        &[(DeviceKind::Sidechain, 0)],
    );
    let stabs = add_track(
        &mut s,
        "House Stabs",
        DeviceKind::Pluck,
        0,
        TrackMix::new(0.6, -0.15, 0.3),
        &[
            (DeviceKind::Chorus, 0),
            (DeviceKind::Delay, 0),
            (DeviceKind::Reverb, 0),
        ],
    );
    let pad = add_track(
        &mut s,
        "Warm Pad",
        DeviceKind::Pad,
        0,
        TrackMix::new(0.45, 0.2, 0.35),
        &[(DeviceKind::Chorus, 1), (DeviceKind::Reverb, 0)],
    );
    let lead = add_track(
        &mut s,
        "Super Lead",
        DeviceKind::Supersaw,
        0,
        TrackMix::new(0.55, 0.1, 0.18),
        &[
            (DeviceKind::Filter, 0),
            (DeviceKind::Delay, 0),
            (DeviceKind::Reverb, 0),
        ],
    );
    let riser = add_track(
        &mut s,
        "Riser FX",
        DeviceKind::Riser,
        0,
        TrackMix::new(0.5, 0.0, 0.4),
        &[(DeviceKind::Reverb, 0)],
    );

    let d = house_drums(4, true);
    session_and_arrangement(&mut s, drums, "Club Groove", 4, &d, &[(0.0, 8), (8.0, 8)]);
    let b = house_bass(4, 33);
    session_and_arrangement(&mut s, bass, "Offbeat Sub", 4, &b, &[(4.0, 4), (8.0, 8)]);
    let st = stab_chords(4, 57);
    session_and_arrangement(&mut s, stabs, "Am7 Stabs", 4, &st, &[(4.0, 8), (12.0, 4)]);
    let pd = pad_chords(4);
    session_and_arrangement(&mut s, pad, "Am Pad", 4, &pd, &[(0.0, 16)]);
    let ld = lead_melody(4);
    session_and_arrangement(&mut s, lead, "Festival Hook", 4, &ld, &[(8.0, 8)]);
    let rn = riser_note(2);
    session_and_arrangement(&mut s, riser, "Riser", 2, &rn, &[(6.0, 2), (14.0, 2)]);

    s.tracks[bass].effects[0] = dev(DeviceKind::Sidechain, 0);
    finish(s)
}

fn riddim() -> Song {
    let mut s = Song::empty("Riddim Protocol", 140.0);
    let drums = add_track(
        &mut s,
        "Half-Time Drums",
        DeviceKind::DrumRack,
        1,
        TrackMix::new(0.85, 0.0, 0.12),
        &[(DeviceKind::Compressor, 1), (DeviceKind::Drive, 0)],
    );
    let growl = add_track(
        &mut s,
        "Growl Bass",
        DeviceKind::GrowlBass,
        0,
        TrackMix::new(0.7, 0.0, 0.05),
        &[
            (DeviceKind::Filter, 1),
            (DeviceKind::Drive, 1),
            (DeviceKind::Bitcrush, 0),
        ],
    );
    let sub = add_track(
        &mut s,
        "Sub",
        DeviceKind::SubBass,
        2,
        TrackMix::new(0.85, 0.0, 0.0),
        &[(DeviceKind::Compressor, 1)],
    );
    let pad = add_track(
        &mut s,
        "Dark Pad",
        DeviceKind::Pad,
        1,
        TrackMix::new(0.3, 0.0, 0.4),
        &[(DeviceKind::Phaser, 0), (DeviceKind::Reverb, 1)],
    );
    let riser = add_track(
        &mut s,
        "Riser",
        DeviceKind::Riser,
        0,
        TrackMix::new(0.55, 0.0, 0.5),
        &[(DeviceKind::Delay, 0), (DeviceKind::Reverb, 0)],
    );
    let lead = add_track(
        &mut s,
        "Piano Lead",
        DeviceKind::Salamander,
        3,
        TrackMix::new(0.42, 0.12, 0.32),
        &[
            (DeviceKind::Chorus, 1),
            (DeviceKind::Delay, 0),
            (DeviceKind::Reverb, 0),
        ],
    );

    let d = half_time_drums(4);
    session_and_arrangement(&mut s, drums, "Riddim Beat", 4, &d, &[(0.0, 16)]);
    let g = growl_pattern(4, 30);
    session_and_arrangement(&mut s, growl, "Wobble A", 4, &g, &[(0.0, 8), (8.0, 8)]);
    let sb = long_sub(4, 30);
    session_and_arrangement(&mut s, sub, "Long Sub", 4, &sb, &[(0.0, 16)]);
    let pd = pad_chords(4);
    session_and_arrangement(&mut s, pad, "Dark Pad", 4, &pd, &[(0.0, 16)]);
    let ld = lead_melody(4);
    session_and_arrangement(&mut s, lead, "Piano Hook", 4, &ld, &[(8.0, 8)]);
    let rn = riser_note(2);
    session_and_arrangement(&mut s, riser, "Sweep", 2, &rn, &[(6.0, 2), (14.0, 2)]);
    finish(s)
}

const VOCAL_RIDDIM_TAKE_BYTES: &[u8] = include_bytes!("../assets/vocal-riddim.wav");

fn vocal_riddim_take() -> AudioTake {
    let wav = crate::samples::decode_wav(VOCAL_RIDDIM_TAKE_BYTES)
        .expect("embedded Vocal Riddim take decodes");
    AudioTake::from_samples("Vocal Riddim Take", wav.rate.round() as u32, &wav.data)
}

fn vocal_chop_pattern(bars: usize) -> Vec<Note> {
    const A: &[(f64, f64, f32)] = &[
        (0.0, 0.65, 0.95),
        (1.5, 0.45, 0.68),
        (2.25, 0.55, 0.86),
        (3.25, 0.45, 0.60),
    ];
    const B: &[(f64, f64, f32)] = &[
        (0.25, 0.45, 0.72),
        (1.0, 0.65, 0.92),
        (2.0, 0.9, 0.98),
        (3.5, 0.3, 0.55),
    ];
    const C: &[(f64, f64, f32)] = &[
        (0.0, 0.5, 0.90),
        (1.25, 0.5, 0.70),
        (2.5, 0.5, 0.82),
        (3.0, 0.85, 0.95),
    ];
    const D: &[(f64, f64, f32)] = &[
        (0.0, 0.45, 0.95),
        (0.75, 0.3, 0.55),
        (1.75, 0.55, 0.80),
        (2.5, 0.4, 0.70),
        (3.25, 0.65, 0.88),
    ];
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        let motif = match bar % 4 {
            0 => A,
            1 => B,
            2 => C,
            _ => D,
        };
        for (start, len, vel) in motif {
            v.push(nt(o + start, *len, 60, *vel));
        }
    }
    v
}

fn vocal_growl_pattern(bars: usize) -> Vec<Note> {
    const A: &[(f64, f64, f32)] = &[
        (0.0, 0.45, 1.0),
        (0.75, 0.3, 0.70),
        (1.5, 0.5, 0.92),
        (2.5, 0.4, 0.78),
        (3.25, 0.55, 0.95),
    ];
    const B: &[(f64, f64, f32)] = &[
        (0.0, 0.55, 0.95),
        (1.0, 0.4, 0.80),
        (2.0, 0.5, 0.90),
        (3.0, 0.45, 0.72),
    ];
    const C: &[(f64, f64, f32)] = &[
        (0.0, 0.4, 0.95),
        (0.5, 0.35, 0.62),
        (1.5, 0.5, 0.88),
        (2.25, 0.4, 0.76),
        (3.5, 0.3, 0.60),
    ];
    const D: &[(f64, f64, f32)] = &[
        (0.0, 0.6, 1.0),
        (1.25, 0.4, 0.74),
        (2.0, 0.5, 0.90),
        (3.0, 0.75, 0.98),
    ];
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        let motif = match bar % 4 {
            0 => A,
            1 => B,
            2 => C,
            _ => D,
        };
        for (start, len, vel) in motif {
            v.push(nt(o + start, *len, 60, *vel));
        }
    }
    v
}

/// A 16-bar vocal-led riddim. The supplied take is used twice: chopped and
/// pitched to A3 for the hook, and pitched down an octave through the same
/// Vocal Take -> Bassify -> Wobble chain as the Voice -> Bass import path.
fn vocal_riddim() -> Song {
    let mut s = Song::empty("Vocal Riddim", 140.0);
    let take = vocal_riddim_take();

    let hook = add_track(
        &mut s,
        "Vocal Hook",
        DeviceKind::VocalTake,
        0,
        TrackMix::new(0.62, -0.08, 0.24),
        &[
            (DeviceKind::Chorus, 0),
            (DeviceKind::Delay, 1),
            (DeviceKind::Reverb, 0),
            (DeviceKind::Eq3, 1),
        ],
    );
    {
        let t = &mut s.tracks[hook];
        t.take = Some(take.clone());
        t.instrument.set(0, 0.90);
        t.instrument.set(1, 0.0);
        t.instrument.set(3, 0.90);
        t.instrument.set(4, 0.14);
    }

    let harmony = add_track(
        &mut s,
        "Vocal Harmony",
        DeviceKind::VocalTake,
        0,
        TrackMix::new(0.26, 0.28, 0.36),
        &[
            (DeviceKind::Chorus, 1),
            (DeviceKind::Delay, 0),
            (DeviceKind::Reverb, 0),
        ],
    );
    {
        let t = &mut s.tracks[harmony];
        t.take = Some(take.clone());
        t.instrument.set(0, 0.55);
        t.instrument.set(1, 7.0);
        t.instrument.set(3, 0.72);
        t.instrument.set(4, 0.20);
    }

    let vocal_growl = add_vocal_bass_track(&mut s, take.clone(), VocalStyle::Dubstep);
    {
        let t = &mut s.tracks[vocal_growl];
        t.name = "Vocal Growl".to_string();
        t.volume = 0.76;
        t.send = 0.04;
        t.instrument.set(3, 0.58);
        if let Some(c) = t.clips.iter_mut().find(|c| c.scene == 0) {
            c.length = 16.0;
            c.notes = vocal_growl_pattern(4);
        }
        if let Some(c) = t.clips.iter_mut().find(|c| c.scene == -1) {
            c.start = 16.0;
            c.length = 16.0;
            c.notes = vocal_growl_pattern(4);
        }
    }
    clip(
        &mut s.tracks[vocal_growl],
        "Vocal Growl B",
        -1,
        48.0,
        4.0,
        vocal_growl_pattern(4),
    );

    let drums = add_track(
        &mut s,
        "Half-Time Drums",
        DeviceKind::DrumRack,
        3,
        TrackMix::new(0.85, 0.0, 0.12),
        &[
            (DeviceKind::Compressor, 1),
            (DeviceKind::Drive, 0),
            (DeviceKind::Transient, 0),
        ],
    );
    let sub = add_track(
        &mut s,
        "Sub",
        DeviceKind::SubBass,
        2,
        TrackMix::new(0.82, 0.0, 0.0),
        &[(DeviceKind::Compressor, 1)],
    );
    let synth_growl = add_track(
        &mut s,
        "Growl Bass",
        DeviceKind::GrowlBass,
        0,
        TrackMix::new(0.58, 0.0, 0.05),
        &[
            (DeviceKind::Filter, 1),
            (DeviceKind::Drive, 1),
            (DeviceKind::Bitcrush, 0),
            (DeviceKind::Chorus, 0),
        ],
    );
    let pad = add_track(
        &mut s,
        "Dark Pad",
        DeviceKind::Pad,
        1,
        TrackMix::new(0.26, 0.0, 0.42),
        &[(DeviceKind::Phaser, 0), (DeviceKind::Reverb, 1)],
    );
    let piano = add_track(
        &mut s,
        "Piano Hook",
        DeviceKind::Salamander,
        3,
        TrackMix::new(0.30, 0.16, 0.34),
        &[
            (DeviceKind::Chorus, 1),
            (DeviceKind::Delay, 0),
            (DeviceKind::Reverb, 0),
        ],
    );
    let riser = add_track(
        &mut s,
        "Riser",
        DeviceKind::Riser,
        0,
        TrackMix::new(0.50, 0.0, 0.48),
        &[(DeviceKind::Delay, 0), (DeviceKind::Reverb, 0)],
    );

    let chops = vocal_chop_pattern(4);
    session_and_arrangement(&mut s, hook, "Vocal Chops", 4, &chops, &[(0.0, 16)]);
    session_and_arrangement(&mut s, harmony, "Vocal Harmony", 4, &chops, &[(12.0, 4)]);

    let d = half_time_drums(4);
    session_and_arrangement(
        &mut s,
        drums,
        "Vocal Riddim Beat",
        4,
        &d,
        &[(4.0, 4), (12.0, 4)],
    );
    let sb = long_sub(4, 33);
    session_and_arrangement(&mut s, sub, "Long Sub", 4, &sb, &[(4.0, 4), (12.0, 4)]);
    let g = growl_pattern(4, 33);
    session_and_arrangement(
        &mut s,
        synth_growl,
        "Wobble Drop",
        4,
        &g,
        &[(4.0, 4), (12.0, 4)],
    );
    let pd = pad_chords(4);
    session_and_arrangement(&mut s, pad, "Vocal Riddim Pad", 4, &pd, &[(0.0, 16)]);
    let lead = lead_melody(4);
    session_and_arrangement(&mut s, piano, "Piano Answer", 4, &lead, &[(8.0, 8)]);
    let rn = riser_note(1);
    session_and_arrangement(
        &mut s,
        riser,
        "Sweep",
        1,
        &rn,
        &[(3.0, 1), (7.0, 1), (11.0, 1)],
    );

    finish(s)
}

fn big_room() -> Song {
    let mut s = Song::empty("Big Room Anthem", 128.0);
    let drums = add_track(
        &mut s,
        "Big Drums",
        DeviceKind::DrumRack,
        2,
        TrackMix::new(0.85, 0.0, 0.1),
        &[(DeviceKind::Compressor, 1), (DeviceKind::Eq3, 1)],
    );
    let chords = add_track(
        &mut s,
        "Supersaw Chords",
        DeviceKind::Supersaw,
        1,
        TrackMix::new(0.82, 0.0, 0.25),
        &[
            (DeviceKind::Sidechain, 1),
            (DeviceKind::Chorus, 1),
            (DeviceKind::Reverb, 0),
        ],
    );
    let bass = add_track(
        &mut s,
        "Sub Bass",
        DeviceKind::SubBass,
        1,
        TrackMix::new(0.92, 0.0, 0.0),
        &[(DeviceKind::Sidechain, 1)],
    );
    let lead = add_track(
        &mut s,
        "Pluck Lead",
        DeviceKind::Pluck,
        2,
        TrackMix::new(0.78, 0.25, 0.2),
        &[(DeviceKind::Delay, 0), (DeviceKind::Reverb, 0)],
    );
    let riser = add_track(
        &mut s,
        "Riser",
        DeviceKind::Riser,
        1,
        TrackMix::new(0.62, 0.0, 0.4),
        &[(DeviceKind::Reverb, 0)],
    );

    // A real big-room shape: four bars of drums, a four-bar drums-out build
    // with the supersaws and riser, then the drop opens at bar 9 with bass,
    // lead, full drums and a crash. Rendering the same loop under every section
    // left the demo's section RMS almost flat, so this arrangement is the
    // audible build/drop rather than only a label.
    let d = big_room_drums(4);
    session_and_arrangement(&mut s, drums, "Main Beat", 4, &d, &[(0.0, 4), (8.0, 8)]);
    let ch = supersaw_chords(4);
    session_and_arrangement(&mut s, chords, "Big Chords", 4, &ch, &[(4.0, 12)]);
    let b = house_bass(4, 33);
    session_and_arrangement(&mut s, bass, "Pumping Sub", 4, &b, &[(8.0, 8)]);
    let l = lead_melody(4);
    session_and_arrangement(&mut s, lead, "Pluck Hook", 4, &l, &[(8.0, 8)]);
    let rn = riser_note(4);
    session_and_arrangement(&mut s, riser, "Build", 4, &rn, &[(4.0, 4)]);
    finish(s)
}

fn deep_house() -> Song {
    let mut s = Song::empty("Deep Groove", 122.0);
    let drums = add_track(
        &mut s,
        "Shuffled Drums",
        DeviceKind::DrumRack,
        0,
        TrackMix::new(0.8, 0.0, 0.15),
        &[(DeviceKind::Compressor, 0), (DeviceKind::Filter, 0)],
    );
    let keys = add_track(
        &mut s,
        "FM Keys",
        DeviceKind::Keys,
        2,
        TrackMix::new(0.55, -0.1, 0.35),
        &[(DeviceKind::Chorus, 0), (DeviceKind::Reverb, 0)],
    );
    let bass = add_track(
        &mut s,
        "Warm Sub",
        DeviceKind::SubBass,
        1,
        TrackMix::new(0.75, 0.0, 0.0),
        &[(DeviceKind::Compressor, 0)],
    );
    let pluck = add_track(
        &mut s,
        "Dream Pluck",
        DeviceKind::Pluck,
        1,
        TrackMix::new(0.4, 0.3, 0.3),
        &[(DeviceKind::Delay, 1), (DeviceKind::Reverb, 1)],
    );
    let pad = add_track(
        &mut s,
        "Soft Pad",
        DeviceKind::Pad,
        0,
        TrackMix::new(0.35, 0.0, 0.4),
        &[(DeviceKind::Chorus, 1), (DeviceKind::Reverb, 1)],
    );

    let d = deep_drums(4);
    session_and_arrangement(&mut s, drums, "Deep Groove", 4, &d, &[(0.0, 16)]);
    let k = keys_chords(4, 57);
    session_and_arrangement(&mut s, keys, "Rhodes Chords", 4, &k, &[(0.0, 16)]);
    let b = house_bass(4, 33);
    session_and_arrangement(&mut s, bass, "Warm Bass", 4, &b, &[(0.0, 16)]);
    let p = stab_chords(4, 57);
    session_and_arrangement(&mut s, pluck, "Pluck Stabs", 4, &p, &[(4.0, 12)]);
    let pd = pad_chords(4);
    session_and_arrangement(&mut s, pad, "Pad Bed", 4, &pd, &[(0.0, 16)]);
    finish(s)
}

fn techno() -> Song {
    let mut s = Song::empty("Warehouse Techno", 132.0);
    let drums = add_track(
        &mut s,
        "Warehouse Drums",
        DeviceKind::DrumRack,
        2,
        TrackMix::new(0.85, 0.0, 0.1),
        &[(DeviceKind::Drive, 2), (DeviceKind::Compressor, 1)],
    );
    let bass = add_track(
        &mut s,
        "Reese Bass",
        DeviceKind::ReeseBass,
        2,
        TrackMix::new(0.65, 0.0, 0.0),
        &[(DeviceKind::Filter, 0), (DeviceKind::Compressor, 1)],
    );
    let keys = add_track(
        &mut s,
        "Dark Stabs",
        DeviceKind::Keys,
        2,
        TrackMix::new(0.45, 0.1, 0.25),
        &[(DeviceKind::Phaser, 0), (DeviceKind::Delay, 1)],
    );
    let pad = add_track(
        &mut s,
        "Metal Pad",
        DeviceKind::Pad,
        2,
        TrackMix::new(0.3, -0.2, 0.35),
        &[(DeviceKind::Phaser, 1), (DeviceKind::Reverb, 1)],
    );
    let riser = add_track(
        &mut s,
        "Riser",
        DeviceKind::Riser,
        2,
        TrackMix::new(0.4, 0.0, 0.4),
        &[(DeviceKind::Reverb, 0)],
    );

    let d = techno_drums(4);
    session_and_arrangement(&mut s, drums, "Techno Beat", 4, &d, &[(0.0, 16)]);
    let b = reese_line(4, 33);
    session_and_arrangement(&mut s, bass, "Reese Line", 4, &b, &[(0.0, 16)]);
    let k = keys_chords(4, 57);
    session_and_arrangement(&mut s, keys, "Dark Stabs", 4, &k, &[(4.0, 12)]);
    let pd = pad_chords(4);
    session_and_arrangement(&mut s, pad, "Metal Bed", 4, &pd, &[(0.0, 16)]);
    let rn = riser_note(2);
    session_and_arrangement(&mut s, riser, "Sweep", 2, &rn, &[(6.0, 2), (14.0, 2)]);
    finish(s)
}

// ---------------------------------------------------------------------------
// Sample-forward demos: melodic bass, live-feeling piano and rap
// ---------------------------------------------------------------------------

fn vocal_bass_line(bars: usize, root: u8) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        let steps = [
            (0.0, 0.7, root, 0.95),
            (0.75, 0.22, root, 0.6),
            (1.5, 0.45, root + 12, 0.8),
            (2.25, 0.4, root + 3, 0.7),
            (3.0, 0.45, root + 5, 0.78),
            (3.5, 0.45, root + 3, 0.68),
        ];
        for (s, l, p, vel) in steps {
            v.push(nt(o + s, l, p, vel));
        }
        if bar % 4 == 3 {
            v.push(nt(o + 2.5, 0.35, root + 10, 0.7));
        }
    }
    v
}

fn neon_piano(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        let (root, ivs): (u8, &[i32]) = match bar % 4 {
            0 => (57, &[0, 7, 10, 14]), // Am9
            1 => (53, &[0, 7, 11, 16]), // Fmaj9
            2 => (48, &[0, 7, 12, 16]), // Cmaj7
            _ => (55, &[0, 7, 10, 14]), // G9
        };
        chord(&mut v, root, ivs, o, 3.6, 0.52);
        chord(&mut v, root + 12, &[0, 7], o + 0.5, 2.8, 0.32);
        match bar % 4 {
            0 => {
                v.push(nt(o + 0.5, 0.75, 76, 0.72));
                v.push(nt(o + 1.5, 0.75, 81, 0.8));
                v.push(nt(o + 2.5, 1.35, 84, 0.85));
            }
            1 => {
                v.push(nt(o + 0.5, 0.75, 81, 0.78));
                v.push(nt(o + 1.5, 0.75, 77, 0.68));
                v.push(nt(o + 2.5, 1.35, 76, 0.72));
            }
            2 => {
                v.push(nt(o + 0.5, 0.75, 79, 0.7));
                v.push(nt(o + 1.5, 0.75, 76, 0.66));
                v.push(nt(o + 2.5, 1.35, 72, 0.68));
            }
            _ => {
                v.push(nt(o + 0.5, 0.5, 71, 0.62));
                v.push(nt(o + 1.5, 0.5, 74, 0.66));
                v.push(nt(o + 2.25, 1.6, 79, 0.76));
            }
        }
    }
    v
}

fn ballad_progression(bar: usize) -> (u8, &'static [i32]) {
    match bar % 4 {
        0 => (50, &[0, 7, 11, 14]), // Dmaj9
        1 => (47, &[0, 7, 10, 14]), // Bm9
        2 => (43, &[0, 7, 11, 14]), // Gmaj9
        _ => (45, &[0, 7, 10, 14]), // A9
    }
}

fn piano_ballad(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        let (root, ivs) = ballad_progression(bar);
        chord(&mut v, root, ivs, o, 3.8, 0.55);
        chord(&mut v, root + 12, &[0, 7], o + 0.25, 3.2, 0.34);
        match bar % 4 {
            0 => {
                v.push(nt(o + 0.5, 0.75, 74, 0.7));
                v.push(nt(o + 1.5, 0.75, 78, 0.78));
                v.push(nt(o + 2.5, 1.4, 81, 0.82));
            }
            1 => {
                v.push(nt(o + 0.5, 0.75, 78, 0.75));
                v.push(nt(o + 1.5, 0.75, 76, 0.7));
                v.push(nt(o + 2.5, 1.4, 74, 0.72));
            }
            2 => {
                v.push(nt(o + 0.5, 0.75, 71, 0.62));
                v.push(nt(o + 1.5, 0.75, 74, 0.68));
                v.push(nt(o + 2.5, 1.4, 79, 0.8));
            }
            _ => {
                v.push(nt(o + 0.5, 1.0, 81, 0.78));
                v.push(nt(o + 1.5, 0.5, 76, 0.65));
                v.push(nt(o + 2.25, 1.65, 73, 0.68));
            }
        }
    }
    v
}

fn ballad_pad(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        let (root, ivs) = ballad_progression(bar);
        chord(&mut v, root + 12, ivs, o, 4.0, 0.4);
    }
    v
}

fn salamander_bass_line(bars: usize, root: u8) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        v.push(nt(o, 1.5, root, 0.86));
        v.push(nt(o + 1.75, 0.5, root, 0.62));
        v.push(nt(o + 2.5, 1.3, root + 7, 0.78));
        if bar % 4 == 3 {
            v.push(nt(o + 3.75, 0.22, root + 12, 0.55));
        }
    }
    v
}

fn cypher_drums(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        v.push(nt(o, 0.25, 36, 1.0));
        v.push(nt(o + 2.5, 0.25, 36, 0.9));
        if bar % 2 == 1 {
            v.push(nt(o + 3.25, 0.2, 36, 0.66));
        }
        v.push(nt(o + 1.0, 0.2, 38, 0.95));
        v.push(nt(o + 3.0, 0.2, 38, 0.9));
        v.push(nt(o + 1.0, 0.2, 39, 0.42));
        v.push(nt(o + 3.0, 0.2, 39, 0.38));
        v.push(nt(o + 2.75, 0.12, 38, 0.3));
        v.push(nt(o, 0.5, 35, 0.62));
        for step in 0..8 {
            let pos = step as f64 * 0.5;
            let vel = if step % 2 == 0 { 0.32 } else { 0.2 };
            v.push(nt(o + pos, 0.1, 42, vel));
        }
        for step in 0..4 {
            v.push(nt(o + step as f64 * 1.0 + 0.75, 0.08, 70, 0.18));
        }
        if bar % 4 == 0 {
            v.push(nt(o, 1.0, 49, 0.35));
        }
        if bar % 8 == 7 {
            v.push(nt(o + 3.5, 0.2, 45, 0.55));
            v.push(nt(o + 3.75, 0.2, 48, 0.62));
        }
    }
    v
}

fn cypher_keys(bars: usize) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        let (root, ivs): (u8, &[i32]) = match bar % 4 {
            0 => (49, &[0, 7, 10, 14]), // C#m9
            1 => (45, &[0, 7, 11, 16]), // Amaj9
            2 => (52, &[0, 7, 11, 14]), // E9
            _ => (47, &[0, 4, 7, 10]),  // B7
        };
        chord(&mut v, root, ivs, o, 3.75, 0.5);
        chord(&mut v, root + 12, &[0, 7], o + 0.5, 2.6, 0.28);
        match bar % 4 {
            0 => {
                v.push(nt(o + 0.5, 0.5, 61, 0.62));
                v.push(nt(o + 1.5, 0.5, 64, 0.7));
                v.push(nt(o + 2.5, 1.0, 68, 0.74));
            }
            1 => {
                v.push(nt(o + 0.5, 0.5, 64, 0.62));
                v.push(nt(o + 1.5, 0.5, 61, 0.58));
            }
            2 => {
                v.push(nt(o + 0.5, 0.5, 63, 0.58));
                v.push(nt(o + 1.5, 0.5, 66, 0.62));
                v.push(nt(o + 2.5, 1.0, 71, 0.7));
            }
            _ => {
                v.push(nt(o + 0.5, 0.75, 66, 0.58));
                v.push(nt(o + 2.0, 1.0, 63, 0.55));
            }
        }
    }
    v
}

fn rap_808(bars: usize, root: u8) -> Vec<Note> {
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        v.push(nt(o, 1.25, root, 0.95));
        v.push(nt(o + 1.75, 0.5, root, 0.7));
        v.push(nt(o + 2.5, 1.15, root + 7, 0.82));
        if bar % 4 == 3 {
            v.push(nt(o + 3.75, 0.2, root + 12, 0.6));
        }
    }
    v
}

fn neon_cathedral() -> Song {
    let mut s = Song::empty("Neon Cathedral", 150.0);
    let drums = add_track(
        &mut s,
        "Half-Time Drums",
        DeviceKind::DrumRack,
        3,
        TrackMix::new(0.82, 0.0, 0.12),
        &[
            (DeviceKind::Transient, 0),
            (DeviceKind::Drive, 0),
            (DeviceKind::Compressor, 1),
        ],
    );
    let sub = add_track(
        &mut s,
        "808 Sub",
        DeviceKind::SubBass,
        0,
        TrackMix::new(0.68, 0.0, 0.0),
        &[(DeviceKind::Drive, 0), (DeviceKind::Compressor, 0)],
    );
    let vox = add_track(
        &mut s,
        "Vox Bass Lead",
        DeviceKind::VoxBass,
        0,
        TrackMix::new(0.62, 0.0, 0.18),
        &[
            (DeviceKind::Bassify, 0),
            (DeviceKind::Wobble, 0),
            (DeviceKind::Delay, 0),
            (DeviceKind::Reverb, 1),
        ],
    );
    let piano = add_track(
        &mut s,
        "Salamander Piano",
        DeviceKind::Salamander,
        3,
        TrackMix::new(0.58, -0.1, 0.3),
        &[
            (DeviceKind::Chorus, 1),
            (DeviceKind::Tape, 0),
            (DeviceKind::Reverb, 0),
        ],
    );
    let pad = add_track(
        &mut s,
        "Cathedral Pad",
        DeviceKind::Pad,
        1,
        TrackMix::new(0.3, 0.15, 0.4),
        &[(DeviceKind::Chorus, 0), (DeviceKind::Reverb, 0)],
    );
    let riser = add_track(
        &mut s,
        "Riser",
        DeviceKind::Riser,
        0,
        TrackMix::new(0.42, 0.0, 0.45),
        &[(DeviceKind::Delay, 0), (DeviceKind::Reverb, 0)],
    );

    let d = half_time_drums(4);
    session_and_arrangement(&mut s, drums, "Half-Time", 4, &d, &[(0.0, 16)]);
    let sb = long_sub(4, 33);
    session_and_arrangement(&mut s, sub, "Long 808", 4, &sb, &[(0.0, 16)]);
    let vb = vocal_bass_line(4, 33);
    session_and_arrangement(&mut s, vox, "Talking Bass", 4, &vb, &[(4.0, 12)]);
    let pn = neon_piano(4);
    session_and_arrangement(&mut s, piano, "Neon Piano", 4, &pn, &[(0.0, 16)]);
    let pd = pad_chords(4);
    session_and_arrangement(&mut s, pad, "Cathedral Pad", 4, &pd, &[(0.0, 16)]);
    let rn = riser_note(2);
    session_and_arrangement(&mut s, riser, "Sweep", 2, &rn, &[(6.0, 2), (14.0, 2)]);

    let t = &mut s.tracks[vox];
    t.effects[0].set(0, 0.55);
    t.effects[0].set(6, 0.55);
    t.effects[1].set(0, 6.0);
    finish(s)
}

fn velvet_sky() -> Song {
    let mut s = Song::empty("Velvet Sky", 108.0);
    let drums = add_track(
        &mut s,
        "Acoustic Drums",
        DeviceKind::DrumRack,
        0,
        TrackMix::new(0.8, 0.0, 0.18),
        &[(DeviceKind::Transient, 0), (DeviceKind::Compressor, 0)],
    );
    let piano = add_track(
        &mut s,
        "Salamander Grand",
        DeviceKind::Salamander,
        3,
        TrackMix::new(0.66, -0.08, 0.3),
        &[
            (DeviceKind::Chorus, 1),
            (DeviceKind::Tape, 0),
            (DeviceKind::Reverb, 1),
        ],
    );
    let bass = add_track(
        &mut s,
        "Salamander Bass",
        DeviceKind::Salamander,
        2,
        TrackMix::new(0.5, 0.0, 0.0),
        &[(DeviceKind::Tape, 0), (DeviceKind::Compressor, 0)],
    );
    let sub = add_track(
        &mut s,
        "Warm Sub",
        DeviceKind::SubBass,
        0,
        TrackMix::new(0.5, 0.0, 0.0),
        &[(DeviceKind::Compressor, 0)],
    );
    let pad = add_track(
        &mut s,
        "Air Pad",
        DeviceKind::Pad,
        0,
        TrackMix::new(0.3, 0.15, 0.42),
        &[(DeviceKind::Chorus, 0), (DeviceKind::Reverb, 1)],
    );

    let d = deep_drums(4);
    session_and_arrangement(&mut s, drums, "Velvet Groove", 4, &d, &[(0.0, 16)]);
    let pn = piano_ballad(4);
    session_and_arrangement(&mut s, piano, "Piano Theme", 4, &pn, &[(0.0, 16)]);
    let bass_notes = salamander_bass_line(4, 38);
    session_and_arrangement(&mut s, bass, "Sample Bass", 4, &bass_notes, &[(0.0, 16)]);
    let sb = house_bass(4, 38);
    session_and_arrangement(&mut s, sub, "Warm Sub", 4, &sb, &[(0.0, 16)]);
    let pd = ballad_pad(4);
    session_and_arrangement(&mut s, pad, "Air Bed", 4, &pd, &[(0.0, 16)]);
    finish(s)
}

fn rap_flow_line(bars: usize) -> Vec<Note> {
    // C# minor topline. Each bar re-roots with the Cypher progression while
    // the rhythm stays in a rapped, conversational pocket.
    let roots = [61u8, 57, 64, 59];
    let intervals = [0i32, 3, 7, 10, 12, 15];
    let motif: [(f64, usize, f64, f32); 8] = [
        (0.0, 0, 0.28, 0.76),
        (0.5, 2, 0.22, 0.68),
        (1.0, 1, 0.32, 0.72),
        (1.5, 4, 0.2, 0.62),
        (2.0, 2, 0.28, 0.7),
        (2.75, 5, 0.18, 0.58),
        (3.0, 3, 0.42, 0.68),
        (3.5, 1, 0.22, 0.6),
    ];
    let alt: [(f64, usize, f64, f32); 8] = [
        (0.0, 4, 0.24, 0.72),
        (0.75, 2, 0.2, 0.64),
        (1.0, 0, 0.3, 0.74),
        (1.75, 3, 0.22, 0.62),
        (2.5, 1, 0.26, 0.68),
        (2.75, 0, 0.2, 0.6),
        (3.5, 5, 0.16, 0.58),
        (3.75, 2, 0.2, 0.6),
    ];
    let mut v = Vec::new();
    for bar in 0..bars {
        let o = bar as f64 * 4.0;
        let root = roots[bar % roots.len()] as i32;
        let steps = if bar % 2 == 1 { &alt[..] } else { &motif[..] };
        for (s, deg, len, vel) in steps {
            let p = (root + intervals[(*deg).min(intervals.len() - 1)]).clamp(0, 127) as u8;
            v.push(nt(o + *s, *len, p, *vel));
        }
    }
    v
}

fn cypher_room() -> Song {
    let mut s = Song::empty("Cypher Room", 90.0);
    let drums = add_track(
        &mut s,
        "Boom Bap Drums",
        DeviceKind::DrumRack,
        4,
        TrackMix::new(0.84, 0.0, 0.14),
        &[
            (DeviceKind::Transient, 0),
            (DeviceKind::Tape, 1),
            (DeviceKind::Compressor, 0),
        ],
    );
    let piano = add_track(
        &mut s,
        "Felt Piano",
        DeviceKind::Salamander,
        1,
        TrackMix::new(0.6, -0.12, 0.28),
        &[(DeviceKind::Tape, 0), (DeviceKind::Reverb, 1)],
    );
    let bass = add_track(
        &mut s,
        "Bass Sample",
        DeviceKind::Salamander,
        2,
        TrackMix::new(0.56, 0.0, 0.0),
        &[(DeviceKind::Transient, 0), (DeviceKind::Tape, 0)],
    );
    let sub = add_track(
        &mut s,
        "Slide 808",
        DeviceKind::SubBass,
        0,
        TrackMix::new(0.64, 0.0, 0.0),
        &[(DeviceKind::Drive, 0), (DeviceKind::Compressor, 0)],
    );
    let flow = add_track(
        &mut s,
        "Rap Flow",
        DeviceKind::Keys,
        2,
        TrackMix::new(0.86, 0.08, 0.16),
        &[
            (DeviceKind::Chorus, 0),
            (DeviceKind::Delay, 0),
            (DeviceKind::Compressor, 1),
            (DeviceKind::Eq3, 0),
            (DeviceKind::Reverb, 1),
        ],
    );

    let d = cypher_drums(4);
    session_and_arrangement(&mut s, drums, "Cypher Beat", 4, &d, &[(0.0, 16)]);
    let k = cypher_keys(4);
    session_and_arrangement(&mut s, piano, "Piano Loop", 4, &k, &[(0.0, 16)]);
    let b = salamander_bass_line(4, 37);
    session_and_arrangement(&mut s, bass, "Bass Sample", 4, &b, &[(0.0, 16)]);
    let e = rap_808(4, 37);
    session_and_arrangement(&mut s, sub, "Slide 808", 4, &e, &[(0.0, 16)]);
    let flow_notes = rap_flow_line(4);
    session_and_arrangement(&mut s, flow, "Rap Flow", 4, &flow_notes, &[(8.0, 8)]);

    // The flow is the vocal-equivalent foreground in this starter. Give it
    // enough level and presence to sit above the 808 and boom-bap bed, and
    // keep the compression/air controlled rather than washing it out with the
    // reverb return.
    s.tracks[flow].instrument.set(0, 1.15);
    s.tracks[flow].instrument.set(7, 0.68);
    let comp = &mut s.tracks[flow].effects[2];
    comp.set(0, -22.0);
    comp.set(1, 4.0);
    comp.set(2, 0.006);
    comp.set(3, 0.10);
    comp.set(4, 4.0);
    let eq = &mut s.tracks[flow].effects[3];
    eq.set(0, -3.0);
    eq.set(1, 3.0);
    eq.set(2, 1.5);
    eq.set(3, 1400.0);
    finish(s)
}

fn organ_house() -> Song {
    let mut s = Song::empty("Organ House", 124.0);
    let drums = add_track(
        &mut s,
        "Club Drums",
        DeviceKind::DrumRack,
        1,
        TrackMix::new(0.84, 0.0, 0.1),
        &[(DeviceKind::Compressor, 0), (DeviceKind::Eq3, 0)],
    );
    let sub = add_track(
        &mut s,
        "Offbeat Sub",
        DeviceKind::SubBass,
        0,
        TrackMix::new(0.76, 0.0, 0.0),
        &[(DeviceKind::Sidechain, 1)],
    );
    let organ = add_track(
        &mut s,
        "Tonewheel Organ",
        DeviceKind::Organ,
        1,
        TrackMix::new(0.56, -0.12, 0.28),
        &[
            (DeviceKind::Formant, 1),
            (DeviceKind::Chorus, 0),
            (DeviceKind::Reverb, 0),
        ],
    );
    let lead = add_track(
        &mut s,
        "Gospel Lead",
        DeviceKind::Organ,
        0,
        TrackMix::new(0.38, 0.18, 0.24),
        &[
            (DeviceKind::Drive, 0),
            (DeviceKind::Delay, 0),
            (DeviceKind::Reverb, 0),
        ],
    );
    let pad = add_track(
        &mut s,
        "Air Pad",
        DeviceKind::Pad,
        0,
        TrackMix::new(0.26, 0.1, 0.4),
        &[(DeviceKind::Chorus, 0), (DeviceKind::Reverb, 0)],
    );
    let riser = add_track(
        &mut s,
        "Riser FX",
        DeviceKind::Riser,
        0,
        TrackMix::new(0.45, 0.0, 0.4),
        &[(DeviceKind::Reverb, 0)],
    );

    let d = house_drums(4, true);
    session_and_arrangement(&mut s, drums, "Club Groove", 4, &d, &[(0.0, 16)]);
    let b = house_bass(4, 33);
    session_and_arrangement(&mut s, sub, "Offbeat Sub", 4, &b, &[(0.0, 16)]);
    let k = keys_chords(4, 57);
    session_and_arrangement(&mut s, organ, "Organ Chords", 4, &k, &[(0.0, 16)]);
    let ld = lead_melody(4);
    session_and_arrangement(&mut s, lead, "Gospel Hook", 4, &ld, &[(8.0, 8)]);
    let pd = pad_chords(4);
    session_and_arrangement(&mut s, pad, "Air Bed", 4, &pd, &[(0.0, 16)]);
    let rn = riser_note(2);
    session_and_arrangement(&mut s, riser, "Sweep", 2, &rn, &[(6.0, 2), (14.0, 2)]);

    // Keep the vowel movement audible without turning the chords into a FX
    // solo: the preset is intentionally wet, so pull the mix back here.
    if let Some(formant) = s.tracks[organ].effects.first_mut() {
        formant.set(1, 0.3);
        formant.set(2, 0.35);
        formant.set(4, 0.55);
    }
    finish(s)
}

fn live_room() -> Song {
    let mut s = Song::empty("Live Room", 118.0);
    let drums = add_track(
        &mut s,
        "Salamander Studio Kit",
        DeviceKind::DrumRack,
        6,
        TrackMix::new(0.84, 0.0, 0.12),
        &[
            (DeviceKind::Transient, 1),
            (DeviceKind::Tape, 0),
            (DeviceKind::Compressor, 0),
        ],
    );
    let perc = add_track(
        &mut s,
        "Hand Percussion",
        DeviceKind::DrumRack,
        9,
        TrackMix::new(0.46, -0.2, 0.18),
        &[(DeviceKind::Transient, 0), (DeviceKind::Compressor, 0)],
    );
    let piano = add_track(
        &mut s,
        "Felt Piano",
        DeviceKind::Salamander,
        1,
        TrackMix::new(0.58, 0.08, 0.3),
        &[(DeviceKind::Tape, 0), (DeviceKind::Reverb, 1)],
    );
    let sub = add_track(
        &mut s,
        "Warm Sub",
        DeviceKind::SubBass,
        0,
        TrackMix::new(0.68, 0.0, 0.0),
        &[(DeviceKind::Sidechain, 1), (DeviceKind::Compressor, 0)],
    );
    let pad = add_track(
        &mut s,
        "Room Pad",
        DeviceKind::Pad,
        0,
        TrackMix::new(0.24, 0.12, 0.4),
        &[(DeviceKind::Chorus, 0), (DeviceKind::Reverb, 0)],
    );

    let d = house_drums(4, true);
    session_and_arrangement(&mut s, drums, "Live Room Beat", 4, &d, &[(0.0, 16)]);
    let p = world_percussion(4);
    session_and_arrangement(&mut s, perc, "Hand Percussion", 4, &p, &[(0.0, 16)]);
    let ch = keys_chords(4, 57);
    session_and_arrangement(&mut s, piano, "Felt Piano Chords", 4, &ch, &[(0.0, 16)]);
    let b = house_bass(4, 33);
    session_and_arrangement(&mut s, sub, "Warm Sub", 4, &b, &[(0.0, 16)]);
    let pd = pad_chords(4);
    session_and_arrangement(&mut s, pad, "Room Bed", 4, &pd, &[(0.0, 16)]);
    finish(s)
}

// ---------------------------------------------------------------------------
// Default clip content for empty session slots
// ---------------------------------------------------------------------------

pub fn default_notes(kind: DeviceKind) -> Vec<Note> {
    match kind {
        DeviceKind::DrumRack => house_drums(2, false),
        DeviceKind::SubBass => house_bass(2, 33),
        DeviceKind::ReeseBass => reese_line(2, 33),
        DeviceKind::GrowlBass => growl_pattern(2, 30),
        DeviceKind::Supersaw => supersaw_chords(2),
        DeviceKind::Pluck => stab_chords(2, 57),
        DeviceKind::Keys => keys_chords(2, 57),
        DeviceKind::Organ => keys_chords(2, 57),
        DeviceKind::Pad => pad_chords(2),
        DeviceKind::Riser => vec![nt(0.0, 8.0, 60, 0.8)],
        DeviceKind::Salamander => piano_ballad(2),
        DeviceKind::VoxBass => vocal_bass_line(2, 33),
        DeviceKind::VocalTake => Vec::new(),
        _ => Vec::new(),
    }
}

pub fn default_clip(kind: DeviceKind, scene: i32, length_bars: f64) -> Clip {
    let mut c = Clip::new(kind.name(), scene, 0.0, length_bars * 4.0);
    let base = default_notes(kind);
    let pattern_beats = 8.0;
    let reps = ((length_bars * 4.0) / pattern_beats).ceil() as usize;
    for r in 0..reps {
        for n in &base {
            let mut nn = *n;
            nn.start += r as f64 * pattern_beats;
            if nn.start < c.length {
                c.notes.push(nn);
            }
        }
    }
    c
}

pub fn empty_clip(name: &str, scene: i32, start: f64, length_bars: f64) -> Clip {
    Clip::new(name, scene, start, length_bars * 4.0)
}
