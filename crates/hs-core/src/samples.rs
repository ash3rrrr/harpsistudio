//! Embedded sample bank. Keeping the samples inside the binary means kits and
//! the Salamander piano work offline, with no import dialog or missing-file
//! state. Assets are mono 16-bit 44.1 kHz PCM WAVs, peak-normalized per
//! one-shot so no kit lane clips and every hit is audible. A measured,
//! gain-only per-kit trim aligns kit loudness without altering the WAVs.
//!
//! Licenses and sources are recorded in `assets/audio/ATTRIBUTION.md`.

use std::sync::OnceLock;

pub struct Sample {
    pub data: Vec<f32>,
    pub rate: f32,
    pub root: u8,
}

pub const KIT_NAMES: [&str; 13] = [
    "Acoustic Studio",
    "House Club",
    "Trap 808",
    "Dubstep",
    "Boom Bap",
    "Bounce",
    "Salamander Studio",
    "Salamander Hard",
    "Salamander Soft",
    "World Percussion",
    "Orchestral Percussion",
    "Mallet Lab",
    "TR-808 Studio",
];

/// Gain-only balance trims, in dB, measured through the Drum Rack at 48 kHz
/// with a 0.9-velocity whole-kit probe. They align each kit's median 50 ms RMS
/// to -14.8 dBFS; the source WAVs, EQ, compression and triggering are unchanged.
pub const KIT_TRIM_DB: [f32; 13] = [
    0.9, 0.9, 0.0, -0.3, 2.1, 0.1, -0.7, -1.3, 2.9, 2.7, 0.9, -2.2, -1.2,
];

/// Column order: kick, snare, clap, closed hat, open hat, low tom, crash,
/// rim, 808, percussion, shaker, ride, cowbell, snap, floor tom, high tom.
const KIT_SAMPLES: [[u8; 16]; 13] = [
    [0, 1, 2, 4, 5, 6, 9, 2, 0, 7, 11, 10, 3, 2, 8, 7], // acoustic (Perc = splash)
    [0, 1, 27, 4, 5, 6, 9, 2, 33, 7, 4, 10, 28, 2, 8, 7],
    [
        12, 13, 15, 16, 17, 18, 19, 20, 21, 22, 16, 19, 22, 14, 18, 18,
    ],
    [
        12, 13, 15, 16, 17, 18, 19, 20, 23, 22, 16, 19, 22, 14, 18, 18,
    ],
    [
        34, 35, 36, 37, 38, 39, 40, 35, 41, 42, 37, 40, 36, 36, 39, 39,
    ],
    [
        24, 25, 27, 28, 29, 30, 32, 26, 33, 31, 28, 32, 26, 26, 30, 31,
    ],
    [
        43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58,
    ], // salamander-studio
    [
        59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74,
    ], // salamander-hard
    [
        75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90,
    ], // salamander-soft
    [
        91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106,
    ], // world-percussion
    [
        107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122,
    ], // orchestral-percussion
    [
        123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138,
    ], // mallet-lab
    [
        139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154,
    ], // tr-808
];

