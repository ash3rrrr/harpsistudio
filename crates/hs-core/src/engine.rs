//! Real-time engine: sequencer, voice allocation, track device chains,
//! send bus, master chain, limiter and metering.

use crate::devices::{process_effect, render_instrument, FxState, InstrRt, Voice};
use crate::dsp::{highpass, pan_gains, MAX_VOICES};
use crate::midi_fx::{chain_fingerprint, transform_notes};
use crate::params::{Device, DeviceKind};
use crate::song::{
    next_quantized_beat, Clip, Note, Song, FOLLOW_FIRST, FOLLOW_NEXT, FOLLOW_NONE, FOLLOW_PREV,
    FOLLOW_STOP, PPQ,
};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub const MAX_TRACKS: usize = 24;
const NO_SCENE: i32 = i32::MIN;

#[derive(Clone, Debug)]
pub enum Cmd {
    Launch {
        track: usize,
        scene: i32,
    },
    StopTrack(usize),
    StopClips,
    Play,
    Stop,
    Toggle,
    Seek(f64),
    NoteOn {
        track: usize,
        note: u8,
        vel: f32,
    },
    NoteOff {
        track: usize,
        note: u8,
    },
    Sustain(bool),
    AllNotesOff,
    Panic,
    /// Audition a pattern through the engine without touching the project.
    Preview {
        instrument: Device,
        notes: Vec<Note>,
        length: f64,
        bpm: f64,
        gain: f32,
    },
    StopPreview,
}

pub struct Shared {
    pub song: Mutex<Option<Arc<Song>>>,
    pub playing: AtomicBool,
    pub beat: AtomicU64,
    pub levels: Vec<AtomicU32>,
    /// Active and queued session clip per track, exposed for the clip grid.
    pub session_scene: Vec<AtomicI32>,
    pub pending_scene: Vec<AtomicI32>,
    pub cpu: AtomicU32,
    pub sample_rate: AtomicU32,
    pub live_track: AtomicUsize,
    pub cmd: Sender<Cmd>,
}

impl Shared {
    pub fn new(song: &Song) -> (Arc<Shared>, Arc<Song>, Receiver<Cmd>) {
        let (tx, rx) = channel();
        let arc = Arc::new(song.clone());
        let shared = Arc::new(Shared {
            song: Mutex::new(Some(arc.clone())),
            playing: AtomicBool::new(false),
            beat: AtomicU64::new(0.0f64.to_bits()),
            levels: (0..=MAX_TRACKS).map(|_| AtomicU32::new(0)).collect(),
            session_scene: (0..MAX_TRACKS).map(|_| AtomicI32::new(NO_SCENE)).collect(),
            pending_scene: (0..MAX_TRACKS).map(|_| AtomicI32::new(NO_SCENE)).collect(),
            cpu: AtomicU32::new(0.0f32.to_bits()),
            sample_rate: AtomicU32::new(48000),
            live_track: AtomicUsize::new(0),
            cmd: tx,
        });
        (shared, arc, rx)
    }

    pub fn publish(&self, song: &Song) {
        if let Ok(mut g) = self.song.lock() {
            *g = Some(Arc::new(song.clone()));
        }
    }

    pub fn send(&self, cmd: Cmd) {
        let _ = self.cmd.send(cmd);
    }

    pub fn set_live_track(&self, track: usize) {
        self.live_track.store(track, Ordering::Relaxed);
    }

    pub fn beat(&self) -> f64 {
        f64::from_bits(self.beat.load(Ordering::Relaxed))
    }

    pub fn level(&self, i: usize) -> f32 {
        self.levels
            .get(i)
            .map(|a| f32::from_bits(a.load(Ordering::Relaxed)))
            .unwrap_or(0.0)
    }

    pub fn cpu(&self) -> f32 {
        f32::from_bits(self.cpu.load(Ordering::Relaxed))
    }

    pub fn session_scene(&self, track: usize) -> Option<i32> {
        self.session_scene
            .get(track)
            .map(|a| a.load(Ordering::Relaxed))
            .filter(|scene| *scene != NO_SCENE)
    }

    pub fn pending_scene(&self, track: usize) -> Option<i32> {
        self.pending_scene
            .get(track)
            .map(|a| a.load(Ordering::Relaxed))
            .filter(|scene| *scene != NO_SCENE)
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }
}

