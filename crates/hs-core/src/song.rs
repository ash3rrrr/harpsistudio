//! Song model: tracks, clips, notes and master section.

use crate::audio_take::AudioTake;
use crate::params::{Device, DeviceKind, Preset, TRACK_COLORS};
use serde::{Deserialize, Serialize};

pub const PPQ: f64 = 4.0; // beats per bar (4/4)

/// Global launch quantization choices, in beats. `0.0` means launch instantly.
pub const LAUNCH_QUANTA: [(f64, &str); 8] = [
    (0.0, "OFF"),
    (0.25, "1/16"),
    (0.5, "1/8"),
    (1.0, "1/4"),
    (2.0, "1/2"),
    (4.0, "1 BAR"),
    (8.0, "2 BARS"),
    (16.0, "4 BARS"),
];

/// Per-clip follow-action values. These are stored as integers so old .harp
/// projects without the field keep loading with a sensible default.
pub const FOLLOW_NONE: i32 = 0;
pub const FOLLOW_NEXT: i32 = 1;
pub const FOLLOW_PREV: i32 = 2;
pub const FOLLOW_FIRST: i32 = 3;
pub const FOLLOW_STOP: i32 = 4;
pub const FOLLOW_ACTIONS: [&str; 5] = ["NONE", "NEXT", "PREV", "FIRST", "STOP"];
pub const FOLLOW_AFTER_BARS: [f64; 5] = [1.0, 2.0, 4.0, 8.0, 16.0];

fn default_follow_after() -> f64 {
    4.0
}

fn default_launch_quantum() -> f64 {
    4.0
}

pub fn launch_quantum_label(quantum: f64) -> &'static str {
    LAUNCH_QUANTA
        .iter()
        .find(|(q, _)| (*q - quantum).abs() < 0.001)
        .map(|(_, label)| *label)
        .unwrap_or("1 BAR")
}

pub fn cycle_launch_quantum(quantum: f64) -> f64 {
    let idx = LAUNCH_QUANTA
        .iter()
        .position(|(q, _)| (*q - quantum).abs() < 0.001)
        .unwrap_or(5);
    LAUNCH_QUANTA[(idx + 1) % LAUNCH_QUANTA.len()].0
}

/// First grid boundary strictly after `beat`. A zero quantum is immediate.
pub fn next_quantized_beat(beat: f64, quantum: f64) -> f64 {
    if quantum <= 0.0 {
        beat.max(0.0)
    } else {
        ((beat.max(0.0) / quantum).floor() + 1.0) * quantum
    }
}

pub fn follow_action_label(action: i32) -> &'static str {
    FOLLOW_ACTIONS
        .get(action.clamp(0, 4) as usize)
        .copied()
        .unwrap_or("NONE")
}

pub fn next_follow_action(action: i32) -> i32 {
    (action.clamp(0, 4) + 1) % FOLLOW_ACTIONS.len() as i32
}

pub fn next_follow_after(bars: f64) -> f64 {
    let idx = FOLLOW_AFTER_BARS
        .iter()
        .position(|b| (*b - bars).abs() < 0.001)
        .unwrap_or(2);
    FOLLOW_AFTER_BARS[(idx + 1) % FOLLOW_AFTER_BARS.len()]
}