const DRUM_FILES: &[&[u8]] = &[
    include_bytes!("../assets/audio/drums/acoustic/kick-01.wav"),
    include_bytes!("../assets/audio/drums/acoustic/snare-01.wav"),
    include_bytes!("../assets/audio/drums/acoustic/snare-02.wav"),
    include_bytes!("../assets/audio/drums/acoustic/snare-03.wav"),
    include_bytes!("../assets/audio/drums/acoustic/hihat-closed.wav"),
    include_bytes!("../assets/audio/drums/acoustic/hihat-open.wav"),
    include_bytes!("../assets/audio/drums/acoustic/tom-01.wav"),
    include_bytes!("../assets/audio/drums/acoustic/tom-02.wav"),
    include_bytes!("../assets/audio/drums/acoustic/tom-03.wav"),
    include_bytes!("../assets/audio/drums/acoustic/crash-01.wav"),
    include_bytes!("../assets/audio/drums/acoustic/ride-01.wav"),
    include_bytes!("../assets/audio/drums/acoustic/splash-01.wav"),
    include_bytes!("../assets/audio/drums/trap/kick.wav"),
    include_bytes!("../assets/audio/drums/trap/snare.wav"),
    include_bytes!("../assets/audio/drums/trap/snare2.wav"),
    include_bytes!("../assets/audio/drums/trap/clap.wav"),
    include_bytes!("../assets/audio/drums/trap/hat.wav"),
    include_bytes!("../assets/audio/drums/trap/open.wav"),
    include_bytes!("../assets/audio/drums/trap/tom.wav"),
    include_bytes!("../assets/audio/drums/trap/crash.wav"),
    include_bytes!("../assets/audio/drums/trap/rim.wav"),
    include_bytes!("../assets/audio/drums/trap/808.wav"),
    include_bytes!("../assets/audio/drums/trap/cowbell.wav"),
    include_bytes!("../assets/audio/drums/trap/808dist.wav"),
    include_bytes!("../assets/audio/drums/bounce/kick.wav"),
    include_bytes!("../assets/audio/drums/bounce/snare.wav"),
    include_bytes!("../assets/audio/drums/bounce/snare2.wav"),
    include_bytes!("../assets/audio/drums/bounce/clap.wav"),
    include_bytes!("../assets/audio/drums/bounce/hat.wav"),
    include_bytes!("../assets/audio/drums/bounce/open.wav"),
    include_bytes!("../assets/audio/drums/bounce/tom.wav"),
    include_bytes!("../assets/audio/drums/bounce/tomhi.wav"),
    include_bytes!("../assets/audio/drums/bounce/crash.wav"),
    include_bytes!("../assets/audio/drums/bounce/808.wav"),
    include_bytes!("../assets/audio/drums/vintage/kick.wav"),
    include_bytes!("../assets/audio/drums/vintage/snare.wav"),
    include_bytes!("../assets/audio/drums/vintage/clap.wav"),
    include_bytes!("../assets/audio/drums/vintage/hat.wav"),
    include_bytes!("../assets/audio/drums/vintage/open.wav"),
    include_bytes!("../assets/audio/drums/vintage/tom.wav"),
    include_bytes!("../assets/audio/drums/vintage/crash.wav"),
    include_bytes!("../assets/audio/drums/vintage/808.wav"),
    include_bytes!("../assets/audio/drums/vintage/shaker.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/kick.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/snare.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/clap.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/hat-closed.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/hat-open.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/tom-low.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/crash.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/rim.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/808.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/percussion.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/shaker.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/ride.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/cowbell.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/snap.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/tom-floor.wav"),
    include_bytes!("../assets/audio/drums/salamander-studio/tom-high.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/kick.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/snare.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/clap.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/hat-closed.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/hat-open.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/tom-low.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/crash.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/rim.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/808.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/percussion.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/shaker.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/ride.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/cowbell.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/snap.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/tom-floor.wav"),
    include_bytes!("../assets/audio/drums/salamander-hard/tom-high.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/kick.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/snare.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/clap.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/hat-closed.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/hat-open.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/tom-low.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/crash.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/rim.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/808.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/percussion.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/shaker.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/ride.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/cowbell.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/snap.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/tom-floor.wav"),
    include_bytes!("../assets/audio/drums/salamander-soft/tom-high.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/kick.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/snare.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/clap.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/hat-closed.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/hat-open.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/tom-low.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/crash.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/rim.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/808.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/percussion.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/shaker.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/ride.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/cowbell.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/snap.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/tom-floor.wav"),
    include_bytes!("../assets/audio/drums/world-percussion/tom-high.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/kick.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/snare.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/clap.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/hat-closed.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/hat-open.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/tom-low.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/crash.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/rim.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/808.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/percussion.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/shaker.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/ride.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/cowbell.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/snap.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/tom-floor.wav"),
    include_bytes!("../assets/audio/drums/orchestral-percussion/tom-high.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/marimba-c2.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/marimba-f1.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/marimba-g2.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/marimba-b2.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/marimba-c4.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/marimba-f3.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/marimba-g4.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/marimba-b4.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/marimba-c6.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/glock-g4.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/glock-c5.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/glock-g6.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/xylo-g3.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/xylo-c4.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/bell-c4.wav"),
    include_bytes!("../assets/audio/drums/mallet-lab/bell-g4.wav"),
    include_bytes!("../assets/audio/drums/tr-808/kick.wav"),
    include_bytes!("../assets/audio/drums/tr-808/snare.wav"),
    include_bytes!("../assets/audio/drums/tr-808/clap.wav"),
    include_bytes!("../assets/audio/drums/tr-808/hat-closed.wav"),
    include_bytes!("../assets/audio/drums/tr-808/hat-open.wav"),
    include_bytes!("../assets/audio/drums/tr-808/tom-low.wav"),
    include_bytes!("../assets/audio/drums/tr-808/crash.wav"),
    include_bytes!("../assets/audio/drums/tr-808/rim.wav"),
    include_bytes!("../assets/audio/drums/tr-808/808.wav"),
    include_bytes!("../assets/audio/drums/tr-808/percussion.wav"),
    include_bytes!("../assets/audio/drums/tr-808/shaker.wav"),
    include_bytes!("../assets/audio/drums/tr-808/ride.wav"),
    include_bytes!("../assets/audio/drums/tr-808/cowbell.wav"),
    include_bytes!("../assets/audio/drums/tr-808/snap.wav"),
    include_bytes!("../assets/audio/drums/tr-808/tom-floor.wav"),
    include_bytes!("../assets/audio/drums/tr-808/tom-high.wav"),
];