struct TrackRt {
    voices: [Voice; MAX_VOICES],
    live: [Voice; MAX_VOICES],
    instr: InstrRt,
    instr_kind: Option<DeviceKind>,
    fx: Vec<FxState>,
    /// Notes after the track's MIDI-FX chain, one entry per clip. Rebuilt only
    /// when the chain or the project changes, never on the audio thread.
    midi_notes: Vec<Arc<[Note]>>,
    midi_sig: u64,
    session: Option<i32>,
    session_start: f64,
    pending: Option<i32>,
    pending_at: f64,
    vol: f32,
    peak: f32,
    dc_l: [f32; 2],
    dc_r: [f32; 2],
}

impl TrackRt {
    fn new() -> Self {
        Self {
            voices: std::array::from_fn(|_| Voice::new()),
            live: std::array::from_fn(|_| Voice::new()),
            instr: InstrRt::new(),
            instr_kind: None,
            fx: Vec::new(),
            midi_notes: Vec::new(),
            midi_sig: 0,
            session: None,
            session_start: 0.0,
            pending: None,
            pending_at: 0.0,
            vol: 0.8,
            peak: 0.0,
            dc_l: [0.0; 2],
            dc_r: [0.0; 2],
        }
    }

    fn release_all(&mut self) {
        for v in &mut self.voices {
            if v.active {
                v.release();
            }
        }
    }

    fn reset_voices(&mut self) {
        for v in self.voices.iter_mut().chain(self.live.iter_mut()) {
            v.active = false;
            v.sustained = false;
        }
    }

    fn voice_slot(voices: &[Voice; MAX_VOICES]) -> usize {
        let mut best = 0;
        let mut best_score = f32::MAX;
        for (i, v) in voices.iter().enumerate() {
            if !v.active {
                return i;
            }
            let score = v.amp / (1.0 + v.age);
            if score < best_score {
                best_score = score;
                best = i;
            }
        }
        best
    }

    fn trigger(&mut self, note: u8, vel: f32, end_beat: f64, seed: u32) {
        let idx = Self::voice_slot(&self.voices);
        self.voices[idx].note_on(note, vel, end_beat, seed);
    }

    fn trigger_live(&mut self, note: u8, vel: f32, seed: u32) {
        let idx = Self::voice_slot(&self.live);
        self.live[idx].note_on(note, vel, f64::INFINITY, seed);
    }

    fn release_live(&mut self, note: u8) {
        for v in &mut self.live {
            if v.active && v.note == note && v.gate {
                v.release();
            }
        }
    }

    fn sustain_live(&mut self, note: u8) {
        for v in &mut self.live {
            if v.active && v.note == note && v.gate {
                v.sustained = true;
            }
        }
    }

    fn release_sustained_live(&mut self) {
        for v in &mut self.live {
            if v.active && v.gate && v.sustained {
                v.release();
                v.sustained = false;
            }
        }
    }

    fn release_all_live(&mut self) {
        for v in &mut self.live {
            if v.active && v.gate {
                v.release();
            }
            v.sustained = false;
        }
    }
}

struct PreviewState {
    instrument: Device,
    instr: InstrRt,
    voices: [Voice; MAX_VOICES],
    notes: Vec<Note>,
    next: usize,
    offs: Vec<(f64, u8)>,
    pos: f64,
    length: f64,
    bpm: f64,
    gain: f32,
}

impl PreviewState {
    fn new(instrument: Device, notes: Vec<Note>, length: f64, bpm: f64, gain: f32) -> Self {
        let mut notes = notes;
        notes.sort_by(|a, b| {
            a.start
                .partial_cmp(&b.start)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.pitch.cmp(&b.pitch))
        });
        Self {
            instrument,
            instr: InstrRt::new(),
            voices: std::array::from_fn(|_| Voice::new()),
            notes,
            next: 0,
            offs: Vec::new(),
            pos: 0.0,
            length,
            bpm: bpm.max(30.0),
            gain: gain.clamp(0.05, 1.0),
        }
    }

    fn stop(&mut self) {
        for v in &mut self.voices {
            if v.active {
                v.release();
            }
        }
    }
}