pub fn follow_after_label(bars: f64) -> String {
    if (bars - 1.0).abs() < 0.001 {
        "1 BAR".to_string()
    } else {
        format!("{:.0} BARS", bars.round())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub start: f64,
    pub len: f64,
    pub pitch: u8,
    pub vel: f32,
}

impl Note {
    pub fn new(start: f64, len: f64, pitch: u8, vel: f32) -> Self {
        Self {
            start,
            len,
            pitch,
            vel,
        }
    }
    pub fn end(&self) -> f64 {
        self.start + self.len
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub name: String,
    /// >= 0 is a session clip in that scene row; -1 is an arrangement clip.
    pub scene: i32,
    pub start: f64,
    pub length: f64,
    pub looping: bool,
    /// Session-clip follow action (see the FOLLOW_* constants).
    #[serde(default)]
    pub follow_action: i32,
    /// How long the clip plays before the follow action fires, in bars.
    #[serde(default = "default_follow_after")]
    pub follow_after: f64,
    pub notes: Vec<Note>,
}

impl Clip {
    pub fn new(name: &str, scene: i32, start: f64, length: f64) -> Self {
        Self {
            name: name.to_string(),
            scene,
            start,
            length,
            looping: true,
            follow_action: FOLLOW_NONE,
            follow_after: default_follow_after(),
            notes: Vec::new(),
        }
    }

    pub fn follow_label(&self) -> &'static str {
        follow_action_label(self.follow_action)
    }

    pub fn end(&self) -> f64 {
        self.start + self.length
    }

    /// Sorted copy of notes, used by the piano roll and clip previews.
    pub fn sorted_notes(&self) -> Vec<Note> {
        let mut n = self.notes.clone();
        n.sort_by(|a, b| {
            a.start
                .partial_cmp(&b.start)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        n
    }

    pub fn note_bounds(&self) -> (u8, u8) {
        let mut lo = 127u8;
        let mut hi = 0u8;
        for n in &self.notes {
            lo = lo.min(n.pitch);
            hi = hi.max(n.pitch);
        }
        if self.notes.is_empty() {
            (48, 72)
        } else {
            (lo.saturating_sub(3), (hi + 4).min(127))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    pub color: [u8; 3],
    pub instrument: Device,
    /// A real vocal/audio take lives with the track, so projects stay portable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub take: Option<AudioTake>,
    pub effects: Vec<Device>,
    /// Note-generating devices (arpeggiator, chord generator). They run before
    /// the instrument and are kept separate from the audio effect chain.
    #[serde(default)]
    pub midi_fx: Vec<Device>,
    pub volume: f32,
    pub pan: f32,
    pub send: f32,
    pub mute: bool,
    pub solo: bool,
    pub clips: Vec<Clip>,
}

impl Track {
    pub fn new(kind: DeviceKind, index: usize) -> Self {
        Self {
            name: kind.name().to_string(),
            color: TRACK_COLORS[index % TRACK_COLORS.len()],
            instrument: Device::new(kind),
            take: None,
            effects: Vec::new(),
            midi_fx: Vec::new(),
            volume: 0.8,
            pan: 0.0,
            send: 0.12,
            mute: false,
            solo: false,
            clips: Vec::new(),
        }
    }

    pub fn from_preset(kind: DeviceKind, preset: &Preset, index: usize) -> Self {
        let mut t = Track::new(kind, index);
        t.instrument.load(preset);
        t.name = format!("{} {}", kind.name(), preset.name);
        t
    }

    pub fn add_effect(&mut self, kind: DeviceKind) {
        self.effects.push(Device::new(kind));
    }

    pub fn session_clip(&self, scene: i32) -> Option<&Clip> {
        self.clips.iter().find(|c| c.scene == scene)
    }

    pub fn session_clip_mut(&mut self, scene: i32) -> Option<&mut Clip> {
        self.clips.iter_mut().find(|c| c.scene == scene)
    }

    pub fn arrangement_clips(&self) -> impl Iterator<Item = &Clip> {
        self.clips.iter().filter(|c| c.scene < 0)
    }

    pub fn ensure_session_clip(&mut self, scene: i32, len: f64) -> &mut Clip {
        if self.session_clip(scene).is_none() {
            self.clips
                .push(Clip::new(&self.name.clone(), scene, 0.0, len));
        }
        self.session_clip_mut(scene).unwrap()
    }

    pub fn remove_session_clip(&mut self, scene: i32) {
        self.clips.retain(|c| c.scene != scene);
    }

    pub fn peak_end(&self) -> f64 {
        self.clips.iter().map(|c| c.end()).fold(0.0, f64::max)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub name: String,
    pub bpm: f64,
    pub master_volume: f32,
    pub limiter: bool,
    /// Piano-roll scale helper, saved with the project so melodies keep their
    /// key and snap setting when the song is reopened.
    #[serde(default)]
    pub scale_root: i32,
    #[serde(default)]
    pub scale_mode: i32,
    #[serde(default)]
    pub scale_snap: bool,
    /// Global launch quantization for session clips, in beats.
    #[serde(default = "default_launch_quantum")]
    pub launch_quantum: f64,
    pub tracks: Vec<Track>,
    pub master_fx: Vec<Device>,
    pub return_fx: Device,
    pub scenes: Vec<String>,
    pub loop_enabled: bool,
    pub loop_start: f64,
    pub loop_end: f64,
}

impl Song {
    pub fn empty(name: &str, bpm: f64) -> Self {
        let mut return_fx = Device::new(DeviceKind::Reverb);
        return_fx.set(3, 1.0);
        Self {
            name: name.to_string(),
            bpm,
            master_volume: 0.85,
            limiter: true,
            scale_root: 0,
            scale_mode: 0,
            scale_snap: false,
            launch_quantum: default_launch_quantum(),
            tracks: Vec::new(),
            master_fx: vec![Device::new(DeviceKind::Compressor)],
            return_fx,
            scenes: (0..8).map(|i| format!("Scene {}", i + 1)).collect(),
            loop_enabled: false,
            loop_start: 0.0,
            loop_end: 32.0,
        }
    }

    pub fn add_track(&mut self, kind: DeviceKind) -> usize {
        let idx = self.tracks.len();
        self.tracks.push(Track::new(kind, idx));
        idx
    }

    pub fn add_track_preset(&mut self, kind: DeviceKind, preset: &Preset) -> usize {
        let idx = self.tracks.len();
        self.tracks.push(Track::from_preset(kind, preset, idx));
        idx
    }

    pub fn any_solo(&self) -> bool {
        self.tracks.iter().any(|t| t.solo)
    }

    pub fn ensure_scenes(&mut self, n: usize) {
        while self.scenes.len() < n {
            self.scenes.push(format!("Scene {}", self.scenes.len() + 1));
        }
    }

    pub fn scene_names(&self) -> Vec<String> {
        let mut s = self.scenes.clone();
        if s.is_empty() {
            s.push("Scene 1".to_string());
        }
        s
    }

    /// Last used beat in the arrangement.
    pub fn length_beats(&self) -> f64 {
        let mut end: f64 = 0.0;
        for t in &self.tracks {
            for c in t.clips.iter().filter(|c| c.scene < 0) {
                end = end.max(c.end());
            }
        }
        if end <= 0.0 {
            16.0
        } else {
            end
        }
    }

    pub fn all_session_clips(&self) -> Vec<&Clip> {
        self.tracks
            .iter()
            .flat_map(|t| t.clips.iter().filter(|c| c.scene >= 0))
            .collect()
    }

    pub fn clip(&self, track: usize, scene: i32) -> Option<&Clip> {
        self.tracks.get(track)?.session_clip(scene)
    }

    pub fn bar_of(beat: f64) -> f64 {
        beat / PPQ
    }

    /// Clean up clips so no session clip is assigned to a missing scene row.
    pub fn normalize(&mut self) {
        self.scale_root = self.scale_root.rem_euclid(12);
        self.scale_mode = self.scale_mode.clamp(0, 4);
        self.launch_quantum = LAUNCH_QUANTA
            .iter()
            .map(|(q, _)| *q)
            .min_by(|a, b| {
                (a - self.launch_quantum)
                    .abs()
                    .partial_cmp(&(b - self.launch_quantum).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(4.0);
        let scenes = self.scenes.len() as i32;
        for t in &mut self.tracks {
            t.clips.retain(|c| c.scene < 0 || c.scene < scenes);
            for c in &mut t.clips {
                if c.length < 0.25 {
                    c.length = 0.25;
                }
                c.follow_action = c.follow_action.clamp(FOLLOW_NONE, FOLLOW_STOP);
                if c.follow_after <= 0.0 {
                    c.follow_after = default_follow_after();
                }
                c.follow_after = c.follow_after.clamp(1.0, 16.0);
                for n in &mut c.notes {
                    n.pitch = n.pitch.min(127);
                    n.len = n.len.max(0.0625);
                    n.vel = n.vel.clamp(0.05, 1.0);
                }
            }
        }
    }
}

pub fn format_position(beat: f64) -> String {
    let b = beat.max(0.0);
    let bar = (b / PPQ).floor() as i64 + 1;
    let beat_in_bar = (b % PPQ).floor() as i64 + 1;
    let sixteenth = ((b % 1.0) * 4.0).floor() as i64 + 1;
    format!("{} . {} . {}", bar, beat_in_bar, sixteenth)
}