const DEFAULT_LANE_LABELS: [&str; 16] = [
    "Kick",
    "Snare",
    "Clap",
    "Hat",
    "Open Hat",
    "Low Tom",
    "Crash",
    "Rim",
    "Sub 808",
    "Perc",
    "Shaker",
    "Ride",
    "Cowbell",
    "Snap",
    "Floor Tom",
    "High Tom",
];

const SALAMANDER_LANE_LABELS: [&str; 16] = [
    "Kick",
    "Snare",
    "Stick",
    "Closed Hat",
    "Open Hat",
    "Low Tom",
    "Crash",
    "Off Snare",
    "Kick 2",
    "Splash",
    "Foot Hat",
    "Ride",
    "Cowbell",
    "Snare 2",
    "Floor Tom",
    "High Tom",
];

const WORLD_LANE_LABELS: [&str; 16] = [
    "Cajon",
    "Bongo",
    "Hand Clap",
    "Egg Shaker",
    "Tambourine",
    "Low Conga",
    "Darbuka Pa",
    "Claves",
    "Cajon Bass",
    "Conga",
    "Maracas",
    "Castanets",
    "Darbuka Tak",
    "Clap 2",
    "Tumba",
    "Bongo High",
];

const ORCHESTRAL_LANE_LABELS: [&str; 16] = [
    "Bass Drum",
    "Snare",
    "Rimshot",
    "Sleigh Bells",
    "Susp Cym",
    "Tenor Low",
    "Crash",
    "Claves",
    "Muted BD",
    "Low Conga",
    "Shaker",
    "Cym Ride",
    "Cowbell",
    "Snare Off",
    "Tenor Floor",
    "Tenor High",
];

const MALLET_LANE_LABELS: [&str; 16] = [
    "Marimba C2",
    "Marimba F1",
    "Marimba G2",
    "Marimba B2",
    "Marimba C4",
    "Marimba F3",
    "Marimba G4",
    "Marimba B4",
    "Marimba C6",
    "Glock G4",
    "Glock C5",
    "Glock G6",
    "Xylo G3",
    "Xylo C4",
    "Bell C4",
    "Bell G4",
];

const TR808_LANE_LABELS: [&str; 16] = [
    "Kick",
    "Snare",
    "Clap",
    "Closed Hat",
    "Open Hat",
    "Low Tom",
    "Cymbal",
    "Rim",
    "Long 808",
    "Claves",
    "Maracas",
    "Ride",
    "Cowbell",
    "Snare Long",
    "Mid Tom",
    "High Tom",
];