/// Fingerprint of every clip's note data. The MIDI-note cache stores the
/// result of running this data through the MIDI-FX chain, so its key must
/// change whenever a note is added, removed, moved or retuned.
fn clip_notes_fingerprint(clips: &[Clip]) -> u64 {
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for clip in clips {
        h ^= clip.notes.len() as u64;
        h = h.wrapping_mul(FNV_PRIME);
        for note in &clip.notes {
            for bits in [
                note.start.to_bits(),
                note.len.to_bits(),
                note.vel.to_bits() as u64,
                note.pitch as u64,
            ] {
                h ^= bits;
                h = h.wrapping_mul(FNV_PRIME);
            }
        }
    }
    h
}

fn scene_action(track: &crate::song::Track, from: i32, action: i32) -> Option<i32> {
    let mut scenes: Vec<i32> = track
        .clips
        .iter()
        .filter(|c| c.scene >= 0)
        .map(|c| c.scene)
        .collect();
    scenes.sort_unstable();
    scenes.dedup();
    match action {
        FOLLOW_NEXT => scenes.into_iter().find(|s| *s > from),
        FOLLOW_PREV => scenes.into_iter().rev().find(|s| *s < from),
        FOLLOW_FIRST => scenes.first().copied(),
        _ => None,
    }
}

fn preview_slot(voices: &[Voice; MAX_VOICES]) -> usize {
    let mut best = 0;
    let mut best_score = f32::MAX;
    for (i, v) in voices.iter().enumerate() {
        if !v.active {
            return i;
        }
        let score = v.amp / (1.0 + v.age);
        if score < best_score {
            best_score = score;
            best = i;
        }
    }
    best
}

pub struct Engine {
    pub song: Arc<Song>,
    pub sr: f32,
    pub beat: f64,
    pub playing: bool,
    tracks: Vec<TrackRt>,
    master_fx: Vec<FxState>,
    send_fx: FxState,
    cmd_rx: Option<Receiver<Cmd>>,
    shared: Option<Arc<Shared>>,
    lim_gain: f32,
    pub peak_l: f32,
    pub peak_r: f32,
    seed: u32,
    cpu_smooth: f32,
    live_sustain: bool,
    preview: Option<PreviewState>,
    pub last_cpu: f32,
}

impl Engine {
    pub fn new(
        sr: f32,
        song: Arc<Song>,
        cmd_rx: Option<Receiver<Cmd>>,
        shared: Option<Arc<Shared>>,
    ) -> Self {
        let mut e = Self {
            song: song.clone(),
            sr,
            beat: 0.0,
            playing: false,
            tracks: Vec::new(),
            master_fx: Vec::new(),
            send_fx: FxState::new(DeviceKind::Reverb, sr),
            cmd_rx,
            shared,
            lim_gain: 1.0,
            peak_l: 0.0,
            peak_r: 0.0,
            seed: 0x1234_5678,
            cpu_smooth: 0.0,
            live_sustain: false,
            preview: None,
            last_cpu: 0.0,
        };
        e.sync_song(song);
        e
    }