/// Pad labels follow the actual instrument recorded in each lane. The note
/// numbers are stable, so changing a kit never moves a lane on the pad map.
pub fn drum_lane_label(kit: usize, lane: usize) -> &'static str {
    let lane = lane.min(15);
    match kit {
        6..=8 => SALAMANDER_LANE_LABELS[lane],
        9 => WORLD_LANE_LABELS[lane],
        10 => ORCHESTRAL_LANE_LABELS[lane],
        11 => MALLET_LANE_LABELS[lane],
        12 => TR808_LANE_LABELS[lane],
        _ => DEFAULT_LANE_LABELS[lane],
    }
}

/// A piano sample every six semitones, two velocity layers. The player
/// pitch-shifts between roots, so the whole range shares a consistent timbre.
pub const PIANO_ROOTS: [u8; 13] = [33, 39, 45, 51, 57, 63, 69, 75, 81, 87, 93, 99, 105];

const PIANO_SOFT: &[&[u8]] = &[
    include_bytes!("../assets/audio/piano/piano_33_soft.wav"),
    include_bytes!("../assets/audio/piano/piano_39_soft.wav"),
    include_bytes!("../assets/audio/piano/piano_45_soft.wav"),
    include_bytes!("../assets/audio/piano/piano_51_soft.wav"),
    include_bytes!("../assets/audio/piano/piano_57_soft.wav"),
    include_bytes!("../assets/audio/piano/piano_63_soft.wav"),
    include_bytes!("../assets/audio/piano/piano_69_soft.wav"),
    include_bytes!("../assets/audio/piano/piano_75_soft.wav"),
    include_bytes!("../assets/audio/piano/piano_81_soft.wav"),
    include_bytes!("../assets/audio/piano/piano_87_soft.wav"),
    include_bytes!("../assets/audio/piano/piano_93_soft.wav"),
    include_bytes!("../assets/audio/piano/piano_99_soft.wav"),
    include_bytes!("../assets/audio/piano/piano_105_soft.wav"),
];

const PIANO_HARD: &[&[u8]] = &[
    include_bytes!("../assets/audio/piano/piano_33_hard.wav"),
    include_bytes!("../assets/audio/piano/piano_39_hard.wav"),
    include_bytes!("../assets/audio/piano/piano_45_hard.wav"),
    include_bytes!("../assets/audio/piano/piano_51_hard.wav"),
    include_bytes!("../assets/audio/piano/piano_57_hard.wav"),
    include_bytes!("../assets/audio/piano/piano_63_hard.wav"),
    include_bytes!("../assets/audio/piano/piano_69_hard.wav"),
    include_bytes!("../assets/audio/piano/piano_75_hard.wav"),
    include_bytes!("../assets/audio/piano/piano_81_hard.wav"),
    include_bytes!("../assets/audio/piano/piano_87_hard.wav"),
    include_bytes!("../assets/audio/piano/piano_93_hard.wav"),
    include_bytes!("../assets/audio/piano/piano_99_hard.wav"),
    include_bytes!("../assets/audio/piano/piano_105_hard.wav"),
];

struct Bank {
    drums: Vec<Option<Sample>>,
    piano_soft: Vec<Option<Sample>>,
    piano_hard: Vec<Option<Sample>>,
}

static BANK: OnceLock<Bank> = OnceLock::new();

fn remove_dc(sample: &mut Sample) {
    if sample.data.is_empty() {
        return;
    }
    let mean = sample.data.iter().map(|v| *v as f64).sum::<f64>() / sample.data.len() as f64;
    if mean.abs() <= f64::EPSILON {
        return;
    }
    let mean = mean as f32;
    for value in &mut sample.data {
        *value -= mean;
    }
}

fn bank() -> &'static Bank {
    BANK.get_or_init(|| Bank {
        // Some of the recorded one-shots carry a small DC bias from the source
        // recording. Remove that bias once, at decode time, rather than letting
        // each Drum Rack voice or the master bus introduce a corrective
        // high-pass. This keeps the samples' transients and musical low end.
        drums: DRUM_FILES
            .iter()
            .map(|b| {
                decode_wav(b).map(|mut sample| {
                    remove_dc(&mut sample);
                    sample
                })
            })
            .collect(),
        piano_soft: PIANO_SOFT
            .iter()
            .enumerate()
            .map(|(i, b)| {
                decode_wav_with_root(b, PIANO_ROOTS[i]).map(|mut sample| {
                    remove_dc(&mut sample);
                    sample
                })
            })
            .collect(),
        piano_hard: PIANO_HARD
            .iter()
            .enumerate()
            .map(|(i, b)| {
                decode_wav_with_root(b, PIANO_ROOTS[i]).map(|mut sample| {
                    remove_dc(&mut sample);
                    sample
                })
            })
            .collect(),
    })
}

/// Decode once before audio starts, so no WAV parsing happens on the callback.
pub fn preload() {
    let _ = bank();
}

pub fn drum_kit_count() -> usize {
    KIT_SAMPLES.len()
}

pub fn drum_sample_count() -> usize {
    DRUM_FILES.len()
}

pub fn drum_kit_trim_db(kit: usize) -> f32 {
    KIT_TRIM_DB[kit.min(KIT_TRIM_DB.len() - 1)]
}

pub fn drum_kit_gain(kit: usize) -> f32 {
    10.0f32.powf(drum_kit_trim_db(kit) / 20.0)
}

pub fn drum(kit: usize, lane: usize) -> Option<&'static Sample> {
    let map = KIT_SAMPLES.get(kit.min(KIT_SAMPLES.len() - 1))?;
    let idx = *map.get(lane.min(map.len() - 1))? as usize;
    bank().drums.get(idx).and_then(|s| s.as_ref())
}

/// Index into the embedded drum-file bank for a kit lane. This is exposed for
/// coverage/measurement tests so every embedded one-shot can be accounted for.
pub fn drum_sample_index(kit: usize, lane: usize) -> Option<usize> {
    let map = KIT_SAMPLES.get(kit.min(KIT_SAMPLES.len() - 1))?;
    map.get(lane.min(map.len() - 1)).map(|v| *v as usize)
}

/// Returns the closest recorded piano note and whether the hard layer was used.
pub fn piano(note: u8, hard: bool) -> Option<&'static Sample> {
    let mut best = 0usize;
    let mut best_dist = u16::MAX;
    for (i, root) in PIANO_ROOTS.iter().enumerate() {
        let d = (*root as i16 - note as i16).unsigned_abs();
        if d < best_dist {
            best_dist = d;
            best = i;
        }
    }
    let list = if hard {
        &bank().piano_hard
    } else {
        &bank().piano_soft
    };
    list.get(best).and_then(|s| s.as_ref())
}

fn decode_wav_with_root(bytes: &'static [u8], root: u8) -> Option<Sample> {
    let mut s = decode_wav(bytes)?;
    s.root = root;
    Some(s)
}