    fn sync_song(&mut self, new: Arc<Song>) {
        let sr = self.sr;
        let n = new.tracks.len().min(MAX_TRACKS);
        self.tracks.resize_with(n, TrackRt::new);
        for i in 0..n {
            let t = &new.tracks[i];
            let rt = &mut self.tracks[i];
            if rt.instr_kind != Some(t.instrument.kind) {
                rt.instr_kind = Some(t.instrument.kind);
                rt.reset_voices();
            }
            let sig = chain_fingerprint(&t.midi_fx)
                ^ clip_notes_fingerprint(&t.clips).wrapping_mul(0x9e37_79b9_7f4a_7c15);
            if rt.midi_sig != sig || rt.midi_notes.len() != t.clips.len() {
                rt.midi_notes = t
                    .clips
                    .iter()
                    .map(|c| Arc::from(transform_notes(&c.notes, &t.midi_fx, new.bpm)))
                    .collect();
                rt.midi_sig = sig;
            }
            sync_chain(&t.effects, &mut rt.fx, sr);
            if rt.session.is_some() && t.session_clip(rt.session.unwrap()).is_none() {
                rt.session = None;
            }
        }
        // track removed
        for i in n..self.tracks.len() {
            self.tracks[i].reset_voices();
        }
        self.tracks.truncate(n);
        sync_chain(&new.master_fx, &mut self.master_fx, sr);
        if self.send_fx.kind != new.return_fx.kind {
            self.send_fx = FxState::new(new.return_fx.kind, sr);
        }
        self.song = new;
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Play => {
                if let Some(mut p) = self.preview.take() {
                    p.stop();
                }
                self.playing = true;
            }
            Cmd::Stop => {
                self.playing = false;
                if let Some(mut p) = self.preview.take() {
                    p.stop();
                }
                for t in &mut self.tracks {
                    t.release_all();
                    t.session = None;
                    t.pending = None;
                }
            }
            Cmd::Toggle => self.playing = !self.playing,
            Cmd::Seek(b) => self.beat = b.max(0.0),
            Cmd::Launch { track, scene } => {
                if let Some(t) = self.tracks.get_mut(track) {
                    t.pending = Some(scene);
                    if self.playing {
                        t.pending_at =
                            next_quantized_beat(self.beat, self.song.launch_quantum.max(0.0));
                    } else {
                        t.pending_at = self.beat;
                        self.playing = true;
                    }
                }
            }
            Cmd::StopTrack(track) => {
                if let Some(t) = self.tracks.get_mut(track) {
                    t.session = None;
                    t.pending = None;
                    t.release_all();
                }
            }
            Cmd::StopClips => {
                for t in &mut self.tracks {
                    t.session = None;
                    t.pending = None;
                    t.release_all();
                }
            }
            Cmd::NoteOn { track, note, vel } => {
                if let Some(t) = self.tracks.get_mut(track) {
                    self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
                    t.trigger_live(note, vel.clamp(0.0, 1.0), self.seed);
                }
            }
            Cmd::NoteOff { track, note } => {
                let sustain = self.live_sustain;
                if let Some(t) = self.tracks.get_mut(track) {
                    if sustain {
                        t.sustain_live(note);
                    } else {
                        t.release_live(note);
                    }
                }
            }
            Cmd::Sustain(on) => {
                self.live_sustain = on;
                if !on {
                    for t in &mut self.tracks {
                        t.release_sustained_live();
                    }
                }
            }
            Cmd::AllNotesOff => {
                self.live_sustain = false;
                for t in &mut self.tracks {
                    t.release_all_live();
                }
            }
            Cmd::Preview {
                instrument,
                notes,
                length,
                bpm,
                gain,
            } => {
                if let Some(mut old) = self.preview.take() {
                    old.stop();
                }
                if !notes.is_empty() {
                    self.preview = Some(PreviewState::new(instrument, notes, length, bpm, gain));
                }
            }
            Cmd::StopPreview => {
                if let Some(mut p) = self.preview.take() {
                    p.stop();
                }
            }
            Cmd::Panic => {
                self.playing = false;
                self.live_sustain = false;
                if let Some(mut p) = self.preview.take() {
                    p.stop();
                }
                for t in &mut self.tracks {
                    t.reset_voices();
                    t.session = None;
                    t.pending = None;
                }
            }
        }
    }

    fn poll_commands(&mut self) {
        let rx = self.cmd_rx.take();
        if let Some(rx) = &rx {
            while let Ok(cmd) = rx.try_recv() {
                self.handle(cmd);
            }
        }
        self.cmd_rx = rx;
    }

    fn poll_song(&mut self) {
        let shared = self.shared.clone();
        if let Some(shared) = shared {
            let mut latest = None;
            if let Ok(g) = shared.song.try_lock() {
                if let Some(arc) = g.as_ref() {
                    if !Arc::ptr_eq(arc, &self.song) {
                        latest = Some(arc.clone());
                    }
                }
            }
            if let Some(arc) = latest {
                self.sync_song(arc);
            }
        }
    }

    /// Trigger notes for this chunk. Runs once per 32-sample block.
    fn schedule(&mut self, frames: usize) {
        let spb = 60.0 / self.bpm();
        let dur = frames as f64 / (self.sr as f64 * spb);
        let b0 = self.beat;
        let b1 = b0 + dur;
        let song = self.song.clone();
        let any_solo = song.any_solo();

        for i in 0..self.tracks.len() {
            let tr = &song.tracks[i];
            let muted = tr.mute || (any_solo && !tr.solo);
            let rt = &mut self.tracks[i];

            // Quantized session launches.
            if let Some(scene) = rt.pending {
                if b1 >= rt.pending_at {
                    rt.session = Some(scene);
                    rt.session_start = rt.pending_at;
                    rt.pending = None;
                }
            }

            // Clip follow actions. They use the same quantized pending slot as
            // a manual launch, so a NEXT clip begins exactly on the follow
            // boundary rather than drifting with the audio block size.
            if self.playing && rt.pending.is_none() {
                if let Some(scene) = rt.session {
                    if let Some((action, after, length)) = tr
                        .session_clip(scene)
                        .map(|c| (c.follow_action, c.follow_after, c.length))
                    {
                        if action != FOLLOW_NONE {
                            let span = if after > 0.0 {
                                after * PPQ
                            } else {
                                length.max(0.25)
                            };
                            let trigger = rt.session_start + span;
                            if b1 + 1e-9 >= trigger {
                                let next = scene_action(tr, scene, action);
                                if action == FOLLOW_STOP {
                                    rt.session = None;
                                    rt.release_all();
                                } else if let Some(next) = next {
                                    rt.pending = Some(next);
                                    rt.pending_at = trigger.max(self.beat);
                                } else {
                                    rt.session = None;
                                    rt.release_all();
                                }
                            }
                        }
                    }
                }
            }

            if muted {
                continue;
            }

            // Release voices whose note length elapsed.
            for v in &mut rt.voices {
                if v.active && v.end_beat <= b1 && v.gate {
                    v.release();
                }
            }

            let mut clip_opt: Option<(usize, f64, f64, bool)> = None;
            if let Some(scene) = rt.session {
                if let Some(ci) = tr.clips.iter().position(|c| c.scene == scene) {
                    let c = &tr.clips[ci];
                    let local = b0 - rt.session_start;
                    clip_opt = Some((ci, local, dur, c.looping));
                }
            } else if let Some((ci, c)) = tr
                .clips
                .iter()
                .enumerate()
                .find(|(_, c)| c.scene < 0 && b1 > c.start && b0 < c.end())
            {
                let local = (b0 - c.start).max(0.0);
                clip_opt = Some((ci, local, dur, false));
            }

            let Some((clip_idx, local0, dur, looping)) = clip_opt else {
                continue;
            };
            let clip = &tr.clips[clip_idx];
            let transformed = rt.midi_notes.get(clip_idx).cloned();
            let notes = transformed.as_deref().unwrap_or(clip.notes.as_slice());
            let len = clip.length.max(0.0625);
            let ranges: [(f64, f64); 2] = if looping {
                let l0 = local0.rem_euclid(len);
                let l1 = l0 + dur;
                if l1 <= len {
                    [(l0, l1), (0.0, 0.0)]
                } else {
                    [(l0, len), (0.0, l1 - len)]
                }
            } else {
                let l0 = local0;
                let l1 = (local0 + dur).min(len);
                [(l0, l1), (0.0, 0.0)]
            };

            for (a, b) in ranges {
                if b <= a {
                    continue;
                }
                for note in notes {
                    if note.start >= a - 1e-9 && note.start < b {
                        let end = b0 + note.len * spb;
                        self.seed = self.seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        rt.trigger(note.pitch, note.vel, end, self.seed);
                    }
                }
            }

            // Arrangement clip ends: release any sounding notes.
            if !looping && local0 + dur > len {
                rt.release_all();
            }
        }

        if self.playing {
            let loop_end = song.loop_end;
            if song.loop_enabled && self.beat >= loop_end && loop_end > song.loop_start {
                self.beat = song.loop_start;
                for t in &mut self.tracks {
                    t.release_all();
                }
            }
        }
    }

    pub fn bpm(&self) -> f64 {
        self.song.bpm.max(20.0)
    }

    fn process_chunk(&mut self, out: &mut [f32], n: usize, channels: usize) {
        let sr = self.sr;
        let bpm = self.bpm();
        let spb = 60.0 / bpm;
        let song = self.song.clone();
        let any_solo = song.any_solo();

        if self.playing {
            self.schedule(n);
        }
        self.schedule_preview(n);

        for k in 0..n {
            let mut mix_l = 0.0f32;
            let mut mix_r = 0.0f32;
            let mut send_l = 0.0f32;
            let mut send_r = 0.0f32;

            for i in 0..song.tracks.len().min(self.tracks.len()) {
                let tr = &song.tracks[i];
                let muted = tr.mute || (any_solo && !tr.solo);
                let rt = &mut self.tracks[i];
                let TrackRt {
                    voices,
                    live,
                    instr,
                    fx,
                    dc_l,
                    dc_r,
                    ..
                } = rt;

                let mut tl = 0.0;
                let mut trr = 0.0;
                for v in voices.iter_mut().chain(live.iter_mut()) {
                    if !v.active {
                        continue;
                    }
                    let s = render_instrument(&tr.instrument, v, instr, sr, bpm, tr.take.as_ref());
                    if !v.active {
                        continue;
                    }
                    let (gl, gr) = pan_gains(tr.pan + v.pan);
                    tl += s * gl;
                    trr += s * gr;
                }

                for (j, dev) in tr.effects.iter().enumerate() {
                    if !dev.on || dev.kind.is_midi() {
                        continue;
                    }
                    if let Some(state) = fx.get_mut(j) {
                        let (a, b) = process_effect(dev, state, tl, trr, sr, self.beat, bpm);
                        tl = a;
                        trr = b;
                    }
                }

                // Imported vocal takes can acquire a DC term from pitch-shifted
                // wavefolding. Nonlinear track effects (compressors, drive,
                // bitcrush) can do the same to an asymmetric bass note even
                // when the instrument itself is clean. Remove that residual at
                // the post-effect track bus; the master bus is deliberately
                // left alone so the calibrated pattern library stays intact.
                let has_audio_fx = tr.effects.iter().any(|dev| dev.on && !dev.kind.is_midi());
                if tr.instrument.kind == DeviceKind::VocalTake || has_audio_fx {
                    let cutoff = if tr.instrument.kind == DeviceKind::VocalTake {
                        30.0
                    } else {
                        10.0
                    };
                    tl = highpass(
                        highpass(tl, &mut dc_l[0], cutoff, sr),
                        &mut dc_l[1],
                        cutoff,
                        sr,
                    );
                    trr = highpass(
                        highpass(trr, &mut dc_r[0], cutoff, sr),
                        &mut dc_r[1],
                        cutoff,
                        sr,
                    );
                }

                let target = if muted { 0.0 } else { tr.volume };
                rt.vol += (target - rt.vol) * 0.0015;
                let g = rt.vol * rt.vol;
                tl *= g;
                trr *= g;
                let peak = tl.abs().max(trr.abs());
                if peak > rt.peak {
                    rt.peak = peak;
                }
                if tr.send > 0.0001 {
                    send_l += tl * tr.send;
                    send_r += trr * tr.send;
                }
                mix_l += tl;
                mix_r += trr;
            }

            // Pattern audition bus. It uses the same instrument renderer and
            // master chain as track playback, but never mutates the project.
            if let Some(pv) = &mut self.preview {
                let PreviewState {
                    instrument,
                    instr,
                    voices,
                    gain,
                    bpm,
                    ..
                } = pv;
                for v in voices.iter_mut() {
                    if !v.active {
                        continue;
                    }
                    let s = render_instrument(instrument, v, instr, sr, *bpm, None);
                    if !v.active {
                        continue;
                    }
                    mix_l += s * *gain;
                    mix_r += s * *gain;
                }
            }

            // Send return (reverb bus).
            if song.return_fx.on {
                let (wl, wr) = process_effect(
                    &song.return_fx,
                    &mut self.send_fx,
                    send_l,
                    send_r,
                    sr,
                    self.beat,
                    bpm,
                );
                mix_l += wl * 0.7;
                mix_r += wr * 0.7;
            }

            // Master effects.
            for (j, dev) in song.master_fx.iter().enumerate() {
                if !dev.on {
                    continue;
                }
                if let Some(state) = self.master_fx.get_mut(j) {
                    let (a, b) = process_effect(dev, state, mix_l, mix_r, sr, self.beat, bpm);
                    mix_l = a;
                    mix_r = b;
                }
            }

            let vol = song.master_volume;
            mix_l *= vol;
            mix_r *= vol;

            if song.limiter {
                let peak = mix_l.abs().max(mix_r.abs());
                let target = if peak > 0.92 { 0.92 / peak } else { 1.0 };
                if target < self.lim_gain {
                    self.lim_gain = target;
                } else {
                    self.lim_gain += (1.0 - self.lim_gain) * 0.00004;
                }
                mix_l *= self.lim_gain;
                mix_r *= self.lim_gain;
                mix_l = mix_l.clamp(-0.999, 0.999);
                mix_r = mix_r.clamp(-0.999, 0.999);
            }

            if mix_l.abs() > self.peak_l {
                self.peak_l = mix_l.abs();
            }
            if mix_r.abs() > self.peak_r {
                self.peak_r = mix_r.abs();
            }

            if channels == 1 {
                out[k] = (mix_l + mix_r) * 0.6;
            } else {
                out[k * channels] = mix_l;
                out[k * channels + 1] = mix_r;
                for c in 2..channels {
                    out[k * channels + c] = 0.0;
                }
            }

            if self.playing {
                self.beat += 1.0 / (sr as f64 * spb);
            }
        }
    }

    pub fn process(&mut self, out: &mut [f32], channels: usize) {
        let start = Instant::now();
        self.poll_commands();
        self.poll_song();
        let frames = out.len() / channels.max(1);
        let mut done = 0;
        while done < frames {
            let n = (frames - done).min(32);
            self.process_chunk(
                &mut out[done * channels..(done + n) * channels],
                n,
                channels,
            );
            done += n;
        }

        let elapsed = start.elapsed().as_secs_f32();
        let budget = frames as f32 / self.sr.max(1.0);
        let load = if budget > 0.0 { elapsed / budget } else { 0.0 };
        self.cpu_smooth += (load - self.cpu_smooth) * 0.1;
        self.last_cpu = self.cpu_smooth;

        if let Some(shared) = &self.shared {
            shared.beat.store(self.beat.to_bits(), Ordering::Relaxed);
            shared.playing.store(self.playing, Ordering::Relaxed);
            shared
                .cpu
                .store(self.cpu_smooth.to_bits(), Ordering::Relaxed);
            for i in 0..MAX_TRACKS {
                let (session, pending) = self
                    .tracks
                    .get(i)
                    .map(|t| (t.session.unwrap_or(NO_SCENE), t.pending.unwrap_or(NO_SCENE)))
                    .unwrap_or((NO_SCENE, NO_SCENE));
                if let Some(a) = shared.session_scene.get(i) {
                    a.store(session, Ordering::Relaxed);
                }
                if let Some(a) = shared.pending_scene.get(i) {
                    a.store(pending, Ordering::Relaxed);
                }
                if i < self.tracks.len() {
                    let p = self.tracks[i].peak.min(1.0);
                    if let Some(a) = shared.levels.get(i) {
                        a.store(p.to_bits(), Ordering::Relaxed);
                    }
                    self.tracks[i].peak *= 0.8;
                }
            }
            if let Some(a) = shared.levels.get(MAX_TRACKS) {
                a.store(
                    self.peak_l.max(self.peak_r).min(1.0).to_bits(),
                    Ordering::Relaxed,
                );
            }
            self.peak_l *= 0.82;
            self.peak_r *= 0.82;
        }
    }

    fn schedule_preview(&mut self, frames: usize) {
        let sr = self.sr;
        let mut seed = self.seed;
        let Some(pv) = &mut self.preview else {
            return;
        };
        let spb = 60.0 / pv.bpm;
        let dur = frames as f64 / (sr as f64 * spb);
        let b1 = pv.pos + dur;

        let mut k = 0;
        while k < pv.offs.len() {
            if pv.offs[k].0 <= b1 {
                let note = pv.offs[k].1;
                for v in &mut pv.voices {
                    if v.active && v.note == note && v.gate {
                        v.release();
                    }
                }
                pv.offs.remove(k);
            } else {
                k += 1;
            }
        }

        while pv.next < pv.notes.len() && pv.notes[pv.next].start < b1 {
            let n = pv.notes[pv.next];
            if n.start >= pv.pos - 1e-9 {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let slot = preview_slot(&pv.voices);
                pv.voices[slot].note_on(n.pitch, n.vel, f64::INFINITY, seed);
                pv.offs.push((n.start + n.len, n.pitch));
            }
            pv.next += 1;
        }

        pv.pos = b1;
        if pv.pos >= pv.length + 0.5 && pv.offs.is_empty() && pv.next >= pv.notes.len() {
            self.preview = None;
        }
        self.seed = seed;
    }

    /// Offline render for WAV export. Renders the arrangement, or falls back
    /// to every session clip in parallel for a quick audition mixdown.
    pub fn render_song(song: &Song, sr: f32) -> Vec<f32> {
        let beats = song.length_beats().max(4.0) + 8.0;
        let seconds = beats * 60.0 / song.bpm.max(30.0);
        let frames = (seconds * sr as f64) as usize;
        let mut out = vec![0.0f32; frames * 2];
        let arc = Arc::new(song.clone());
        let mut e = Engine::new(sr, arc, None, None);
        // If there is no arrangement at all, audition the first session clip
        // on every track in parallel instead of rendering silence.
        if !song
            .tracks
            .iter()
            .any(|t| t.arrangement_clips().next().is_some())
        {
            for (i, t) in song.tracks.iter().enumerate() {
                if let Some(c) = t.clips.iter().find(|c| c.scene >= 0) {
                    if let Some(rt) = e.tracks.get_mut(i) {
                        rt.session = Some(c.scene);
                        rt.session_start = 0.0;
                    }
                }
            }
        }
        e.playing = true;
        e.beat = 0.0;
        e.process(&mut out, 2);
        out
    }
}

fn sync_chain(chain: &[Device], states: &mut Vec<FxState>, sr: f32) {
    while states.len() < chain.len() {
        let kind = chain[states.len()].kind;
        states.push(FxState::new(kind, sr));
    }
    for (i, d) in chain.iter().enumerate() {
        if states[i].kind != d.kind {
            states[i] = FxState::new(d.kind, sr);
        }
    }
    states.truncate(chain.len());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::DeviceKind;
    use crate::song::{Clip, FOLLOW_NEXT};

    fn advance_to(engine: &mut Engine, beat: f64) {
        let mut out = vec![0.0f32; 256 * 2];
        while engine.beat + 1e-9 < beat {
            engine.process(&mut out, 2);
        }
    }

    fn session_song(follow: bool) -> Song {
        let mut song = Song::empty("Session test", 240.0);
        song.add_track(DeviceKind::SubBass);
        song.ensure_scenes(2);
        let mut first = Clip::new("First", 0, 0.0, 4.0);
        if follow {
            first.follow_action = FOLLOW_NEXT;
            first.follow_after = 1.0;
        }
        song.tracks[0].clips.push(first);
        song.tracks[0].clips.push(Clip::new("Second", 1, 0.0, 4.0));
        song
    }

    #[test]
    fn session_launch_waits_for_global_quantum_and_exposes_pending_state() {
        let mut song = session_song(false);
        song.launch_quantum = 1.0;
        let (shared, arc, rx) = Shared::new(&song);
        let mut engine = Engine::new(48_000.0, arc, Some(rx), Some(shared.clone()));

        engine.handle(Cmd::Play);
        engine.handle(Cmd::Seek(1.1));
        engine.handle(Cmd::Launch { track: 0, scene: 0 });
        advance_to(&mut engine, 1.8);

        assert_eq!(engine.tracks[0].session, None);
        assert_eq!(engine.tracks[0].pending, Some(0));
        assert_eq!(engine.tracks[0].pending_at, 2.0);
        assert_eq!(shared.pending_scene(0), Some(0));
        assert_eq!(shared.session_scene(0), None);

        advance_to(&mut engine, 2.2);
        assert_eq!(engine.tracks[0].session, Some(0));
        assert_eq!(engine.tracks[0].pending, None);
        assert_eq!(shared.session_scene(0), Some(0));
    }

    #[test]
    fn follow_next_hands_off_to_the_next_scene_clip() {
        let mut song = session_song(true);
        song.launch_quantum = 0.0;
        let (shared, arc, rx) = Shared::new(&song);
        let mut engine = Engine::new(48_000.0, arc, Some(rx), Some(shared.clone()));

        engine.handle(Cmd::Launch { track: 0, scene: 0 });
        engine.handle(Cmd::Play);
        advance_to(&mut engine, 1.0);
        assert_eq!(engine.tracks[0].session, Some(0));
        assert_eq!(shared.session_scene(0), Some(0));

        advance_to(&mut engine, 4.2);
        assert_eq!(engine.tracks[0].session, Some(1));
        assert_eq!(engine.tracks[0].session_start, 4.0);
        assert_eq!(shared.session_scene(0), Some(1));
        assert_eq!(shared.pending_scene(0), None);
    }
}