/// Minimal RIFF/WAVE reader: PCM 8/16/24/32-bit integer or 32-bit float.
/// The embedded files are mono, but stereo files are mixed down for safety.
pub fn decode_wav(bytes: &[u8]) -> Option<Sample> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut pos = 12usize;
    let mut channels = 1usize;
    let mut rate = 44100u32;
    let mut bits = 16u16;
    let mut format = 1u16;
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().ok()?) as usize;
        let start = pos + 8;
        let end = start.saturating_add(size).min(bytes.len());
        if id == b"fmt " && size >= 16 {
            format = u16::from_le_bytes(bytes[start..start + 2].try_into().ok()?);
            channels = u16::from_le_bytes(bytes[start + 2..start + 4].try_into().ok()?) as usize;
            rate = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().ok()?);
            bits = u16::from_le_bytes(bytes[start + 14..start + 16].try_into().ok()?);
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
            break;
        }
        pos = start + size + (size & 1);
    }
    let data = data?;
    if channels == 0 || channels > 8 {
        return None;
    }
    let bytes_per_sample = (bits as usize / 8).max(1);
    let frame_bytes = bytes_per_sample.saturating_mul(channels);
    if frame_bytes == 0 || data.len() < frame_bytes {
        return None;
    }
    let frames = data.len() / frame_bytes;
    let mut out = Vec::with_capacity(frames);
    for frame in data.chunks_exact(frame_bytes) {
        let mut sum = 0.0f32;
        for ch in 0..channels {
            let b = &frame[ch * bytes_per_sample..(ch + 1) * bytes_per_sample];
            let v = match (format, bits) {
                (3, 32) => f32::from_bits(u32::from_le_bytes(b.try_into().ok()?)),
                (1, 8) => (b[0] as f32 - 128.0) / 128.0,
                (1, 16) => i16::from_le_bytes(b.try_into().ok()?) as f32 / 32768.0,
                (1, 24) => {
                    let raw = (b[0] as i32) | ((b[1] as i32) << 8) | ((b[2] as i32) << 16);
                    let signed = (raw << 8) >> 8;
                    signed as f32 / 8_388_608.0
                }
                (1, 32) => i32::from_le_bytes(b.try_into().ok()?) as f32 / 2_147_483_648.0,
                _ => b[0] as f32 / 128.0,
            };
            sum += v;
        }
        out.push(sum / channels as f32);
    }
    Some(Sample {
        data: out,
        rate: rate.max(1) as f32,
        root: 60,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kit_lane_resolves_to_a_decodable_sample() {
        preload();
        for kit in 0..KIT_SAMPLES.len() {
            for lane in 0..16 {
                let s = drum(kit, lane).unwrap_or_else(|| panic!("kit {kit} lane {lane} missing"));
                assert!(s.data.len() > 100);
                assert!(s.data.iter().all(|x| x.is_finite()));
            }
        }
    }

    #[test]
    fn decoded_one_shots_have_no_baked_in_dc_bias() {
        preload();
        for kit in 0..KIT_SAMPLES.len() {
            for lane in 0..16 {
                let s = drum(kit, lane).unwrap();
                let mean = s.data.iter().map(|v| *v as f64).sum::<f64>() / s.data.len() as f64;
                assert!(
                    mean.abs() < 0.0001,
                    "kit {kit} lane {lane} sample has DC bias {mean:+.7}"
                );
            }
        }
        for root in PIANO_ROOTS {
            for hard in [false, true] {
                let s = piano(root, hard).unwrap();
                let mean = s.data.iter().map(|v| *v as f64).sum::<f64>() / s.data.len() as f64;
                assert!(
                    mean.abs() < 0.0001,
                    "piano root {root} hard={hard} sample has DC bias {mean:+.7}"
                );
            }
        }
    }

    #[test]
    fn salamander_has_both_velocity_layers_across_the_keyboard() {
        preload();
        for root in PIANO_ROOTS {
            assert!(piano(root, false).is_some());
            assert!(piano(root, true).is_some());
        }
    }
    #[test]
    fn expanded_bank_has_thirteen_kits_and_155_files() {
        preload();
        assert_eq!(KIT_NAMES.len(), 13);
        assert_eq!(KIT_SAMPLES.len(), 13);
        assert_eq!(DRUM_FILES.len(), 155);
        assert_eq!(drum_kit_count(), 13);
        assert_eq!(KIT_SAMPLES[6][0], 43);
        assert_eq!(KIT_SAMPLES[12][15], 154);
        for i in 0..DRUM_FILES.len() {
            assert!(
                KIT_SAMPLES.iter().flatten().any(|v| *v as usize == i),
                "drum file {i} is not reachable from any kit"
            );
        }
        let reachable: std::collections::HashSet<_> =
            KIT_SAMPLES.iter().flatten().map(|v| *v as usize).collect();
        assert_eq!(reachable.len(), DRUM_FILES.len());
    }

    #[test]
    fn kit_gain_trims_are_finite_and_bounded() {
        assert_eq!(KIT_TRIM_DB.len(), KIT_NAMES.len());
        for (kit, trim) in KIT_TRIM_DB.iter().enumerate() {
            assert!(trim.is_finite(), "kit {kit} trim is not finite");
            assert!(trim.abs() <= 6.0, "kit {kit} trim is excessive: {trim}");
            assert!(drum_kit_gain(kit).is_finite());
        }
    }

    #[test]
    fn every_kit_has_sixteen_named_lanes() {
        for kit in 0..KIT_NAMES.len() {
            for lane in 0..16 {
                let label = drum_lane_label(kit, lane);
                assert!(!label.is_empty(), "kit {kit} lane {lane} has no label");
                assert!(
                    drum(kit, lane).is_some(),
                    "kit {kit} lane {lane} has no sample"
                );
            }
        }
    }
}
