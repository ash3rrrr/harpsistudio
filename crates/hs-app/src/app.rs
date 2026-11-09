//! Harpsistudio application shell: transport, browser, view routing,
//! project actions and the keyboard-first workflow.

use crate::audio::{self, AudioHandle};
use crate::midi::{self, MidiHandle};
use crate::theme::*;
use crate::widgets::*;
use crate::{view_arrangement, view_device, view_mixer, view_piano, view_session};
use eframe::egui::{self, Align2, Color32, Id, Pos2, Rect, Sense, Ui, Vec2};
use hs_core::audio_take::AudioTake;
use hs_core::engine::{Cmd, Shared, MAX_TRACKS};
use hs_core::midi_fx;
use hs_core::params::{Device, DeviceKind};
use hs_core::patterns::{self, Genre, Role};
use hs_core::plugins::{self, PluginCategory};
use hs_core::presets::{self, VocalStyle, DEMOS};
use hs_core::project::{self, FileEntry};
use hs_core::song::{
    cycle_launch_quantum, follow_action_label, follow_after_label, format_position,
    launch_quantum_label, next_follow_action, next_follow_after, Clip, Note, Song,
};
use hs_core::sounds::{self, SoundCategory};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub const TOP_H: f32 = 60.0;
pub const BROWSER_W: f32 = 250.0;
pub const VIEWBAR_H: f32 = 30.0;
pub const STATUS_H: f32 = 26.0;

const CAPTURE_NAMES: [&str; 8] = [
    "01_session_browser_open",
    "02_clip_only",
    "03_device_only",
    "04_clip_device_stacked",
    "05_mixer",
    "06_session_browser_closed",
    "07_arrangement_browser_closed",
    "08_arrangement_browser_open",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Session,
    Arrangement,
    Mixer,
    Device,
    Piano,
}

impl View {
    pub fn label(self) -> &'static str {
        match self {
            View::Session => "SESSION",
            View::Arrangement => "ARRANGEMENT",
            View::Mixer => "MIXER",
            View::Device => "DEVICE",
            View::Piano => "CLIP",
        }
    }

    /// Session and Arrangement are the two main workspaces. Mixer is a
    /// full-width mode, while Piano and Device live in the bottom detail dock.
    pub fn is_detail(self) -> bool {
        matches!(self, View::Piano | View::Device)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BrowserTab {
    Starters,
    Patterns,
    Sounds,
    Demos,
    Files,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LibraryMode {
    Sounds,
    Plugins,
    Devices,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DevSel {
    Instrument,
    Effect(usize),
    Master,
    Return,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipRef {
    pub track: usize,
    pub scene: i32,
    pub start: f64,
}

#[derive(Clone, Copy, Debug)]
pub enum DragState {
    None,
    ArrangeClip {
        clip: ClipRef,
        grab_beat: f64,
    },
    Note {
        clip: ClipRef,
        index: usize,
        mode: NoteDrag,
        start_beat: f64,
        start_len: f64,
        start_pitch: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoteDrag {
    Move,
    Resize,
}

/// Computer-keyboard piano: A W S E D F T G Y H U J K = C..C.
pub const PIANO_KEYS: [(egui::Key, u8); 13] = [
    (egui::Key::A, 0),
    (egui::Key::W, 1),
    (egui::Key::S, 2),
    (egui::Key::E, 3),
    (egui::Key::D, 4),
    (egui::Key::F, 5),
    (egui::Key::T, 6),
    (egui::Key::G, 7),
    (egui::Key::Y, 8),
    (egui::Key::H, 9),
    (egui::Key::U, 10),
    (egui::Key::J, 11),
    (egui::Key::K, 12),
];

pub enum Act {
    LoadSound(usize),
    AddPluginRack(usize),
    LoadDemo(usize),
    LoadFile(PathBuf),
    Save,
    Export,
    New,
    AddTrack(DeviceKind),
    SetInstrument {
        track: usize,
        kind: DeviceKind,
    },
    AddEffect {
        track: usize,
        kind: DeviceKind,
    },
    ToggleEffect {
        track: usize,
        idx: usize,
    },
    RemoveEffect {
        track: usize,
        idx: usize,
    },
    MoveEffect {
        track: usize,
        idx: usize,
        dir: i32,
    },
    DeleteTrack(usize),
    Launch {
        track: usize,
        scene: i32,
    },
    StopClips,
    AddSessionClip {
        track: usize,
        scene: i32,
    },
    AddArrangementClip {
        track: usize,
        beat: f64,
        bars: f64,
    },
    OpenClip(ClipRef),
    DeleteClip(ClipRef),
    AddScene,
    ToggleLoop,
    Seek(f64),
    ImportVocal(VocalStyle),
    ToggleRecord(VocalStyle),
    PreviewPattern(usize),
    LoadPattern(usize),
    /// Create an empty, immediately editable track for a new part.
    CreatePart(DeviceKind),
    StopTrack(usize),
    CycleClipFollow(ClipRef),
    CycleClipFollowAfter(ClipRef),
    ToggleClipLoop(ClipRef),
    RestartAudio,
}

struct UiCapture {
    dir: PathBuf,
    index: usize,
    end: usize,
    delay: u32,
    requested: bool,
    done: Option<Arc<AtomicBool>>,
}

pub struct HarpsiApp {
    pub song: Song,
    pub shared: Arc<Shared>,
    pub audio: Option<AudioHandle>,
    pub audio_error: Option<String>,
    pub recorder: Option<audio::Recorder>,
    pub record_style: VocalStyle,
    pub vocal_style: VocalStyle,
    pub midi: Option<MidiHandle>,
    pub midi_name: String,
    pub sr: u32,
    pub device_name: String,
    pub view: View,
    pub main_view: View,
    pub clip_open: bool,
    pub device_open: bool,
    pub browser_open: bool,
    pub browser: BrowserTab,
    pub sel_track: usize,
    pub sel_scene: i32,
    pub clip_edit: Option<ClipRef>,
    pub dev_sel: DevSel,
    pub note_len: f64,
    pub note_vel: f32,
    pub sel_notes: BTreeSet<usize>,
    pub sel_anchor: Option<usize>,
    pub note_drag_origins: Vec<(usize, Note)>,
    pub note_clipboard: Vec<Note>,
    pub piano_cursor: f64,
    pub launched: Vec<Option<i32>>,
    pub files: Vec<FileEntry>,
    pub drag: DragState,
    pub status: String,
    pub status_until: f64,
    pub time: f64,
    pub dirty: bool,
    pub keyboard_octave: i32,
    key_notes: [Option<(usize, u8)>; 13],
    capture: Option<UiCapture>,
    pub pattern_genre: Option<Genre>,
    pub pattern_role: Option<Role>,
    pub pattern_key: Option<u8>,
    pub pattern_sel: usize,
    pub pattern_scroll: usize,
    pub pattern_preview_until: f64,
    pub library_mode: LibraryMode,
    pub sound_category: Option<SoundCategory>,
    pub plugin_category: Option<PluginCategory>,
    pub sound_search: String,
    pub plugin_search: String,
    pub sound_scroll: usize,
    pub plugin_scroll: usize,
    pub sound_sel: Option<usize>,
    pub plugin_sel: Option<usize>,
}

impl HarpsiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        let mut visuals = egui::Visuals::dark();
        visuals.extreme_bg_color = PANEL2;
        visuals.override_text_color = Some(TEXT);
        visuals.widgets.inactive.bg_fill = PANEL2;
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, LINE);
        visuals.widgets.hovered.bg_fill = PANEL3;
        visuals.selection.bg_fill = ACCENT_DARK;
        visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);
        cc.egui_ctx.set_visuals(visuals);
        cc.egui_ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(6.0, 6.0);
        });

        // Decode the embedded drum/piano bank before the audio callback starts.
        hs_core::samples::preload();
        let song = presets::demo_song(0);
        let (shared, _arc, _rx) = Shared::new(&song);
        let mut app = Self {
            song,
            shared,
            audio: None,
            audio_error: None,
            recorder: None,
            record_style: VocalStyle::Dubstep,
            vocal_style: VocalStyle::Dubstep,
            midi: None,
            midi_name: "No MIDI device".to_string(),
            sr: 48000,
            device_name: "—".to_string(),
            view: View::Session,
            main_view: View::Session,
            clip_open: false,
            device_open: false,
            browser_open: true,
            browser: BrowserTab::Starters,
            sel_track: 0,
            sel_scene: 0,
            clip_edit: None,
            dev_sel: DevSel::Instrument,
            note_len: 0.25,
            note_vel: 0.9,
            sel_notes: BTreeSet::new(),
            sel_anchor: None,
            note_drag_origins: Vec::new(),
            note_clipboard: Vec::new(),
            piano_cursor: 0.0,
            launched: Vec::new(),
            files: project::list_projects(),
            drag: DragState::None,
            status: "Welcome to Harpsistudio. Press SPACE to play.".to_string(),
            status_until: 6.0,
            time: 0.0,
            dirty: false,
            keyboard_octave: 4,
            key_notes: [None; 13],
            capture: None,
            pattern_genre: None,
            pattern_role: None,
            pattern_key: None,
            pattern_sel: 0,
            pattern_scroll: 0,
            pattern_preview_until: 0.0,
            library_mode: LibraryMode::Sounds,
            sound_category: None,
            plugin_category: None,
            sound_search: String::new(),
            plugin_search: String::new(),
            sound_scroll: 0,
            plugin_scroll: 0,
            sound_sel: Some(0),
            plugin_sel: Some(0),
        };
        app.launched = vec![None; app.song.tracks.len()];
        app.capture = std::env::var_os("HARPSI_CAPTURE_DIR").map(|dir| {
            let start = std::env::var("HARPSI_CAPTURE_START")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0)
                .min(CAPTURE_NAMES.len() - 1);
            let end = std::env::var("HARPSI_CAPTURE_END")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(CAPTURE_NAMES.len())
                .clamp(start + 1, CAPTURE_NAMES.len());
            UiCapture {
                dir: PathBuf::from(dir),
                index: start,
                end,
                delay: 18,
                requested: false,
                done: None,
            }
        });
        match audio::start(&app.song) {
            Ok(handle) => {
                app.shared = handle.shared.clone();
                app.sr = handle.sr;
                app.device_name = handle.device.clone();
                app.audio = Some(handle);
            }
            Err(e) => {
                app.audio_error = Some(e);
                app.status =
                    "Audio output unavailable — you can still edit and export.".to_string();
                app.status_until = 10.0;
            }
        }
        app.open_midi();
        app
    }

    pub fn flash(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
        self.status_until = self.time + 4.0;
    }

    pub fn publish_now(&mut self) {
        self.shared.publish(&self.song);
        self.dirty = false;
    }

    pub fn clear_note_selection(&mut self) {
        self.sel_notes.clear();
        self.sel_anchor = None;
        self.note_drag_origins.clear();
    }

    pub fn select_only_note(&mut self, index: usize) {
        self.sel_notes.clear();
        self.sel_notes.insert(index);
        self.sel_anchor = Some(index);
    }

    pub fn toggle_note_selection(&mut self, index: usize) {
        if !self.sel_notes.insert(index) {
            self.sel_notes.remove(&index);
        }
        self.sel_anchor = if self.sel_notes.contains(&index) {
            Some(index)
        } else {
            self.sel_notes.iter().next_back().copied()
        };
    }

    pub fn select_all_notes(&mut self) {
        let count = self
            .clip_edit
            .and_then(|cr| clip_at(&self.song, &cr))
            .map(|clip| clip.notes.len())
            .unwrap_or(0);
        self.sel_notes = (0..count).collect();
        self.sel_anchor = (count > 0).then_some(0);
    }

    pub fn delete_selected_notes(&mut self) -> bool {
        let Some(cr) = self.clip_edit else {
            return false;
        };
        let mut indices: Vec<usize> = self.sel_notes.iter().copied().collect();
        if indices.is_empty() {
            return false;
        }
        let Some(clip) = clip_at_mut(&mut self.song, &cr) else {
            return false;
        };
        indices.retain(|i| *i < clip.notes.len());
        indices.sort_unstable();
        for i in indices.iter().rev() {
            clip.notes.remove(*i);
        }
        self.clear_note_selection();
        self.dirty = true;
        true
    }

    /// Copy the selected notes into the internal clipboard. `cut` also removes
    /// them, leaving the clipboard intact.
    pub fn copy_selection(&mut self, cut: bool) -> bool {
        let Some(cr) = self.clip_edit else {
            return false;
        };
        let selected: Vec<usize> = self.sel_notes.iter().copied().collect();
        if selected.is_empty() {
            return false;
        }
        let mut notes: Vec<Note> = {
            let Some(clip) = clip_at(&self.song, &cr) else {
                return false;
            };
            selected
                .iter()
                .filter_map(|i| clip.notes.get(*i).copied())
                .collect()
        };
        if notes.is_empty() {
            return false;
        }
        let first = notes.iter().map(|n| n.start).fold(f64::INFINITY, f64::min);
        for note in &mut notes {
            note.start -= first;
        }
        self.note_clipboard = notes;
        if cut {
            self.delete_selected_notes();
        }
        true
    }

    /// Paste the clipboard at the piano-roll cursor, preserving relative note
    /// timing and optionally snapping pitches to the project scale.
    pub fn paste_clipboard(&mut self) -> bool {
        if self.note_clipboard.is_empty() {
            return false;
        }
        let Some(cr) = self.clip_edit else {
            return false;
        };
        let start = (self.piano_cursor.max(0.0) * 4.0).round() / 4.0;
        let scale_snap = self.song.scale_snap;
        let root = self.song.scale_root;
        let mode = self.song.scale_mode.clamp(0, 4) as usize;
        let source = self.note_clipboard.clone();
        let Some(clip) = clip_at_mut(&mut self.song, &cr) else {
            return false;
        };
        let length = clip.length.max(0.25);
        let mut pasted = Vec::new();
        for mut note in source {
            note.start = (start + note.start).clamp(0.0, (length - 0.125).max(0.0));
            if scale_snap {
                note.pitch =
                    midi_fx::nearest_scale_pitch(root, mode, note.pitch as i32).clamp(0, 127) as u8;
            }
            if note.start < length {
                pasted.push(note);
            }
        }
        if pasted.is_empty() {
            return false;
        }
        let base = clip.notes.len();
        let count = pasted.len();
        clip.notes.extend(pasted);
        self.sel_notes = (base..base + count).collect();
        self.sel_anchor = Some(base);
        self.dirty = true;
        true
    }

    /// Duplicate the selected notes immediately after the selection.
    pub fn duplicate_selection(&mut self) -> bool {
        let Some(cr) = self.clip_edit else {
            return false;
        };
        let selected: Vec<usize> = self.sel_notes.iter().copied().collect();
        if selected.is_empty() {
            return false;
        }
        let (notes, end) = {
            let Some(clip) = clip_at(&self.song, &cr) else {
                return false;
            };
            let notes: Vec<Note> = selected
                .iter()
                .filter_map(|i| clip.notes.get(*i).copied())
                .collect();
            if notes.is_empty() {
                return false;
            }
            let first = notes.iter().map(|n| n.start).fold(f64::INFINITY, f64::min);
            let end = notes.iter().map(|n| n.end()).fold(0.0, f64::max);
            let relative = notes
                .into_iter()
                .map(|mut n| {
                    n.start -= first;
                    n
                })
                .collect();
            (relative, end)
        };
        self.note_clipboard = notes;
        self.piano_cursor = end;
        self.paste_clipboard()
    }

    /// Short single-note preview through the engine's audition bus.
    pub fn audition_note(&mut self, pitch: u8, vel: f32) {
        if let Some(track) = self.song.tracks.get(self.sel_track) {
            self.shared.send(Cmd::Preview {
                instrument: track.instrument.clone(),
                notes: vec![Note::new(0.0, 0.25, pitch, vel.clamp(0.05, 1.0))],
                length: 0.5,
                bpm: self.song.bpm.max(30.0),
                gain: 0.6,
            });
        }
    }

    /// Rows the SNDS browser currently shows for the active mode.
    pub fn listed_rows(&self) -> Vec<usize> {
        match self.library_mode {
            LibraryMode::Sounds => self.visible_sounds(),
            LibraryMode::Plugins => self.visible_plugins(),
            LibraryMode::Devices => (0..DeviceKind::ALL.len()).collect(),
        }
    }

    /// Sound indices matching the active category and search, in library order.
    pub fn visible_sounds(&self) -> Vec<usize> {
        let query = self.sound_search.trim().to_ascii_lowercase();
        sounds::library()
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                self.sound_category.is_none_or(|c| s.category == c)
                    && (query.is_empty()
                        || s.name.to_ascii_lowercase().contains(&query)
                        || s.kind.name().to_ascii_lowercase().contains(&query)
                        || s.family.to_ascii_lowercase().contains(&query))
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Plugin-rack indices matching the active category and search.
    pub fn visible_plugins(&self) -> Vec<usize> {
        let query = self.plugin_search.trim().to_ascii_lowercase();
        plugins::library()
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                self.plugin_category.is_none_or(|c| r.category == c)
                    && (query.is_empty()
                        || r.name.to_ascii_lowercase().contains(&query)
                        || r.category.name().to_ascii_lowercase().contains(&query)
                        || r.summary().to_ascii_lowercase().contains(&query))
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn install_vocal(&mut self, take: AudioTake, style: VocalStyle) {
        let source = take.name.clone();
        let idx = presets::add_vocal_bass_track(&mut self.song, take, style);
        self.sync_launched();
        self.sel_track = idx;
        self.sel_scene = 0;
        self.clip_edit = None;
        self.dev_sel = DevSel::Instrument;
        self.view = View::Session;
        self.main_view = View::Session;
        self.clip_open = false;
        self.device_open = false;
        self.vocal_style = style;
        self.launched[idx] = Some(0);
        self.dirty = true;
        self.shared.publish(&self.song);
        self.shared.send(Cmd::Launch {
            track: idx,
            scene: 0,
        });
        self.shared.send(Cmd::Play);
        self.flash(format!(
            "{source} → {} Vox Bass track. Playing through Bassify + Wobble.",
            style.label()
        ));
    }

    fn import_vocal_path(&mut self, path: &Path, style: VocalStyle) {
        match AudioTake::from_wav_file(path) {
            Ok(take) => self.install_vocal(take, style),
            Err(e) => self.flash(format!("Vocal import failed: {e}")),
        }
    }

    fn poll_recording(&mut self) {
        let Some(recorder) = self.recorder.as_ref() else {
            return;
        };
        let result = recorder.poll();
        let style = self.record_style;
        let Some(result) = result else {
            return;
        };
        self.recorder = None;
        match result {
            Ok(recorded) => {
                let take =
                    AudioTake::from_samples(recorded.name, recorded.sample_rate, &recorded.samples);
                self.install_vocal(take, style);
            }
            Err(e) => self.flash(format!("Microphone failed: {e}")),
        }
    }

    fn poll_dropped_vocal(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .find(|p| {
                    p.extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.eq_ignore_ascii_case("wav"))
                        .unwrap_or(false)
                })
        });
        if let Some(path) = dropped {
            self.import_vocal_path(&path, self.vocal_style);
        }
    }

    fn open_midi(&mut self) {
        self.midi = None;
        match midi::start(self.shared.clone()) {
            Ok(handle) => {
                self.midi_name = handle.name.clone();
                self.midi = Some(handle);
            }
            Err(e) => self.midi_name = e,
        }
    }

    fn sync_launched(&mut self) {
        if self.launched.len() < self.song.tracks.len() {
            self.launched.resize(self.song.tracks.len(), None);
        }
        self.launched.truncate(self.song.tracks.len());
    }

    pub fn act(&mut self, act: Act) {
        match act {
            Act::LoadSound(idx) => {
                if let Some(sound) = sounds::library().get(idx) {
                    if let Some(t) = self.song.tracks.get_mut(self.sel_track) {
                        sound.load_into(&mut t.instrument);
                        t.name = sound.name.clone();
                        self.dev_sel = DevSel::Instrument;
                        self.view = View::Device;
                        self.device_open = true;
                        self.flash(format!(
                            "{} loaded on track {}.",
                            sound.name,
                            self.sel_track + 1
                        ));
                        self.publish_now();
                    }
                }
            }
            Act::AddPluginRack(idx) => {
                if let Some(rack) = plugins::library().get(idx) {
                    if let Some(t) = self.song.tracks.get_mut(self.sel_track) {
                        let first = rack.insert_into(&mut t.effects);
                        self.dev_sel = DevSel::Effect(first);
                        self.view = View::Device;
                        self.device_open = true;
                        self.flash(format!(
                            "{} inserted on track {} · {} devices.",
                            rack.name,
                            self.sel_track + 1,
                            rack.devices.len()
                        ));
                        self.publish_now();
                    }
                }
            }
            Act::SetInstrument { track, kind } => {
                if let Some(t) = self.song.tracks.get_mut(track) {
                    t.instrument = Device::new(kind);
                    t.name = kind.name().to_string();
                    self.dev_sel = DevSel::Instrument;
                    self.view = View::Device;
                    self.device_open = true;
                    self.flash(format!("{} loaded on track {}.", kind.name(), track + 1));
                    self.publish_now();
                }
            }
            Act::AddEffect { track, kind } => {
                if let Some(t) = self.song.tracks.get_mut(track) {
                    t.effects.push(Device::new(kind));
                    self.dev_sel = DevSel::Effect(t.effects.len() - 1);
                    self.view = View::Device;
                    self.device_open = true;
                    self.flash(format!("{} added.", kind.name()));
                    self.publish_now();
                }
            }
            Act::ToggleEffect { track, idx } => {
                if let Some(dev) = self
                    .song
                    .tracks
                    .get_mut(track)
                    .and_then(|t| t.effects.get_mut(idx))
                {
                    dev.on = !dev.on;
                    self.publish_now();
                }
            }
            Act::RemoveEffect { track, idx } => {
                if let Some(t) = self.song.tracks.get_mut(track) {
                    if idx < t.effects.len() {
                        t.effects.remove(idx);
                        self.dev_sel = DevSel::Instrument;
                        self.flash("Effect removed.");
                        self.publish_now();
                    }
                }
            }
            Act::MoveEffect { track, idx, dir } => {
                if let Some(t) = self.song.tracks.get_mut(track) {
                    let other = idx as i32 + dir;
                    if idx < t.effects.len() && other >= 0 && (other as usize) < t.effects.len() {
                        t.effects.swap(idx, other as usize);
                        self.dev_sel = DevSel::Effect(other as usize);
                        self.publish_now();
                    }
                }
            }
            Act::AddTrack(kind) => {
                let idx = self.song.add_track(kind);
                self.sel_track = idx;
                self.dev_sel = DevSel::Instrument;
                self.sync_launched();
                self.flash(format!("New {} track.", kind.name()));
                self.publish_now();
            }
            Act::CreatePart(kind) => {
                if self.song.tracks.len() >= MAX_TRACKS {
                    self.flash(format!("Track limit reached ({MAX_TRACKS} tracks)."));
                    return;
                }
                let idx = self.song.add_track(kind);
                self.song.ensure_scenes(1);
                let name = match kind {
                    DeviceKind::DrumRack => "Drums",
                    DeviceKind::SubBass => "Bass",
                    DeviceKind::Salamander => "Melody",
                    _ => kind.name(),
                };
                {
                    let track = &mut self.song.tracks[idx];
                    track.name = name.to_string();
                    let mut clip = presets::empty_clip(name, 0, 0.0, 4.0);
                    clip.name = format!("{name} 1");
                    track.clips.push(clip);
                }
                self.sel_track = idx;
                self.sel_scene = 0;
                self.clip_edit = Some(ClipRef {
                    track: idx,
                    scene: 0,
                    start: 0.0,
                });
                self.clear_note_selection();
                self.sync_launched();
                if idx < self.launched.len() {
                    self.launched[idx] = Some(0);
                }
                self.view = View::Piano;
                self.clip_open = true;
                self.publish_now();
                self.shared.send(Cmd::Launch {
                    track: idx,
                    scene: 0,
                });
                self.shared.send(Cmd::Play);
                self.flash(format!(
                    "New {name} track — draw your own part on the piano roll."
                ));
            }
            Act::DeleteTrack(track) => {
                if track < self.song.tracks.len() {
                    self.song.tracks.remove(track);
                    if let Some(cr) = self.clip_edit {
                        if cr.track == track {
                            self.clip_edit = None;
                        }
                    }
                    self.sel_track = self.sel_track.min(self.song.tracks.len().saturating_sub(1));
                    self.sync_launched();
                    self.flash("Track deleted.");
                    self.publish_now();
                }
            }
            Act::Launch { track, scene } => {
                self.shared.send(Cmd::Launch { track, scene });
                self.shared.send(Cmd::Play);
                self.sel_track = track;
                self.sel_scene = scene;
                if track < self.launched.len() {
                    self.launched[track] = Some(scene);
                }
            }
            Act::StopClips => {
                self.shared.send(Cmd::StopClips);
                self.launched.fill(None);
                self.flash("All session clips stopped.");
            }
            Act::StopTrack(track) => {
                self.shared.send(Cmd::StopTrack(track));
                if let Some(slot) = self.launched.get_mut(track) {
                    *slot = None;
                }
                self.flash(format!("Track {} stopped.", track + 1));
            }
            Act::CycleClipFollow(cr) => {
                if let Some(clip) = clip_at_mut(&mut self.song, &cr) {
                    clip.follow_action = next_follow_action(clip.follow_action);
                    if clip.follow_action != 0 && clip.follow_after <= 0.0 {
                        clip.follow_after = 4.0;
                    }
                    let label = follow_action_label(clip.follow_action);
                    self.flash(format!("Clip follow action: {label}."));
                    self.publish_now();
                }
            }
            Act::CycleClipFollowAfter(cr) => {
                if let Some(clip) = clip_at_mut(&mut self.song, &cr) {
                    clip.follow_after = next_follow_after(clip.follow_after);
                    let label = follow_after_label(clip.follow_after);
                    self.flash(format!("Clip follow after {label}."));
                    self.publish_now();
                }
            }
            Act::ToggleClipLoop(cr) => {
                if let Some(clip) = clip_at_mut(&mut self.song, &cr) {
                    clip.looping = !clip.looping;
                    let label = if clip.looping { "on" } else { "off" };
                    self.flash(format!("Clip loop {label}."));
                    self.publish_now();
                }
            }
            Act::AddSessionClip { track, scene } => {
                if let Some(t) = self.song.tracks.get_mut(track) {
                    if t.session_clip(scene).is_none() {
                        let kind = t.instrument.kind;
                        let mut c = presets::default_clip(kind, scene, 4.0);
                        c.name = t.name.clone();
                        t.clips.push(c);
                        self.clip_edit = Some(ClipRef {
                            track,
                            scene,
                            start: 0.0,
                        });
                        self.clear_note_selection();
                        self.flash("Clip created.");
                        self.publish_now();
                    }
                }
            }
            Act::AddArrangementClip { track, beat, bars } => {
                if let Some(t) = self.song.tracks.get_mut(track) {
                    let kind = t.instrument.kind;
                    let start = (beat / 4.0).floor() * 4.0;
                    let mut c = presets::default_clip(kind, -1, bars);
                    c.start = start;
                    c.name = t.name.clone();
                    let cr = ClipRef {
                        track,
                        scene: -1,
                        start,
                    };
                    t.clips.push(c);
                    self.sel_track = track;
                    self.clip_edit = Some(cr);
                    self.clear_note_selection();
                    self.flash("Clip created in arrangement.");
                    self.publish_now();
                }
            }
            Act::OpenClip(cr) => {
                self.clip_edit = Some(cr);
                self.sel_track = cr.track;
                self.clear_note_selection();
                self.view = View::Piano;
                self.clip_open = true;
            }
            Act::DeleteClip(cr) => {
                if let Some(t) = self.song.tracks.get_mut(cr.track) {
                    t.clips.retain(|c| {
                        !(c.scene == cr.scene
                            && (cr.scene >= 0 || (c.start - cr.start).abs() < 0.001))
                    });
                }
                if self.clip_edit.map(|c| c == cr).unwrap_or(false) {
                    self.clip_edit = None;
                }
                if cr.scene >= 0 {
                    self.shared.send(Cmd::StopTrack(cr.track));
                    if cr.track < self.launched.len() {
                        self.launched[cr.track] = None;
                    }
                }
                self.clear_note_selection();
                self.flash("Clip deleted.");
                self.publish_now();
            }
            Act::AddScene => {
                self.song
                    .scenes
                    .push(format!("Scene {}", self.song.scenes.len() + 1));
                self.flash("Scene added.");
                self.publish_now();
            }
            Act::ToggleLoop => {
                self.song.loop_enabled = !self.song.loop_enabled;
                if self.song.loop_enabled {
                    self.song.loop_end = self.song.length_beats().max(16.0);
                }
                self.flash(if self.song.loop_enabled {
                    "Loop on."
                } else {
                    "Loop off."
                });
                self.publish_now();
            }
            Act::Seek(beat) => {
                self.shared.send(Cmd::Seek(beat.max(0.0)));
            }
            Act::LoadDemo(idx) => {
                self.song = presets::demo_song(idx);
                self.vocal_style = if idx == 1 {
                    VocalStyle::Dubstep
                } else if idx == 7 {
                    VocalStyle::Rap
                } else {
                    self.vocal_style
                };
                self.after_project_load();
                self.flash(format!(
                    "Loaded {}. Use VOICE → BASS in START to add a real vocal.",
                    self.song.name
                ));
            }
            Act::PreviewPattern(idx) => {
                if let Some(p) = patterns::library().get(idx) {
                    let seconds = p.length_beats() * 60.0 / p.bpm().max(30.0);
                    self.shared.send(Cmd::Preview {
                        instrument: p.preview_device(),
                        notes: p.notes.clone(),
                        length: p.length_beats(),
                        bpm: p.bpm(),
                        gain: 0.75,
                    });
                    self.pattern_sel = idx;
                    self.pattern_preview_until = self.time + seconds + 0.4;
                    self.flash(format!(
                        "Previewing {} · {} {} · {:.0} BPM",
                        p.name,
                        p.key_name(),
                        p.scale.label(),
                        p.bpm()
                    ));
                }
            }
            Act::LoadPattern(idx) => {
                if let Some(p) = patterns::library().get(idx) {
                    let track = self.song.add_track(p.instrument);
                    self.song.ensure_scenes(1);
                    let start = (self.shared.beat() / 4.0).floor() * 4.0;
                    let bars = p.bars;
                    let t = &mut self.song.tracks[track];
                    t.name = p.name.clone();
                    t.instrument = p.preview_device();
                    t.volume = 0.8;
                    t.send = 0.18;
                    let mut session = Clip::new(&p.name, 0, 0.0, bars * 4.0);
                    session.notes = p.notes.clone();
                    let mut arrangement = session.clone();
                    arrangement.scene = -1;
                    arrangement.start = start;
                    t.clips.push(session);
                    t.clips.push(arrangement);

                    self.sel_track = track;
                    self.sel_scene = 0;
                    self.clip_edit = Some(ClipRef {
                        track,
                        scene: 0,
                        start: 0.0,
                    });
                    self.dev_sel = DevSel::Instrument;
                    self.view = View::Session;
                    self.main_view = View::Session;
                    self.clip_open = false;
                    self.device_open = false;
                    self.sync_launched();
                    self.launched[track] = Some(0);
                    self.publish_now();
                    self.shared.send(Cmd::StopPreview);
                    self.shared.send(Cmd::Launch { track, scene: 0 });
                    self.shared.send(Cmd::Play);
                    self.flash(format!(
                        "{} loaded on track {} · {} bars · {} {}",
                        p.name,
                        track + 1,
                        bars,
                        p.key_name(),
                        p.scale.label()
                    ));
                }
            }
            Act::ImportVocal(style) => {
                self.vocal_style = style;
                let picked = rfd::FileDialog::new()
                    .add_filter("WAV vocal or acapella", &["wav"])
                    .set_title("Load a dry vocal take")
                    .pick_file();
                if let Some(path) = picked {
                    self.import_vocal_path(&path, style);
                }
            }
            Act::ToggleRecord(style) => {
                self.vocal_style = style;
                if let Some(recorder) = &self.recorder {
                    recorder.stop();
                    self.flash("Finishing microphone take…");
                } else {
                    match audio::start_recording(8.0) {
                        Ok(recorder) => {
                            self.recorder = Some(recorder);
                            self.record_style = style;
                            self.flash(
                                "Recording 8 seconds — sing, speak or beatbox into the mic.",
                            );
                        }
                        Err(e) => self.flash(format!("Could not start microphone: {e}")),
                    }
                }
            }
            Act::LoadFile(path) => match project::load_song(&path) {
                Ok(song) => {
                    self.song = song;
                    self.after_project_load();
                    self.flash(format!("Opened {}", self.song.name));
                }
                Err(e) => self.flash(format!("Could not open project: {e}")),
            },
            Act::Save => match project::save_song(&self.song) {
                Ok(path) => {
                    self.files = project::list_projects();
                    self.flash(format!("Saved {}", path.display()));
                }
                Err(e) => self.flash(format!("Save failed: {e}")),
            },
            Act::Export => {
                self.flash("Rendering mixdown…");
                match project::export_wav(&self.song) {
                    Ok(path) => {
                        self.files = project::list_projects();
                        self.flash(format!("Mixdown written to {}", path.display()));
                    }
                    Err(e) => self.flash(format!("Export failed: {e}")),
                }
            }
            Act::New => {
                self.song = Song::empty("Untitled", 128.0);
                self.song.add_track(DeviceKind::DrumRack);
                self.after_project_load();
                self.flash("New project.");
            }
            Act::RestartAudio => {
                match audio::start(&self.song) {
                    Ok(h) => {
                        self.shared = h.shared.clone();
                        self.sr = h.sr;
                        self.device_name = h.device.clone();
                        self.audio = Some(h);
                        self.audio_error = None;
                        self.publish_now();
                        self.flash(format!("Audio running on {}", self.device_name));
                    }
                    Err(e) => {
                        self.audio_error = Some(e.clone());
                        self.flash(format!("Audio failed: {e}"));
                    }
                }
                self.key_notes = [None; 13];
                self.open_midi();
            }
        }
    }

    fn after_project_load(&mut self) {
        self.recorder = None;
        self.shared.send(Cmd::Stop);
        self.shared.send(Cmd::StopPreview);
        self.shared.send(Cmd::AllNotesOff);
        self.shared.send(Cmd::Seek(0.0));
        self.key_notes = [None; 13];
        self.publish_now();
        self.sel_track = 0;
        self.sel_scene = 0;
        self.clip_edit = None;
        self.dev_sel = DevSel::Instrument;
        self.clear_note_selection();
        self.view = View::Session;
        self.main_view = View::Session;
        self.clip_open = false;
        self.device_open = false;
        self.launched = vec![None; self.song.tracks.len()];
    }

    fn ensure_capture_clip(&mut self) {
        if self.clip_edit.is_some() {
            return;
        }
        let found = self
            .song
            .tracks
            .iter()
            .enumerate()
            .find_map(|(track, data)| {
                data.clips
                    .iter()
                    .find(|clip| clip.scene >= 0)
                    .map(|clip| ClipRef {
                        track,
                        scene: clip.scene,
                        start: clip.start,
                    })
            });
        if let Some(cr) = found {
            self.sel_track = cr.track;
            self.sel_scene = cr.scene.max(0);
            self.clip_edit = Some(cr);
            return;
        }

        if self.song.tracks.is_empty() {
            self.song.add_track(DeviceKind::DrumRack);
        }
        self.song.ensure_scenes(1);
        if let Some(track) = self.song.tracks.first_mut() {
            let mut clip = presets::empty_clip("Capture", 0, 0.0, 4.0);
            for i in 0..4 {
                clip.notes.push(Note::new(i as f64, 0.4, 36, 0.9));
            }
            track.clips.push(clip);
            self.sel_track = 0;
            self.sel_scene = 0;
            self.clip_edit = Some(ClipRef {
                track: 0,
                scene: 0,
                start: 0.0,
            });
        }
    }

    fn apply_capture_stage(&mut self, index: usize) {
        self.browser_open = !matches!(index, 5 | 6);
        self.status.clear();
        self.status_until = 0.0;
        match index {
            0 | 5 => {
                self.main_view = View::Session;
                self.view = View::Session;
                self.clip_open = false;
                self.device_open = false;
            }
            1 => {
                self.main_view = View::Session;
                self.ensure_capture_clip();
                self.clip_open = true;
                self.device_open = false;
                self.view = View::Piano;
            }
            2 => {
                self.main_view = View::Session;
                self.ensure_capture_clip();
                self.clip_open = false;
                self.device_open = true;
                self.view = View::Device;
            }
            3 => {
                self.main_view = View::Session;
                self.ensure_capture_clip();
                self.clip_open = true;
                self.device_open = true;
                self.view = View::Piano;
            }
            4 => {
                self.main_view = View::Session;
                self.ensure_capture_clip();
                self.clip_open = true;
                self.device_open = true;
                self.view = View::Mixer;
            }
            6 | 7 => {
                self.main_view = View::Arrangement;
                self.view = View::Arrangement;
                self.clip_open = false;
                self.device_open = false;
            }
            _ => {}
        }
    }

    fn capture_tick(&mut self, ctx: &egui::Context) {
        let Some(capture) = self.capture.as_ref() else {
            return;
        };
        let index = capture.index;
        let end = capture.end;
        let delay = capture.delay;
        let requested = capture.requested;
        let done = capture.done.clone();
        let dir = capture.dir.clone();

        if let Some(done) = done {
            if !done.load(Ordering::SeqCst) {
                ctx.request_repaint();
                return;
            }
            let next = index + 1;
            if next >= end {
                capture_log(&dir, &format!("close after {index}"));
                self.capture = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            capture_log(&dir, &format!("advance {index} -> {next}"));
            if let Some(capture) = self.capture.as_mut() {
                capture.index = next;
                capture.delay = 12;
                capture.requested = false;
                capture.done = None;
            }
            ctx.request_repaint();
            return;
        }

        if delay > 0 {
            if let Some(capture) = self.capture.as_mut() {
                capture.delay -= 1;
            }
            ctx.request_repaint();
            return;
        }

        if !requested {
            capture_log(&dir, &format!("apply {index}"));
            self.apply_capture_stage(index);
            if let Some(capture) = self.capture.as_mut() {
                capture.requested = true;
                capture.delay = 10;
            }
            ctx.request_repaint();
            return;
        }

        capture_log(&dir, &format!("request {index}"));
        let path = dir.join(format!("{}.bmp", CAPTURE_NAMES[index]));
        let done = Arc::new(AtomicBool::new(false));
        let callback_done = done.clone();
        let callback_dir = dir.clone();
        let callback_index = index;
        ctx.request_screenshot(move |image| {
            if let Err(err) = save_capture_bmp(&path, &image) {
                let _ = std::fs::write(path.with_extension("error.txt"), err.to_string());
            }
            capture_log(&callback_dir, &format!("saved {callback_index}"));
            callback_done.store(true, Ordering::SeqCst);
        });
        if let Some(capture) = self.capture.as_mut() {
            capture.done = Some(done);
        }
        ctx.request_repaint();
    }

    fn handle_keys(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let editing = ctx.text_edit_focused();
        let command = ctx.input(|i| i.modifiers.command);

        if !editing && !command {
            let mut octave_changed = false;
            if ctx.input(|i| i.key_pressed(egui::Key::Z)) {
                self.keyboard_octave = (self.keyboard_octave - 1).max(1);
                octave_changed = true;
            }
            if ctx.input(|i| i.key_pressed(egui::Key::X)) {
                self.keyboard_octave = (self.keyboard_octave + 1).min(7);
                octave_changed = true;
            }
            if octave_changed {
                self.flash(format!("Keyboard octave C{}", self.keyboard_octave));
            }
            if self.sel_track < self.song.tracks.len() {
                for (i, (key, offset)) in PIANO_KEYS.iter().enumerate() {
                    if self.key_notes[i].is_none() && ctx.input(|inp| inp.key_pressed(*key)) {
                        let pitch =
                            ((self.keyboard_octave + 1) * 12 + *offset as i32).clamp(0, 127) as u8;
                        self.shared.send(Cmd::NoteOn {
                            track: self.sel_track,
                            note: pitch,
                            vel: 0.8,
                        });
                        self.key_notes[i] = Some((self.sel_track, pitch));
                    }
                }
            }
        }

        // Always process key-ups, even if a text field just took focus.
        for (i, (key, _)) in PIANO_KEYS.iter().enumerate() {
            if ctx.input(|inp| inp.key_released(*key)) {
                if let Some((track, pitch)) = self.key_notes[i].take() {
                    self.shared.send(Cmd::NoteOff { track, note: pitch });
                }
            }
        }

        if editing {
            return;
        }
        let playing = self.shared.is_playing();
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            if ctx.input(|i| i.modifiers.shift) {
                self.act(Act::StopClips);
            } else if playing {
                self.shared.send(Cmd::Stop);
                self.launched.fill(None);
            } else {
                self.shared.send(Cmd::Play);
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.shared.send(Cmd::Panic);
            self.launched.fill(None);
            self.key_notes.fill(None);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Num1)) {
            self.show_main(View::Session);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Num2)) {
            self.show_main(View::Arrangement);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Num3)) {
            self.toggle_clip();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Num4)) {
            self.toggle_device();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Num5)) {
            self.toggle_mixer();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
            let next = if self.main_view == View::Session {
                View::Arrangement
            } else {
                View::Session
            };
            self.show_main(next);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            let b = self.shared.beat();
            self.shared.send(Cmd::Seek(b + 4.0));
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            let b = self.shared.beat();
            self.shared.send(Cmd::Seek((b - 4.0).max(0.0)));
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.act(Act::Save);
        }
        if self.view == View::Piano && command {
            if ctx.input(|i| i.key_pressed(egui::Key::C)) && self.copy_selection(false) {
                self.flash("Notes copied.");
            }
            if ctx.input(|i| i.key_pressed(egui::Key::X)) && self.copy_selection(true) {
                self.publish_now();
                self.flash("Notes cut.");
            }
            if ctx.input(|i| i.key_pressed(egui::Key::V)) && self.paste_clipboard() {
                self.publish_now();
                self.flash("Notes pasted.");
            }
            if ctx.input(|i| i.key_pressed(egui::Key::D)) && self.duplicate_selection() {
                self.publish_now();
                self.flash("Notes duplicated.");
            }
            if ctx.input(|i| i.key_pressed(egui::Key::A)) {
                self.select_all_notes();
                self.flash("All notes selected.");
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
            let shift = ctx.input(|i| i.modifiers.shift);
            if self.view == View::Piano && !shift {
                if self.delete_selected_notes() {
                    self.publish_now();
                }
            } else if let Some(cr) = self.clip_edit {
                self.act(Act::DeleteClip(cr));
            }
        }
    }

    // -----------------------------------------------------------------
    // Layout
    // -----------------------------------------------------------------

    fn top_bar(&mut self, ui: &mut Ui, rect: Rect) {
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 0.0, PANEL);
        hline(&p, rect.left(), rect.right(), rect.bottom() - 1.0, LINE);

        // Brand
        let x = rect.left() + 16.0;
        mono(
            &p,
            Pos2::new(x, rect.top() + 9.0),
            Align2::LEFT_TOP,
            "HARPSI",
            19.0,
            ACCENT,
        );
        mono(
            &p,
            Pos2::new(x + 60.0, rect.top() + 9.0),
            Align2::LEFT_TOP,
            "STUDIO",
            19.0,
            TEXT,
        );
        txt(
            &p,
            Pos2::new(x + 1.0, rect.top() + 34.0),
            Align2::LEFT_TOP,
            "BEAT WORKSTATION",
            7.5,
            FAINT,
        );

        let playing = self.shared.is_playing();
        let by = rect.top() + 13.0;

        // Live's browser control lives at the top-left of the outer frame.
        let browser_r = Rect::from_min_size(
            Pos2::new(rect.left() + 196.0, by + 4.0),
            Vec2::new(88.0, 26.0),
        );
        if chip(
            ui,
            browser_r,
            Id::unique("browser_toggle"),
            "BROWSER",
            self.browser_open,
            MINT,
        )
        .clicked()
        {
            self.browser_open = !self.browser_open;
        }

        let mut tx = rect.left() + 292.0;

        // Rewind
        let rew = Rect::from_min_size(Pos2::new(tx, by + 3.0), Vec2::new(30.0, 28.0));
        let r = ui.interact(rew, Id::unique("rew"), Sense::click());
        if r.hovered() {
            rounded(&p, rew, 3.0, PANEL2);
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        txt(&p, rew.center(), Align2::CENTER_CENTER, "◀◀", 9.0, DIM);
        if r.clicked() {
            self.act(Act::Seek(0.0));
        }
        tx += 36.0;

        // Play
        let play_r = Rect::from_min_size(Pos2::new(tx, by), Vec2::new(46.0, 34.0));
        let pr = ui.interact(play_r, Id::unique("play"), Sense::click());
        let pb = if playing {
            ACCENT
        } else if pr.hovered() {
            PANEL3
        } else {
            PANEL2
        };
        rounded(&p, play_r, 4.0, pb);
        outline(&p, play_r, 4.0, if playing { ACCENT } else { LINE }, 1.0);
        let c = play_r.center();
        p.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(c.x - 4.0, c.y - 8.0),
                Pos2::new(c.x + 9.0, c.y),
                Pos2::new(c.x - 4.0, c.y + 8.0),
            ],
            if playing { BG } else { MINT },
            egui::Stroke::new(0.0, Color32::TRANSPARENT),
        ));
        if pr.clicked() {
            if playing {
                self.shared.send(Cmd::Stop);
                self.launched.fill(None);
            } else {
                self.shared.send(Cmd::Play);
            }
        }
        tx += 52.0;

        // Stop
        let stop_r = Rect::from_min_size(Pos2::new(tx, by), Vec2::new(38.0, 34.0));
        let sr = ui.interact(stop_r, Id::unique("stop"), Sense::click());
        rounded(&p, stop_r, 4.0, if sr.hovered() { PANEL3 } else { PANEL2 });
        outline(&p, stop_r, 4.0, LINE, 1.0);
        p.rect_filled(
            Rect::from_center_size(stop_r.center(), Vec2::new(10.0, 10.0)),
            1.0,
            TEXT,
        );
        if sr.clicked() {
            self.shared.send(Cmd::Stop);
            self.launched.fill(None);
        }
        tx += 44.0;

        // Loop
        let loop_r = Rect::from_min_size(Pos2::new(tx, by + 4.0), Vec2::new(50.0, 26.0));
        let lr = ui.interact(loop_r, Id::unique("loop"), Sense::click());
        let lb = if self.song.loop_enabled {
            ACCENT
        } else {
            PANEL2
        };
        rounded(&p, loop_r, 3.0, lb);
        outline(
            &p,
            loop_r,
            3.0,
            if self.song.loop_enabled { ACCENT } else { LINE },
            1.0,
        );
        txt(
            &p,
            loop_r.center(),
            Align2::CENTER_CENTER,
            "LOOP",
            9.0,
            if self.song.loop_enabled { BG } else { DIM },
        );
        if lr.clicked() {
            self.act(Act::ToggleLoop);
        }
        tx += 60.0;

        // Position
        let beat = self.shared.beat();
        mono(
            &p,
            Pos2::new(tx, rect.top() + 12.0),
            Align2::LEFT_TOP,
            format_position(beat),
            19.0,
            TEXT,
        );
        txt(
            &p,
            Pos2::new(tx + 1.0, rect.top() + 36.0),
            Align2::LEFT_TOP,
            "BAR . BEAT . 16TH",
            7.5,
            FAINT,
        );
        tx += 128.0;

        // Tempo
        let bpm_rect = Rect::from_min_size(Pos2::new(tx, rect.top() + 8.0), Vec2::new(72.0, 38.0));
        let mut bpm = self.song.bpm;
        if drag_number(ui, bpm_rect, Id::unique("bpm"), &mut bpm, 60.0, 200.0, 0.5) {
            self.song.bpm = bpm;
            self.dirty = true;
        }
        mono(
            &p,
            Pos2::new(bpm_rect.center().x, rect.top() + 12.0),
            Align2::CENTER_TOP,
            format!("{:.1}", self.song.bpm),
            17.0,
            ACCENT,
        );
        txt(
            &p,
            Pos2::new(bpm_rect.center().x, rect.top() + 36.0),
            Align2::CENTER_TOP,
            "BPM",
            7.5,
            FAINT,
        );

        // Global launch quantization, like Live's scene-launch grid.
        let launch_r = Rect::from_min_size(Pos2::new(tx + 84.0, by + 4.0), Vec2::new(104.0, 26.0));
        if chip(
            ui,
            launch_r,
            Id::unique("launchquant"),
            &format!("LAUNCH {}", launch_quantum_label(self.song.launch_quantum)),
            false,
            BLUE,
        )
        .clicked()
        {
            self.song.launch_quantum = cycle_launch_quantum(self.song.launch_quantum);
            self.publish_now();
            self.flash(format!(
                "Session launch quantization: {}.",
                launch_quantum_label(self.song.launch_quantum)
            ));
        }

        // Right cluster. The Live 12 view controls own the top-right corner.
        let view_w = 168.0;
        let right = rect.right() - 14.0 - view_w - 10.0;
        let meter_r = Rect::from_min_size(
            Pos2::new(right - 10.0, rect.top() + 12.0),
            Vec2::new(9.0, 36.0),
        );
        meter(&p, meter_r, self.shared.level(MAX_TRACKS), true);

        let cpu = self.shared.cpu() * 100.0;
        txt(
            &p,
            Pos2::new(right - 26.0, rect.top() + 17.0),
            Align2::RIGHT_TOP,
            format!("{:.0}%", cpu),
            10.0,
            if cpu > 80.0 { RED } else { DIM },
        );
        txt(
            &p,
            Pos2::new(right - 26.0, rect.top() + 31.0),
            Align2::RIGHT_TOP,
            "CPU",
            7.5,
            FAINT,
        );

        let mix_r = Rect::from_min_size(
            Pos2::new(right - 26.0 - 76.0, by + 4.0),
            Vec2::new(72.0, 26.0),
        );
        let mr = ui.interact(mix_r, Id::unique("mixdown"), Sense::click());
        rounded(
            &p,
            mix_r,
            3.0,
            if mr.hovered() {
                mix(PANEL2, MINT, 0.18)
            } else {
                PANEL2
            },
        );
        outline(&p, mix_r, 3.0, MINT, 1.0);
        txt(
            &p,
            mix_r.center(),
            Align2::CENTER_CENTER,
            "MIXDOWN",
            9.0,
            MINT,
        );
        if mr.clicked() {
            self.act(Act::Export);
        }

        let save_r = Rect::from_min_size(
            Pos2::new(mix_r.left() - 60.0, by + 4.0),
            Vec2::new(54.0, 26.0),
        );
        let srv = ui.interact(save_r, Id::unique("save"), Sense::click());
        rounded(
            &p,
            save_r,
            3.0,
            if srv.hovered() {
                mix(PANEL2, ACCENT, 0.18)
            } else {
                PANEL2
            },
        );
        outline(&p, save_r, 3.0, ACCENT, 1.0);
        txt(
            &p,
            save_r.center(),
            Align2::CENTER_CENTER,
            "SAVE",
            9.0,
            ACCENT,
        );
        if srv.clicked() {
            self.act(Act::Save);
        }

        let name_r = Rect::from_min_size(
            Pos2::new(save_r.left() - 202.0, by + 5.0),
            Vec2::new(194.0, 24.0),
        );
        if name_r.left() > tx + 70.0 {
            let mut name = self.song.name.clone();
            let resp = ui.put(
                name_r,
                egui::TextEdit::singleline(&mut name)
                    .desired_width(name_r.width())
                    .text_color(TEXT),
            );
            if resp.changed() {
                self.song.name = name;
                self.dirty = true;
            }
            txt(
                &p,
                Pos2::new(name_r.left(), name_r.top() - 3.0),
                Align2::LEFT_BOTTOM,
                "PROJECT",
                7.5,
                FAINT,
            );
        }

        if self.audio.is_none() {
            let ar = Rect::from_min_size(
                Pos2::new(name_r.left().max(tx + 4.0), by + 36.0),
                Vec2::new(110.0, 18.0),
            );
            if ar.left() > tx && ar.right() < save_r.left() {
                txt(
                    &p,
                    ar.left_top(),
                    Align2::LEFT_TOP,
                    "AUDIO OFF · CLICK TO FIX",
                    8.0,
                    RED,
                );
                let resp = ui.interact(ar, Id::unique("fixaudio"), Sense::click());
                if resp.clicked() {
                    self.act(Act::RestartAudio);
                }
            }
        }

        let view_r = Rect::from_min_size(
            Pos2::new(rect.right() - 14.0 - view_w, by),
            Vec2::new(view_w, 34.0),
        );
        self.view_switch(ui, view_r);
    }

    fn browser(&mut self, ui: &mut Ui, rect: Rect) {
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 0.0, PANEL);
        vline(&p, rect.right() - 1.0, rect.top(), rect.bottom(), LINE);
        txt(
            &p,
            Pos2::new(rect.left() + 14.0, rect.top() + 10.0),
            Align2::LEFT_TOP,
            "BROWSER",
            10.0,
            DIM,
        );
        mono(
            &p,
            Pos2::new(rect.left() + 14.0, rect.top() + 24.0),
            Align2::LEFT_TOP,
            format!("{} SOUNDS · {} PLUGINS", sounds::count(), plugins::count()),
            8.0,
            FAINT,
        );

        let tab_y = rect.top() + 40.0;
        let tabs = [
            ("START", BrowserTab::Starters),
            ("PATS", BrowserTab::Patterns),
            ("SNDS", BrowserTab::Sounds),
            ("DEMOS", BrowserTab::Demos),
            ("FILES", BrowserTab::Files),
        ];
        let tw = (rect.width() - 28.0 - 16.0) / 5.0;
        for (i, (label, tab)) in tabs.iter().enumerate() {
            let r = Rect::from_min_size(
                Pos2::new(rect.left() + 14.0 + i as f32 * (tw + 4.0), tab_y),
                Vec2::new(tw, 24.0),
            );
            if chip(
                ui,
                r,
                Id::unique(("btab", i)),
                label,
                self.browser == *tab,
                ACCENT,
            )
            .clicked()
            {
                self.browser = *tab;
            }
        }

        let content = Rect::from_min_max(
            Pos2::new(rect.left() + 14.0, tab_y + 34.0),
            Pos2::new(rect.right() - 14.0, rect.bottom() - 12.0),
        );
        let mut act: Option<Act> = None;
        match self.browser {
            BrowserTab::Starters => {
                let mut y = content.top();
                section(&p, Pos2::new(content.left(), y), "VOICE → BASS");
                y += 18.0;
                let vp = Rect::from_min_size(
                    Pos2::new(content.left(), y),
                    Vec2::new(content.width(), 96.0),
                );
                rounded(&p, vp, 4.0, mix(PANEL2, MINT, 0.08));
                outline(&p, vp, 4.0, mix(LINE, MINT, 0.45), 1.0);
                p.rect_filled(
                    Rect::from_min_size(vp.min, Vec2::new(3.0, vp.height())),
                    1.0,
                    MINT,
                );
                txt(
                    &p,
                    Pos2::new(vp.left() + 11.0, vp.top() + 7.0),
                    Align2::LEFT_TOP,
                    "REAL VOCAL → TALKING BASS",
                    9.0,
                    MINT,
                );
                txt(
                    &p,
                    Pos2::new(vp.left() + 11.0, vp.top() + 21.0),
                    Align2::LEFT_TOP,
                    "WAV, mic or system audio → Vocal Take → Bassify",
                    8.0,
                    DIM,
                );
                let bw = (vp.width() - 22.0) / 2.0;
                let load_r = Rect::from_min_size(
                    Pos2::new(vp.left() + 8.0, vp.top() + 40.0),
                    Vec2::new(bw, 22.0),
                );
                let rec_r = Rect::from_min_size(
                    Pos2::new(load_r.right() + 6.0, vp.top() + 40.0),
                    Vec2::new(bw, 22.0),
                );
                if chip(
                    ui,
                    load_r,
                    Id::unique("vocal_load"),
                    "LOAD WAV",
                    false,
                    MINT,
                )
                .clicked()
                {
                    act = Some(Act::ImportVocal(self.vocal_style));
                }
                let recording = self.recorder.as_ref().map(|r| r.progress()).unwrap_or(0.0);
                let rec_label = if self.recorder.is_some() {
                    format!("STOP {:.0}%", recording * 100.0)
                } else {
                    "RECORD 8s".to_string()
                };
                if chip(
                    ui,
                    rec_r,
                    Id::unique("vocal_rec"),
                    &rec_label,
                    self.recorder.is_some(),
                    RED,
                )
                .clicked()
                {
                    act = Some(Act::ToggleRecord(self.vocal_style));
                }
                let sw = (vp.width() - 22.0) / 2.0;
                let dub_r = Rect::from_min_size(
                    Pos2::new(vp.left() + 8.0, vp.top() + 68.0),
                    Vec2::new(sw, 20.0),
                );
                let rap_r = Rect::from_min_size(
                    Pos2::new(dub_r.right() + 6.0, vp.top() + 68.0),
                    Vec2::new(sw, 20.0),
                );
                if chip(
                    ui,
                    dub_r,
                    Id::unique("vocal_dub"),
                    "DUBSTEP",
                    self.vocal_style == VocalStyle::Dubstep,
                    ACCENT,
                )
                .clicked()
                {
                    self.vocal_style = VocalStyle::Dubstep;
                }
                if chip(
                    ui,
                    rap_r,
                    Id::unique("vocal_rap"),
                    "RAP",
                    self.vocal_style == VocalStyle::Rap,
                    ACCENT,
                )
                .clicked()
                {
                    self.vocal_style = VocalStyle::Rap;
                }
                y += 102.0;

                section(&p, Pos2::new(content.left(), y), "GENRE STARTERS");
                y += 18.0;
                let starters = [
                    (
                        1usize,
                        "DUBSTEP",
                        "Half-time kit, growl bass, dark chords and a Salamander piano hook.",
                        [255, 126, 176],
                    ),
                    (
                        0usize,
                        "HOUSE",
                        "Four-on-the-floor, offbeat sub, warm chord stabs and a supersaw lead.",
                        [90, 220, 170],
                    ),
                    (
                        2usize,
                        "EDM",
                        "Wide supersaw chords, a drums-out build and a festival drop.",
                        [255, 190, 60],
                    ),
                    (
                        7usize,
                        "RAP",
                        "Boom-bap drums, felt piano, 808 and a vocal-forward rap-flow topline.",
                        [245, 206, 130],
                    ),
                ];
                for (idx, tag, blurb, col) in starters {
                    let r = Rect::from_min_size(
                        Pos2::new(content.left(), y),
                        Vec2::new(content.width(), 92.0),
                    );
                    let resp = ui.interact(r, Id::unique(("starter", idx)), Sense::click());
                    let col = Color32::from_rgb(col[0], col[1], col[2]);
                    rounded(&p, r, 4.0, if resp.hovered() { PANEL2 } else { RGBA_PANEL });
                    outline(
                        &p,
                        r,
                        4.0,
                        if resp.hovered() { col } else { LINE_SOFT },
                        1.0,
                    );
                    p.rect_filled(
                        Rect::from_min_size(r.min, Vec2::new(3.0, r.height())),
                        1.0,
                        col,
                    );
                    txt(
                        &p,
                        Pos2::new(r.left() + 12.0, r.top() + 8.0),
                        Align2::LEFT_TOP,
                        tag,
                        10.0,
                        col,
                    );
                    mono(
                        &p,
                        Pos2::new(r.right() - 10.0, r.top() + 8.0),
                        Align2::RIGHT_TOP,
                        format!("{:.0} BPM", DEMOS[idx].bpm),
                        8.0,
                        FAINT,
                    );
                    txt(
                        &p,
                        Pos2::new(r.left() + 12.0, r.top() + 25.0),
                        Align2::LEFT_TOP,
                        DEMOS[idx].name,
                        13.0,
                        TEXT,
                    );
                    txt(
                        &p,
                        Pos2::new(r.left() + 12.0, r.top() + 45.0),
                        Align2::LEFT_TOP,
                        fit(blurb, r.width() - 24.0, 9.0),
                        9.0,
                        DIM,
                    );
                    txt(
                        &p,
                        Pos2::new(r.left() + 12.0, r.bottom() - 10.0),
                        Align2::LEFT_BOTTOM,
                        "LOAD STARTER →",
                        8.0,
                        if resp.hovered() { MINT } else { FAINT },
                    );
                    if resp.clicked() {
                        act = Some(Act::LoadDemo(idx));
                    }
                    y += 96.0;
                }
                if self.recorder.is_none() {
                    txt(
                        &p,
                        Pos2::new(content.left(), content.bottom() - 8.0),
                        Align2::LEFT_BOTTOM,
                        "Or drag a .wav onto the window. Imports are saved inside the project.",
                        7.5,
                        FAINT,
                    );
                }
            }
            BrowserTab::Patterns => {
                let lib = patterns::library();
                let mut y = content.top();
                section(&p, Pos2::new(content.left(), y), "PATTERN LIBRARY");
                mono(
                    &p,
                    Pos2::new(content.right(), y),
                    Align2::RIGHT_TOP,
                    format!("{} TOTAL", lib.len()),
                    7.5,
                    FAINT,
                );
                y += 18.0;

                let mut filters_changed = false;
                let genre_opts: Vec<Option<Genre>> = std::iter::once(None)
                    .chain(Genre::ALL.iter().copied().map(Some))
                    .collect();
                let gw = (content.width() - 8.0) / 3.0;
                for (i, g) in genre_opts.iter().enumerate() {
                    let r = Rect::from_min_size(
                        Pos2::new(
                            content.left() + (i % 3) as f32 * (gw + 4.0),
                            y + (i / 3) as f32 * 21.0,
                        ),
                        Vec2::new(gw, 19.0),
                    );
                    let label = g.map(|g| g.short()).unwrap_or("ALL");
                    if chip(
                        ui,
                        r,
                        Id::unique(("patgenre", i)),
                        label,
                        self.pattern_genre == *g,
                        ACCENT,
                    )
                    .clicked()
                    {
                        self.pattern_genre = *g;
                        filters_changed = true;
                    }
                }
                y += 3.0 * 21.0 + 5.0;

                let role_opts: Vec<Option<Role>> = std::iter::once(None)
                    .chain(Role::ALL.iter().copied().map(Some))
                    .collect();
                let rw = (content.width() - 12.0) / 4.0;
                for (i, role) in role_opts.iter().enumerate() {
                    let r = Rect::from_min_size(
                        Pos2::new(
                            content.left() + (i % 4) as f32 * (rw + 4.0),
                            y + (i / 4) as f32 * 21.0,
                        ),
                        Vec2::new(rw, 19.0),
                    );
                    let label = role.map(|r| r.label()).unwrap_or("ALL");
                    if chip(
                        ui,
                        r,
                        Id::unique(("patrole", i)),
                        label,
                        self.pattern_role == *role,
                        MINT,
                    )
                    .clicked()
                    {
                        self.pattern_role = *role;
                        filters_changed = true;
                    }
                }
                y += 2.0 * 21.0 + 5.0;

                let key_label = match self.pattern_key {
                    Some(k) => format!("KEY {}", patterns::KEY_NAMES[(k as usize) % 12]),
                    None => "KEY · ANY".to_string(),
                };
                let key_r = Rect::from_min_size(
                    Pos2::new(content.left(), y),
                    Vec2::new(content.width() - 80.0, 21.0),
                );
                if chip(
                    ui,
                    key_r,
                    Id::unique("patkey"),
                    &key_label,
                    self.pattern_key.is_some(),
                    BLUE,
                )
                .clicked()
                {
                    self.pattern_key = match self.pattern_key {
                        None => Some(0),
                        Some(k) if k < 11 => Some(k + 1),
                        _ => None,
                    };
                    filters_changed = true;
                }
                let reset_r = Rect::from_min_size(
                    Pos2::new(content.right() - 74.0, y),
                    Vec2::new(74.0, 21.0),
                );
                if chip(ui, reset_r, Id::unique("patreset"), "RESET", false, RED).clicked() {
                    self.pattern_genre = None;
                    self.pattern_role = None;
                    self.pattern_key = None;
                    filters_changed = true;
                }
                y += 27.0;
                if filters_changed {
                    self.pattern_scroll = 0;
                }

                let filtered: Vec<usize> = lib
                    .iter()
                    .enumerate()
                    .filter(|(_, q)| {
                        self.pattern_genre.is_none_or(|g| q.genre == g)
                            && self.pattern_role.is_none_or(|r| q.role == r)
                            && self.pattern_key.is_none_or(|k| q.key == k)
                    })
                    .map(|(i, _)| i)
                    .collect();
                if !filtered.contains(&self.pattern_sel) {
                    if let Some(&first) = filtered.first() {
                        self.pattern_sel = first;
                    }
                }

                let bar_h = 62.0;
                let card_h = 44.0;
                let list_bottom = content.bottom() - bar_h - 6.0;
                let visible = (((list_bottom - y) / card_h).floor() as usize).max(1);
                let max_scroll = filtered.len().saturating_sub(visible);
                let list_rect = Rect::from_min_max(
                    Pos2::new(content.left(), y),
                    Pos2::new(content.right(), list_bottom.max(y + 1.0)),
                );
                let hovered = ui
                    .ctx()
                    .input(|i| i.pointer.hover_pos())
                    .map(|pos| list_rect.contains(pos))
                    .unwrap_or(false);
                if hovered {
                    let dy = ui.input(|i| i.smooth_scroll_delta.y);
                    if dy < -0.1 {
                        self.pattern_scroll = (self.pattern_scroll + 1).min(max_scroll);
                    } else if dy > 0.1 {
                        self.pattern_scroll = self.pattern_scroll.saturating_sub(1);
                    }
                }
                self.pattern_scroll = self.pattern_scroll.min(max_scroll);

                if filtered.is_empty() {
                    txt(
                        &p,
                        Pos2::new(content.left(), y + 6.0),
                        Align2::LEFT_TOP,
                        "No patterns match those filters.",
                        9.5,
                        DIM,
                    );
                }

                let first = self.pattern_scroll.min(filtered.len());
                let last = (first + visible).min(filtered.len());
                let mut yy = y;
                for &idx in &filtered[first..last] {
                    let pat = &lib[idx];
                    let r = Rect::from_min_size(
                        Pos2::new(content.left(), yy),
                        Vec2::new(content.width(), card_h - 4.0),
                    );
                    let resp = ui.interact(r, Id::unique(("patcard", idx)), Sense::click());
                    let selected = self.pattern_sel == idx;
                    let col = c(pat.instrument.color());
                    rounded(
                        &p,
                        r,
                        3.0,
                        if selected {
                            mix(PANEL2, ACCENT, 0.16)
                        } else if resp.hovered() {
                            PANEL2
                        } else {
                            RGBA_PANEL
                        },
                    );
                    outline(
                        &p,
                        r,
                        3.0,
                        if selected { ACCENT_DARK } else { LINE_SOFT },
                        1.0,
                    );
                    p.rect_filled(
                        Rect::from_min_size(r.min, Vec2::new(2.0, r.height())),
                        1.0,
                        col,
                    );

                    let play_r = Rect::from_min_size(
                        Pos2::new(r.left() + 5.0, r.top() + 7.0),
                        Vec2::new(20.0, 20.0),
                    );
                    let play_resp =
                        ui.interact(play_r, Id::unique(("patplay", idx)), Sense::click());
                    let is_playing = selected && self.time < self.pattern_preview_until;
                    p.circle_filled(
                        play_r.center(),
                        8.0,
                        if is_playing { ACCENT } else { PANEL3 },
                    );
                    let pc = play_r.center();
                    if is_playing {
                        p.rect_filled(Rect::from_center_size(pc, Vec2::splat(6.0)), 1.0, BG);
                    } else {
                        p.add(egui::Shape::convex_polygon(
                            vec![
                                Pos2::new(pc.x - 2.5, pc.y - 5.0),
                                Pos2::new(pc.x + 6.0, pc.y),
                                Pos2::new(pc.x - 2.5, pc.y + 5.0),
                            ],
                            MINT,
                            egui::Stroke::new(0.0, Color32::TRANSPARENT),
                        ));
                    }

                    txt(
                        &p,
                        Pos2::new(r.left() + 30.0, r.top() + 4.0),
                        Align2::LEFT_TOP,
                        fit(&pat.name, r.width() - 104.0, 11.0),
                        11.0,
                        if selected { TEXT } else { mix(TEXT, DIM, 0.2) },
                    );
                    mono(
                        &p,
                        Pos2::new(r.right() - 8.0, r.top() + 5.0),
                        Align2::RIGHT_TOP,
                        format!("{} {}", pat.key_name(), pat.scale.label()),
                        7.0,
                        alpha(col, 230),
                    );
                    mono(
                        &p,
                        Pos2::new(r.left() + 30.0, r.top() + 21.0),
                        Align2::LEFT_TOP,
                        format!(
                            "{} · {} · {:.0} BPM",
                            pat.genre.short(),
                            pat.role.label(),
                            pat.bpm()
                        ),
                        7.0,
                        FAINT,
                    );
                    mono(
                        &p,
                        Pos2::new(r.right() - 8.0, r.top() + 21.0),
                        Align2::RIGHT_TOP,
                        pat.instrument.short(),
                        7.0,
                        FAINT,
                    );

                    if resp.clicked() {
                        self.pattern_sel = idx;
                    }
                    if resp.double_clicked() {
                        act = Some(Act::LoadPattern(idx));
                    }
                    if play_resp.clicked() {
                        self.pattern_sel = idx;
                        act = Some(Act::PreviewPattern(idx));
                    }
                    yy += card_h;
                }

                if filtered.len() > visible {
                    mono(
                        &p,
                        Pos2::new(content.right(), (list_bottom - 10.0).max(y + 1.0)),
                        Align2::RIGHT_BOTTOM,
                        format!(
                            "SHOWING {}-{} OF {} · SCROLL",
                            first + 1,
                            last,
                            filtered.len()
                        ),
                        6.5,
                        FAINT,
                    );
                }

                // Selected-pattern action bar, kept on screen while scrolling.
                let bar = Rect::from_min_size(
                    Pos2::new(content.left(), content.bottom() - bar_h + 2.0),
                    Vec2::new(content.width(), bar_h - 6.0),
                );
                rounded(&p, bar, 4.0, mix(PANEL2, ACCENT, 0.06));
                outline(&p, bar, 4.0, LINE_SOFT, 1.0);
                if let Some(pat) = lib.get(self.pattern_sel) {
                    txt(
                        &p,
                        Pos2::new(bar.left() + 8.0, bar.top() + 5.0),
                        Align2::LEFT_TOP,
                        fit(&pat.name, bar.width() - 16.0, 10.5),
                        10.5,
                        TEXT,
                    );
                    mono(
                        &p,
                        Pos2::new(bar.left() + 8.0, bar.top() + 19.0),
                        Align2::LEFT_TOP,
                        format!(
                            "{} · {} · {} {} · {} BARS",
                            pat.genre.label(),
                            pat.role.long_label().to_uppercase(),
                            pat.key_name(),
                            pat.scale.label(),
                            pat.bars
                        ),
                        6.5,
                        FAINT,
                    );
                    let bw = (bar.width() - 14.0) / 2.0;
                    let prev_r = Rect::from_min_size(
                        Pos2::new(bar.left() + 6.0, bar.bottom() - 27.0),
                        Vec2::new(bw, 22.0),
                    );
                    let load_r = Rect::from_min_size(
                        Pos2::new(prev_r.right() + 4.0, bar.bottom() - 27.0),
                        Vec2::new(bw, 22.0),
                    );
                    let previewing = self.time < self.pattern_preview_until;
                    if chip(
                        ui,
                        prev_r,
                        Id::unique("patpreview"),
                        if previewing {
                            "STOP PREVIEW"
                        } else {
                            "PREVIEW"
                        },
                        previewing,
                        MINT,
                    )
                    .clicked()
                    {
                        if previewing {
                            self.shared.send(Cmd::StopPreview);
                            self.pattern_preview_until = 0.0;
                        } else {
                            act = Some(Act::PreviewPattern(self.pattern_sel));
                        }
                    }
                    if chip(
                        ui,
                        load_r,
                        Id::unique("patload"),
                        "LOAD TRACK",
                        false,
                        ACCENT,
                    )
                    .clicked()
                    {
                        act = Some(Act::LoadPattern(self.pattern_sel));
                    }
                }
            }
            BrowserTab::Sounds => {
                let mut y = content.top();
                section(&p, Pos2::new(content.left(), y), "SOUND BROWSER");
                mono(
                    &p,
                    Pos2::new(content.right(), y),
                    Align2::RIGHT_TOP,
                    format!("{} SOUNDS", sounds::count()),
                    7.0,
                    FAINT,
                );
                y += 18.0;

                let modes = [
                    ("SOUNDS", LibraryMode::Sounds),
                    ("PLUGINS", LibraryMode::Plugins),
                    ("DEVICES", LibraryMode::Devices),
                ];
                let mw = (content.width() - 8.0) / 3.0;
                for (i, (label, mode)) in modes.iter().enumerate() {
                    let r = Rect::from_min_size(
                        Pos2::new(content.left() + i as f32 * (mw + 4.0), y),
                        Vec2::new(mw, 21.0),
                    );
                    if chip(
                        ui,
                        r,
                        Id::unique(("libmode", i)),
                        label,
                        self.library_mode == *mode,
                        ACCENT,
                    )
                    .clicked()
                    {
                        self.library_mode = *mode;
                    }
                }
                y += 27.0;

                match self.library_mode {
                    LibraryMode::Sounds => {
                        let cats: Vec<Option<SoundCategory>> = std::iter::once(None)
                            .chain(SoundCategory::ALL.iter().copied().map(Some))
                            .collect();
                        let cw = (content.width() - 8.0) / 3.0;
                        for (i, cat) in cats.iter().enumerate() {
                            let r = Rect::from_min_size(
                                Pos2::new(
                                    content.left() + (i % 3) as f32 * (cw + 4.0),
                                    y + (i / 3) as f32 * 20.0,
                                ),
                                Vec2::new(cw, 18.0),
                            );
                            let label = cat.map(|c| c.label()).unwrap_or("ALL");
                            if chip(
                                ui,
                                r,
                                Id::unique(("sndcat", i)),
                                label,
                                self.sound_category == *cat,
                                MINT,
                            )
                            .clicked()
                            {
                                self.sound_category = *cat;
                                self.sound_scroll = 0;
                            }
                        }
                        y += cats.len().div_ceil(3) as f32 * 20.0 + 4.0;

                        let search_r = Rect::from_min_size(
                            Pos2::new(content.left(), y),
                            Vec2::new(content.width(), 24.0),
                        );
                        let search = ui.put(
                            search_r,
                            egui::TextEdit::singleline(&mut self.sound_search)
                                .hint_text("Search sounds")
                                .desired_width(search_r.width())
                                .text_color(TEXT),
                        );
                        if search.changed() {
                            self.sound_scroll = 0;
                        }
                        y += 28.0;

                        let filtered = self.listed_rows();
                        if self.sound_sel.is_none_or(|sel| !filtered.contains(&sel)) {
                            self.sound_sel = filtered.first().copied();
                        }

                        let bar_h = 76.0;
                        let card_h = 37.0;
                        let list_bottom = content.bottom() - bar_h - 6.0;
                        let visible = (((list_bottom - y) / card_h).floor() as usize).max(1);
                        let max_scroll = filtered.len().saturating_sub(visible);
                        let list_rect = Rect::from_min_max(
                            Pos2::new(content.left(), y),
                            Pos2::new(content.right(), list_bottom.max(y + 1.0)),
                        );
                        let hovered = ui
                            .ctx()
                            .input(|i| i.pointer.hover_pos())
                            .map(|pos| list_rect.contains(pos))
                            .unwrap_or(false);
                        if hovered {
                            let dy = ui.input(|i| i.smooth_scroll_delta.y);
                            if dy < -0.1 {
                                self.sound_scroll = (self.sound_scroll + 2).min(max_scroll);
                            } else if dy > 0.1 {
                                self.sound_scroll = self.sound_scroll.saturating_sub(2);
                            }
                        }
                        self.sound_scroll = self.sound_scroll.min(max_scroll);

                        if filtered.is_empty() {
                            txt(
                                &p,
                                Pos2::new(content.left(), y + 6.0),
                                Align2::LEFT_TOP,
                                "No sounds match that search.",
                                9.0,
                                DIM,
                            );
                        }

                        let first = self.sound_scroll.min(filtered.len());
                        let last = (first + visible).min(filtered.len());
                        let mut yy = y;
                        for &idx in &filtered[first..last] {
                            let sound = &sounds::library()[idx];
                            let r = Rect::from_min_size(
                                Pos2::new(content.left(), yy),
                                Vec2::new(content.width(), card_h - 3.0),
                            );
                            let resp = ui.interact(r, Id::unique(("sndcard", idx)), Sense::click());
                            let selected = self.sound_sel == Some(idx);
                            let col = c(sound.kind.color());
                            rounded(
                                &p,
                                r,
                                3.0,
                                if selected {
                                    mix(PANEL2, ACCENT, 0.16)
                                } else if resp.hovered() {
                                    PANEL2
                                } else {
                                    RGBA_PANEL
                                },
                            );
                            outline(
                                &p,
                                r,
                                3.0,
                                if selected { ACCENT_DARK } else { LINE_SOFT },
                                1.0,
                            );
                            p.rect_filled(
                                Rect::from_min_size(r.min, Vec2::new(2.0, r.height())),
                                1.0,
                                col,
                            );
                            txt(
                                &p,
                                Pos2::new(r.left() + 8.0, r.top() + 4.0),
                                Align2::LEFT_TOP,
                                fit(&sound.name, r.width() - 54.0, 10.5),
                                10.5,
                                if selected { TEXT } else { mix(TEXT, DIM, 0.2) },
                            );
                            mono(
                                &p,
                                Pos2::new(r.right() - 6.0, r.top() + 5.0),
                                Align2::RIGHT_TOP,
                                sound.kind.short(),
                                6.5,
                                alpha(col, 220),
                            );
                            mono(
                                &p,
                                Pos2::new(r.left() + 8.0, r.top() + 20.0),
                                Align2::LEFT_TOP,
                                format!("{} · {}", sound.category.label(), sound.character),
                                6.5,
                                FAINT,
                            );
                            if resp.clicked() {
                                self.sound_sel = Some(idx);
                            }
                            if resp.double_clicked() {
                                act = Some(Act::LoadSound(idx));
                            }
                            yy += card_h;
                        }

                        if filtered.len() > visible {
                            mono(
                                &p,
                                Pos2::new(content.right(), (list_bottom - 8.0).max(y + 1.0)),
                                Align2::RIGHT_BOTTOM,
                                format!("{}-{} OF {} · SCROLL", first + 1, last, filtered.len()),
                                6.0,
                                FAINT,
                            );
                        }

                        let bar = Rect::from_min_size(
                            Pos2::new(content.left(), content.bottom() - bar_h + 2.0),
                            Vec2::new(content.width(), bar_h - 6.0),
                        );
                        rounded(&p, bar, 4.0, mix(PANEL2, ACCENT, 0.06));
                        outline(&p, bar, 4.0, LINE_SOFT, 1.0);
                        if let Some(idx) = self.sound_sel {
                            if let Some(sound) = sounds::library().get(idx) {
                                txt(
                                    &p,
                                    Pos2::new(bar.left() + 8.0, bar.top() + 5.0),
                                    Align2::LEFT_TOP,
                                    fit(&sound.name, bar.width() - 16.0, 10.5),
                                    10.5,
                                    TEXT,
                                );
                                mono(
                                    &p,
                                    Pos2::new(bar.left() + 8.0, bar.top() + 20.0),
                                    Align2::LEFT_TOP,
                                    format!(
                                        "{} · {} · {} PARAMS",
                                        sound.kind.name(),
                                        sound.family,
                                        sound.values.len()
                                    ),
                                    6.0,
                                    FAINT,
                                );
                                let load_r = Rect::from_min_size(
                                    Pos2::new(bar.left() + 6.0, bar.bottom() - 27.0),
                                    Vec2::new(bar.width() - 12.0, 22.0),
                                );
                                if chip(
                                    ui,
                                    load_r,
                                    Id::unique("sndload"),
                                    "LOAD SOUND ON TRACK",
                                    false,
                                    MINT,
                                )
                                .clicked()
                                {
                                    act = Some(Act::LoadSound(idx));
                                }
                            }
                        }
                    }
                    LibraryMode::Plugins => {
                        let cats: Vec<Option<PluginCategory>> = std::iter::once(None)
                            .chain(PluginCategory::ALL.iter().copied().map(Some))
                            .collect();
                        let cw = (content.width() - 8.0) / 3.0;
                        for (i, cat) in cats.iter().enumerate() {
                            let r = Rect::from_min_size(
                                Pos2::new(
                                    content.left() + (i % 3) as f32 * (cw + 4.0),
                                    y + (i / 3) as f32 * 19.0,
                                ),
                                Vec2::new(cw, 17.0),
                            );
                            let label = cat.map(|c| c.label()).unwrap_or("ALL");
                            if chip(
                                ui,
                                r,
                                Id::unique(("plugcat", i)),
                                label,
                                self.plugin_category == *cat,
                                BLUE,
                            )
                            .clicked()
                            {
                                self.plugin_category = *cat;
                                self.plugin_scroll = 0;
                            }
                        }
                        y += cats.len().div_ceil(3) as f32 * 19.0 + 4.0;

                        let search_r = Rect::from_min_size(
                            Pos2::new(content.left(), y),
                            Vec2::new(content.width(), 24.0),
                        );
                        let search = ui.put(
                            search_r,
                            egui::TextEdit::singleline(&mut self.plugin_search)
                                .hint_text("Search plugin racks")
                                .desired_width(search_r.width())
                                .text_color(TEXT),
                        );
                        if search.changed() {
                            self.plugin_scroll = 0;
                        }
                        y += 28.0;

                        let filtered = self.listed_rows();
                        if self.plugin_sel.is_none_or(|sel| !filtered.contains(&sel)) {
                            self.plugin_sel = filtered.first().copied();
                        }

                        let bar_h = 76.0;
                        let card_h = 37.0;
                        let list_bottom = content.bottom() - bar_h - 6.0;
                        let visible = (((list_bottom - y) / card_h).floor() as usize).max(1);
                        let max_scroll = filtered.len().saturating_sub(visible);
                        let list_rect = Rect::from_min_max(
                            Pos2::new(content.left(), y),
                            Pos2::new(content.right(), list_bottom.max(y + 1.0)),
                        );
                        let hovered = ui
                            .ctx()
                            .input(|i| i.pointer.hover_pos())
                            .map(|pos| list_rect.contains(pos))
                            .unwrap_or(false);
                        if hovered {
                            let dy = ui.input(|i| i.smooth_scroll_delta.y);
                            if dy < -0.1 {
                                self.plugin_scroll = (self.plugin_scroll + 2).min(max_scroll);
                            } else if dy > 0.1 {
                                self.plugin_scroll = self.plugin_scroll.saturating_sub(2);
                            }
                        }
                        self.plugin_scroll = self.plugin_scroll.min(max_scroll);

                        if filtered.is_empty() {
                            txt(
                                &p,
                                Pos2::new(content.left(), y + 6.0),
                                Align2::LEFT_TOP,
                                "No plugin racks match that search.",
                                9.0,
                                DIM,
                            );
                        }

                        let first = self.plugin_scroll.min(filtered.len());
                        let last = (first + visible).min(filtered.len());
                        let mut yy = y;
                        for &idx in &filtered[first..last] {
                            let rack = &plugins::library()[idx];
                            let r = Rect::from_min_size(
                                Pos2::new(content.left(), yy),
                                Vec2::new(content.width(), card_h - 3.0),
                            );
                            let resp =
                                ui.interact(r, Id::unique(("plugcard", idx)), Sense::click());
                            let selected = self.plugin_sel == Some(idx);
                            let col = c(rack.category.color());
                            rounded(
                                &p,
                                r,
                                3.0,
                                if selected {
                                    mix(PANEL2, ACCENT, 0.16)
                                } else if resp.hovered() {
                                    PANEL2
                                } else {
                                    RGBA_PANEL
                                },
                            );
                            outline(
                                &p,
                                r,
                                3.0,
                                if selected { ACCENT_DARK } else { LINE_SOFT },
                                1.0,
                            );
                            p.rect_filled(
                                Rect::from_min_size(r.min, Vec2::new(2.0, r.height())),
                                1.0,
                                col,
                            );
                            txt(
                                &p,
                                Pos2::new(r.left() + 8.0, r.top() + 4.0),
                                Align2::LEFT_TOP,
                                fit(&rack.name, r.width() - 54.0, 10.5),
                                10.5,
                                if selected { TEXT } else { mix(TEXT, DIM, 0.2) },
                            );
                            mono(
                                &p,
                                Pos2::new(r.right() - 6.0, r.top() + 5.0),
                                Align2::RIGHT_TOP,
                                format!("{} FX", rack.devices.len()),
                                6.5,
                                alpha(col, 220),
                            );
                            mono(
                                &p,
                                Pos2::new(r.left() + 8.0, r.top() + 20.0),
                                Align2::LEFT_TOP,
                                format!(
                                    "{} · {}",
                                    rack.category.label(),
                                    rack.devices[0].kind.short()
                                ),
                                6.5,
                                FAINT,
                            );
                            if resp.clicked() {
                                self.plugin_sel = Some(idx);
                            }
                            if resp.double_clicked() {
                                act = Some(Act::AddPluginRack(idx));
                            }
                            yy += card_h;
                        }

                        if filtered.len() > visible {
                            mono(
                                &p,
                                Pos2::new(content.right(), (list_bottom - 8.0).max(y + 1.0)),
                                Align2::RIGHT_BOTTOM,
                                format!("{}-{} OF {} · SCROLL", first + 1, last, filtered.len()),
                                6.0,
                                FAINT,
                            );
                        }

                        let bar = Rect::from_min_size(
                            Pos2::new(content.left(), content.bottom() - bar_h + 2.0),
                            Vec2::new(content.width(), bar_h - 6.0),
                        );
                        rounded(&p, bar, 4.0, mix(PANEL2, ACCENT, 0.06));
                        outline(&p, bar, 4.0, LINE_SOFT, 1.0);
                        if let Some(idx) = self.plugin_sel {
                            if let Some(rack) = plugins::library().get(idx) {
                                txt(
                                    &p,
                                    Pos2::new(bar.left() + 8.0, bar.top() + 5.0),
                                    Align2::LEFT_TOP,
                                    fit(&rack.name, bar.width() - 16.0, 10.5),
                                    10.5,
                                    TEXT,
                                );
                                mono(
                                    &p,
                                    Pos2::new(bar.left() + 8.0, bar.top() + 20.0),
                                    Align2::LEFT_TOP,
                                    fit(&rack.summary(), bar.width() - 16.0, 6.0),
                                    6.0,
                                    FAINT,
                                );
                                let load_r = Rect::from_min_size(
                                    Pos2::new(bar.left() + 6.0, bar.bottom() - 27.0),
                                    Vec2::new(bar.width() - 12.0, 22.0),
                                );
                                if chip(
                                    ui,
                                    load_r,
                                    Id::unique("plugload"),
                                    "INSERT PLUGIN RACK",
                                    false,
                                    BLUE,
                                )
                                .clicked()
                                {
                                    act = Some(Act::AddPluginRack(idx));
                                }
                            }
                        }
                    }
                    LibraryMode::Devices => {
                        let row_h = 17.0;
                        section(&p, Pos2::new(content.left(), y), "INSTRUMENTS");
                        y += 18.0;
                        let sel_kind = self
                            .song
                            .tracks
                            .get(self.sel_track)
                            .map(|t| t.instrument.kind);
                        for (i, kind) in DeviceKind::ALL
                            .iter()
                            .enumerate()
                            .filter(|(_, k)| k.is_instrument())
                        {
                            let r = Rect::from_min_size(
                                Pos2::new(content.left(), y),
                                Vec2::new(content.width(), row_h),
                            );
                            let resp = ui.interact(r, Id::unique(("inst", i)), Sense::click());
                            if resp.hovered() {
                                rounded(&p, r, 3.0, PANEL2);
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            if sel_kind == Some(*kind) {
                                p.rect_filled(
                                    Rect::from_min_size(r.min, Vec2::new(2.0, r.height())),
                                    1.0,
                                    ACCENT,
                                );
                            }
                            let col = c(kind.color());
                            p.circle_filled(Pos2::new(r.left() + 12.0, r.center().y), 3.5, col);
                            txt(
                                &p,
                                Pos2::new(r.left() + 23.0, r.center().y),
                                Align2::LEFT_CENTER,
                                kind.name(),
                                11.0,
                                if resp.hovered() {
                                    TEXT
                                } else {
                                    mix(TEXT, DIM, 0.25)
                                },
                            );
                            txt(
                                &p,
                                Pos2::new(r.right() - 6.0, r.center().y),
                                Align2::RIGHT_CENTER,
                                "+",
                                12.0,
                                if resp.hovered() { ACCENT } else { FAINT },
                            );
                            if resp.clicked() {
                                act = Some(Act::SetInstrument {
                                    track: self.sel_track,
                                    kind: *kind,
                                });
                            }
                            y += row_h;
                        }
                        y += 12.0;
                        section(&p, Pos2::new(content.left(), y), "EFFECTS");
                        y += 18.0;
                        for (i, kind) in DeviceKind::ALL
                            .iter()
                            .enumerate()
                            .filter(|(_, k)| !k.is_instrument())
                        {
                            let r = Rect::from_min_size(
                                Pos2::new(content.left(), y),
                                Vec2::new(content.width(), row_h),
                            );
                            let resp = ui.interact(r, Id::unique(("fx", i)), Sense::click());
                            if resp.hovered() {
                                rounded(&p, r, 3.0, PANEL2);
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                            let col = c(kind.color());
                            p.circle_filled(Pos2::new(r.left() + 12.0, r.center().y), 3.0, col);
                            txt(
                                &p,
                                Pos2::new(r.left() + 23.0, r.center().y),
                                Align2::LEFT_CENTER,
                                kind.name(),
                                10.5,
                                if resp.hovered() { TEXT } else { DIM },
                            );
                            txt(
                                &p,
                                Pos2::new(r.right() - 6.0, r.center().y),
                                Align2::RIGHT_CENTER,
                                "+",
                                11.0,
                                if resp.hovered() { ACCENT } else { FAINT },
                            );
                            if resp.clicked() {
                                act = Some(Act::AddEffect {
                                    track: self.sel_track,
                                    kind: *kind,
                                });
                            }
                            y += row_h;
                        }
                        let add_y = (y + 4.0).min(content.bottom() - 30.0);
                        let add_r = Rect::from_min_size(
                            Pos2::new(content.left(), add_y),
                            Vec2::new(content.width(), 28.0),
                        );
                        let ar = ui.interact(add_r, Id::unique("addtrack"), Sense::click());
                        rounded(&p, add_r, 3.0, if ar.hovered() { PANEL3 } else { PANEL2 });
                        outline(&p, add_r, 3.0, LINE, 1.0);
                        txt(
                            &p,
                            add_r.center(),
                            Align2::CENTER_CENTER,
                            "＋  NEW DRUM TRACK",
                            9.5,
                            MINT,
                        );
                        if ar.clicked() {
                            act = Some(Act::AddTrack(DeviceKind::DrumRack));
                        }
                    }
                }
            }
            BrowserTab::Demos => {
                let mut y = content.top();
                let demo_label = format!("FACTORY DEMOS · {}", DEMOS.len());
                section(&p, Pos2::new(content.left(), y), &demo_label);
                y += 20.0;
                let gap = 3.0;
                let count = DEMOS.len().max(1) as f32;
                let card_h =
                    ((content.height() - 48.0 - gap * (count - 1.0)) / count).clamp(54.0, 64.0);
                for (i, demo) in DEMOS.iter().enumerate() {
                    let r = Rect::from_min_size(
                        Pos2::new(content.left(), y),
                        Vec2::new(content.width(), card_h),
                    );
                    let resp = ui.interact(r, Id::unique(("demo", i)), Sense::click());
                    rounded(&p, r, 4.0, if resp.hovered() { PANEL2 } else { RGBA_PANEL });
                    outline(
                        &p,
                        r,
                        4.0,
                        if resp.hovered() {
                            ACCENT_DARK
                        } else {
                            LINE_SOFT
                        },
                        1.0,
                    );
                    txt(
                        &p,
                        Pos2::new(r.left() + 10.0, r.top() + 6.0),
                        Align2::LEFT_TOP,
                        demo.tag,
                        8.0,
                        ACCENT,
                    );
                    mono(
                        &p,
                        Pos2::new(r.right() - 10.0, r.top() + 6.0),
                        Align2::RIGHT_TOP,
                        format!("{:.0} BPM", demo.bpm),
                        8.5,
                        FAINT,
                    );
                    txt(
                        &p,
                        Pos2::new(r.left() + 10.0, r.top() + 17.0),
                        Align2::LEFT_TOP,
                        demo.name,
                        13.0,
                        TEXT,
                    );
                    txt(
                        &p,
                        Pos2::new(r.left() + 10.0, r.top() + 34.0),
                        Align2::LEFT_TOP,
                        fit(demo.blurb, r.width() - 20.0, 9.5),
                        9.5,
                        DIM,
                    );
                    txt(
                        &p,
                        Pos2::new(r.left() + 10.0, r.bottom() - 5.0),
                        Align2::LEFT_BOTTOM,
                        "LOAD →",
                        8.0,
                        if resp.hovered() { MINT } else { FAINT },
                    );
                    if resp.clicked() {
                        act = Some(Act::LoadDemo(i));
                    }
                    y += card_h + gap;
                }
                let hint_y = (y + 2.0).min(content.bottom() - 18.0);
                txt(
                    &p,
                    Pos2::new(content.left(), hint_y),
                    Align2::LEFT_TOP,
                    "Demos replace the current project. Save yours first.",
                    9.0,
                    FAINT,
                );
            }
            BrowserTab::Files => {
                let mut y = content.top();
                section(&p, Pos2::new(content.left(), y), "PROJECT FILES");
                y += 20.0;
                let dir = project::projects_dir();
                txt(
                    &p,
                    Pos2::new(content.left(), y),
                    Align2::LEFT_TOP,
                    fit(&dir.display().to_string(), content.width(), 8.5),
                    8.5,
                    FAINT,
                );
                y += 18.0;

                let bw = (content.width() - 6.0) / 2.0;
                let save_r = Rect::from_min_size(Pos2::new(content.left(), y), Vec2::new(bw, 28.0));
                let exp_r = Rect::from_min_size(
                    Pos2::new(content.left() + bw + 6.0, y),
                    Vec2::new(bw, 28.0),
                );
                if chip(
                    ui,
                    save_r,
                    Id::unique("fsave"),
                    "SAVE PROJECT",
                    false,
                    ACCENT,
                )
                .clicked()
                {
                    act = Some(Act::Save);
                }
                if chip(ui, exp_r, Id::unique("fexp"), "EXPORT WAV", false, MINT).clicked() {
                    act = Some(Act::Export);
                }
                y += 34.0;
                let new_r = Rect::from_min_size(Pos2::new(content.left(), y), Vec2::new(bw, 26.0));
                let ref_r = Rect::from_min_size(
                    Pos2::new(content.left() + bw + 6.0, y),
                    Vec2::new(bw, 26.0),
                );
                if chip(ui, new_r, Id::unique("fnew"), "NEW PROJECT", false, DIM).clicked() {
                    act = Some(Act::New);
                }
                if chip(ui, ref_r, Id::unique("fref"), "REFRESH LIST", false, DIM).clicked() {
                    self.files = project::list_projects();
                }
                y += 38.0;

                txt(
                    &p,
                    Pos2::new(content.left(), y),
                    Align2::LEFT_TOP,
                    "RECENT",
                    8.0,
                    FAINT,
                );
                y += 16.0;
                let files = self.files.clone();
                if files.is_empty() {
                    txt(
                        &p,
                        Pos2::new(content.left(), y + 4.0),
                        Align2::LEFT_TOP,
                        "No saved projects yet.",
                        9.5,
                        DIM,
                    );
                    y += 24.0;
                }
                for (i, f) in files.iter().take(9).enumerate() {
                    let r = Rect::from_min_size(
                        Pos2::new(content.left(), y),
                        Vec2::new(content.width(), 26.0),
                    );
                    let resp = ui.interact(r, Id::unique(("file", i)), Sense::click());
                    if resp.hovered() {
                        rounded(&p, r, 3.0, PANEL2);
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    txt(
                        &p,
                        Pos2::new(r.left() + 6.0, r.center().y),
                        Align2::LEFT_CENTER,
                        fit(&f.name, r.width() - 26.0, 10.0),
                        10.0,
                        if resp.hovered() { TEXT } else { DIM },
                    );
                    txt(
                        &p,
                        Pos2::new(r.right() - 6.0, r.center().y),
                        Align2::RIGHT_CENTER,
                        "▸",
                        9.0,
                        FAINT,
                    );
                    if resp.clicked() {
                        act = Some(Act::LoadFile(f.path.clone()));
                    }
                    y += 25.0;
                }

                let info_y = (content.bottom() - 76.0).max(y + 6.0);
                hline(&p, content.left(), content.right(), info_y, LINE_SOFT);
                txt(
                    &p,
                    Pos2::new(content.left(), info_y + 8.0),
                    Align2::LEFT_TOP,
                    "OUTPUT",
                    8.0,
                    FAINT,
                );
                txt(
                    &p,
                    Pos2::new(content.left(), info_y + 21.0),
                    Align2::LEFT_TOP,
                    fit(&self.device_name, content.width(), 9.5),
                    9.5,
                    if self.audio.is_some() { DIM } else { RED },
                );
                txt(
                    &p,
                    Pos2::new(content.left(), info_y + 35.0),
                    Align2::LEFT_TOP,
                    format!(
                        "{:.1} kHz · {:.0} ms buffer target",
                        self.sr as f32 / 1000.0,
                        if self.audio.is_some() {
                            256.0 / self.sr as f32 * 1000.0
                        } else {
                            0.0
                        }
                    ),
                    8.5,
                    FAINT,
                );
                if self.audio.is_none() {
                    let fix_r = Rect::from_min_size(
                        Pos2::new(content.left(), info_y + 50.0),
                        Vec2::new(content.width(), 22.0),
                    );
                    if chip(ui, fix_r, Id::unique("fixaudio2"), "RETRY AUDIO", true, RED).clicked()
                    {
                        act = Some(Act::RestartAudio);
                    }
                }
            }
        }
        if let Some(a) = act {
            self.act(a);
        }
    }

    fn focus_detail_or_main(&mut self) {
        self.view = if self.clip_open {
            View::Piano
        } else if self.device_open {
            View::Device
        } else {
            self.main_view
        };
    }

    /// Change the workspace behind the detail stack. If Clip or Device is open,
    /// the detail views stay open, just like Live's Session/Arrangement switch.
    pub fn show_main(&mut self, view: View) {
        debug_assert!(matches!(view, View::Session | View::Arrangement));
        self.main_view = view;
        if self.view == View::Mixer {
            self.focus_detail_or_main();
        } else if self.detail_open() {
            if !self.view.is_detail() {
                self.focus_detail_or_main();
            }
        } else {
            self.view = view;
        }
    }

    pub fn open_detail(&mut self, view: View) {
        match view {
            View::Piano => {
                self.clip_open = true;
                self.view = View::Piano;
            }
            View::Device => {
                self.device_open = true;
                self.view = View::Device;
            }
            _ => {}
        }
    }

    pub fn close_clip(&mut self) {
        self.clip_open = false;
        if self.device_open {
            self.view = View::Device;
        } else {
            self.view = self.main_view;
        }
    }

    pub fn close_device(&mut self) {
        self.device_open = false;
        if self.clip_open {
            self.view = View::Piano;
        } else {
            self.view = self.main_view;
        }
    }

    pub fn toggle_clip(&mut self) {
        if self.clip_open {
            if self.view == View::Mixer {
                self.view = View::Piano;
            } else {
                self.close_clip();
            }
        } else {
            self.open_detail(View::Piano);
        }
    }

    pub fn toggle_device(&mut self) {
        if self.device_open {
            if self.view == View::Mixer {
                self.view = View::Device;
            } else {
                self.close_device();
            }
        } else {
            self.open_detail(View::Device);
        }
    }

    pub fn toggle_mixer(&mut self) {
        if self.view == View::Mixer {
            self.focus_detail_or_main();
        } else {
            self.view = View::Mixer;
        }
    }

    pub fn detail_open(&self) -> bool {
        self.clip_open || self.device_open
    }

    fn show_main_view(&mut self, ui: &mut Ui, rect: Rect, view: View) {
        match view {
            View::Session => view_session::show(self, ui, rect),
            View::Arrangement => view_arrangement::show(self, ui, rect),
            View::Mixer => view_mixer::show(self, ui, rect),
            _ => {}
        }
    }

    /// Live 12 places the Session/Arrangement selector in the top-right
    /// corner of the Control Bar.
    fn view_switch(&mut self, ui: &mut Ui, rect: Rect) {
        let p = ui.painter_at(rect);

        let widths = [64.0, 100.0];
        let mut x = rect.right() - widths[0] - widths[1] - 4.0;
        for (i, view) in [View::Session, View::Arrangement].iter().enumerate() {
            let w = widths[i];
            let r = Rect::from_min_size(Pos2::new(x, rect.top() + 5.0), Vec2::new(w, 24.0));
            let active = self.main_view == *view;
            let resp = ui.interact(r, Id::unique(("mainview", i)), Sense::click());
            if active {
                rounded(&p, r, 3.0, PANEL2);
                p.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(r.left(), r.bottom() - 2.0),
                        Vec2::new(r.width(), 2.0),
                    ),
                    1.0,
                    ACCENT,
                );
            } else if resp.hovered() {
                rounded(&p, r, 3.0, PANEL2);
            }
            txt(
                &p,
                r.center(),
                Align2::CENTER_CENTER,
                view.label(),
                8.5,
                if active { TEXT } else { DIM },
            );
            if resp.clicked() {
                self.show_main(*view);
            }
            x += w + 4.0;
        }
    }

    /// Outer-bottom view controls, matching Live's bottom-right Clip, Device
    /// and Mixer selectors. Clip and Device can both be active at once.
    fn view_controls(&mut self, ui: &mut Ui, rect: Rect) {
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 0.0, PANEL);
        hline(&p, rect.left(), rect.right(), rect.top(), LINE);

        let accent = self
            .song
            .tracks
            .get(self.sel_track)
            .map(|t| c(t.color))
            .unwrap_or(ACCENT);
        p.rect_filled(
            Rect::from_min_size(rect.min, Vec2::new(3.0, rect.height())),
            0.0,
            accent,
        );

        let clip_name = self
            .clip_edit
            .and_then(|cr| crate::app::clip_at(&self.song, &cr).map(|clip| clip.name.clone()));
        let state = if self.view == View::Mixer {
            "MIXER".to_string()
        } else if self.clip_open && self.device_open {
            "CLIP + DEVICE · STACKED".to_string()
        } else if self.clip_open {
            clip_name
                .map(|name| format!("CLIP · {name}"))
                .unwrap_or_else(|| "CLIP · NO CLIP SELECTED".to_string())
        } else if self.device_open {
            let track = self
                .song
                .tracks
                .get(self.sel_track)
                .map(|t| t.name.to_uppercase())
                .unwrap_or_else(|| "NO TRACK".to_string());
            format!("DEVICE · {track}")
        } else {
            "CLIP AND DEVICE VIEWS".to_string()
        };

        let clip_w = 62.0;
        let device_w = 76.0;
        let mix_w = 78.0;
        let gap = 6.0;
        let total = clip_w + device_w + mix_w + gap * 2.0;
        txt(
            &p,
            Pos2::new(rect.left() + 14.0, rect.center().y),
            Align2::LEFT_CENTER,
            fit(&state, (rect.width() - total - 42.0).max(80.0), 8.0),
            8.0,
            FAINT,
        );

        let y = rect.top() + 4.0;
        let mut x = rect.right() - 14.0 - total;
        let clip_r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(clip_w, 22.0));
        if chip(
            ui,
            clip_r,
            Id::unique("view_clip"),
            "3  CLIP",
            self.clip_open,
            ACCENT,
        )
        .clicked()
        {
            self.toggle_clip();
        }
        x += clip_w + gap;
        let device_r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(device_w, 22.0));
        if chip(
            ui,
            device_r,
            Id::unique("view_device"),
            "4  DEVICE",
            self.device_open,
            MINT,
        )
        .clicked()
        {
            self.toggle_device();
        }
        x += device_w + gap;
        let mix_r = Rect::from_min_size(Pos2::new(x, y), Vec2::new(mix_w, 22.0));
        if chip(
            ui,
            mix_r,
            Id::unique("view_mixer"),
            "5  MIXER",
            self.view == View::Mixer,
            BLUE,
        )
        .clicked()
        {
            self.toggle_mixer();
        }
    }

    fn status_bar(&mut self, ui: &mut Ui, rect: Rect) {
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 0.0, PANEL);
        hline(&p, rect.left(), rect.right(), rect.top(), LINE);
        txt(
            &p,
            Pos2::new(rect.left() + 14.0, rect.center().y),
            Align2::LEFT_CENTER,
            "SPACE play/stop  ·  1/2 main  ·  TAB session/arrangement  ·  3 clip  ·  4 device  ·  5 mixer  ·  A-K piano  ·  CTRL+S save",
            8.5,
            FAINT,
        );

        if self.time < self.status_until && !self.status.is_empty() {
            txt(
                &p,
                Pos2::new(rect.center().x, rect.center().y),
                Align2::CENTER_CENTER,
                fit(&self.status, rect.width() * 0.42, 9.0),
                9.0,
                MINT,
            );
        }

        let info_w = 420.0;
        let midi_label = fit(&self.midi_name, 150.0, 8.0);
        let right = if let Some(err) = &self.audio_error {
            let r = Rect::from_min_size(
                Pos2::new(rect.right() - info_w - 14.0, rect.top() + 4.0),
                Vec2::new(info_w, 18.0),
            );
            let resp = ui.interact(r, Id::unique("err_audio"), Sense::click());
            let msg = format!(
                "MIDI: {} · AUDIO ERROR · {} · click to retry",
                midi_label,
                fit(err, 120.0, 8.5)
            );
            txt(
                &p,
                Pos2::new(r.right(), rect.center().y),
                Align2::RIGHT_CENTER,
                fit(&msg, info_w, 8.5),
                8.5,
                RED,
            );
            if resp.clicked() {
                self.act(Act::RestartAudio);
            }
            r.left()
        } else {
            let msg = format!(
                "MIDI: {} · {} · {:.1} kHz · {} TRACKS",
                midi_label,
                self.device_name,
                self.sr as f32 / 1000.0,
                self.song.tracks.len()
            );
            txt(
                &p,
                Pos2::new(rect.right() - 14.0, rect.center().y),
                Align2::RIGHT_CENTER,
                fit(&msg, info_w, 8.0),
                8.0,
                if self.midi.is_some() { MINT } else { FAINT },
            );
            rect.right() - info_w - 14.0
        };
        let _ = right;
    }
}

pub const RGBA_PANEL: Color32 = Color32::from_rgb(21, 23, 29);

fn capture_log(dir: &Path, line: &str) {
    use std::io::Write;

    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("capture.log"))
        .and_then(|mut file| writeln!(file, "{line}"));
}

fn save_capture_bmp(path: &Path, image: &egui::ColorImage) -> std::io::Result<()> {
    let [width, height] = image.size;
    if width == 0 || height == 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "empty screenshot",
        ));
    }
    let row = width * 3;
    let padding = (4 - row % 4) % 4;
    let data_size = (row + padding) * height;
    let file_size = 54 + data_size;
    let mut out = BufWriter::new(File::create(path)?);
    out.write_all(b"BM")?;
    out.write_all(&(file_size as u32).to_le_bytes())?;
    out.write_all(&[0u8; 4])?;
    out.write_all(&54u32.to_le_bytes())?;
    out.write_all(&40u32.to_le_bytes())?;
    out.write_all(&(width as i32).to_le_bytes())?;
    out.write_all(&(height as i32).to_le_bytes())?;
    out.write_all(&1u16.to_le_bytes())?;
    out.write_all(&24u16.to_le_bytes())?;
    out.write_all(&0u32.to_le_bytes())?;
    out.write_all(&(data_size as u32).to_le_bytes())?;
    out.write_all(&2835i32.to_le_bytes())?;
    out.write_all(&2835i32.to_le_bytes())?;
    out.write_all(&0u32.to_le_bytes())?;
    out.write_all(&0u32.to_le_bytes())?;
    for y in (0..height).rev() {
        for x in 0..width {
            let pixel = image.pixels[y * width + x];
            out.write_all(&[pixel.b(), pixel.g(), pixel.r()])?;
        }
        out.write_all(&[0u8; 4][..padding])?;
    }
    out.flush()
}

pub fn clip_at<'a>(song: &'a Song, cr: &ClipRef) -> Option<&'a Clip> {
    let t = song.tracks.get(cr.track)?;
    if cr.scene >= 0 {
        t.session_clip(cr.scene)
    } else {
        t.clips
            .iter()
            .find(|c| c.scene < 0 && (c.start - cr.start).abs() < 0.001)
    }
}

pub fn clip_at_mut<'a>(song: &'a mut Song, cr: &ClipRef) -> Option<&'a mut Clip> {
    let t = song.tracks.get_mut(cr.track)?;
    if cr.scene >= 0 {
        t.session_clip_mut(cr.scene)
    } else {
        t.clips
            .iter_mut()
            .find(|c| c.scene < 0 && (c.start - cr.start).abs() < 0.001)
    }
}

impl eframe::App for HarpsiApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.time = ui.ctx().input(|i| i.time);
        self.poll_recording();
        self.poll_dropped_vocal(ui.ctx());
        let live_track = if self.song.tracks.is_empty() {
            0
        } else {
            self.sel_track.min(self.song.tracks.len() - 1)
        };
        self.shared.set_live_track(live_track);
        let stream_error = self
            .audio
            .as_ref()
            .and_then(|a| a.errors.try_lock().ok().and_then(|g| g.clone()));
        if let Some(e) = stream_error {
            self.audio_error = Some(e);
            self.flash("Audio stream error — click the error to restart audio.");
        }
        if self.dirty {
            self.publish_now();
        }
        let full = ui.max_rect();
        let p = ui.painter_at(full);
        p.rect_filled(full, 0.0, BG);

        let top = Rect::from_min_max(full.min, Pos2::new(full.right(), full.top() + TOP_H));
        self.top_bar(ui, top);

        let status = Rect::from_min_max(Pos2::new(full.left(), full.bottom() - STATUS_H), full.max);
        self.status_bar(ui, status);

        let browser_w = if self.browser_open { BROWSER_W } else { 0.0 };
        if self.browser_open {
            let browser = Rect::from_min_max(
                Pos2::new(full.left(), top.bottom()),
                Pos2::new(full.left() + BROWSER_W, status.top()),
            );
            self.browser(ui, browser);
        }

        let main = Rect::from_min_max(
            Pos2::new(full.left() + browser_w, top.bottom()),
            Pos2::new(full.right(), status.top()),
        );
        let controls =
            Rect::from_min_max(Pos2::new(main.left(), main.bottom() - VIEWBAR_H), main.max);
        let content = Rect::from_min_max(main.min, Pos2::new(main.right(), controls.top()));

        // Live 12 keeps Session/Arrangement as the workspace, stacks Clip and
        // Device in the lower detail area, and exposes all three view toggles
        // on the outer-bottom rail. Mixer is still a full-height subview.
        let detail = self.detail_open() && self.view != View::Mixer;
        let main_view = if self.view == View::Mixer {
            View::Mixer
        } else {
            self.main_view
        };

        if detail {
            let dock_h = (content.height() * 0.45)
                .clamp(300.0, 430.0)
                .min((content.height() - 180.0).max(160.0));
            let split = content.bottom() - dock_h;
            let main_rect = Rect::from_min_max(content.min, Pos2::new(content.right(), split));
            let dock = Rect::from_min_max(Pos2::new(content.left(), split), content.max);

            self.show_main_view(ui, main_rect, main_view);
            if self.clip_open && self.device_open {
                let mid = dock.left() + dock.width() * 0.5;
                let clip_rect = Rect::from_min_max(dock.min, Pos2::new(mid - 3.0, dock.bottom()));
                let device_rect = Rect::from_min_max(Pos2::new(mid + 3.0, dock.top()), dock.max);
                let p = ui.painter_at(dock);
                p.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(mid - 1.0, dock.top()),
                        Pos2::new(mid + 1.0, dock.bottom()),
                    ),
                    0.0,
                    LINE,
                );
                view_piano::show(self, ui, clip_rect);
                view_device::show_detail(self, ui, device_rect);
            } else if self.clip_open {
                view_piano::show(self, ui, dock);
            } else if self.device_open {
                view_device::show_detail(self, ui, dock);
            }
        } else {
            self.show_main_view(ui, content, main_view);
        }

        self.view_controls(ui, controls);
        self.handle_keys(ui);
        self.capture_tick(ui.ctx());
        ui.ctx().request_repaint_after(Duration::from_millis(30));
    }
}

#[cfg(test)]
mod browser_tests {
    use super::*;
    use hs_core::engine::Engine;
    use hs_core::song::Note;

    fn browser_rect() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(BROWSER_W, 860.0))
    }

    fn run_frame(ctx: &egui::Context, app: &mut HarpsiApp, events: Vec<egui::Event>) {
        let raw = egui::RawInput {
            screen_rect: Some(browser_rect()),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw, |ctx| {
            egui::Area::new(egui::Id::unique("headless-snds-browser"))
                .fixed_pos(Pos2::ZERO)
                .show(ctx, |ui| {
                    ui.set_min_size(browser_rect().size());
                    app.browser(ui, browser_rect());
                });
        });
        output.textures_delta.clear();
    }

    fn click_at(ctx: &egui::Context, app: &mut HarpsiApp, pos: Pos2) {
        run_frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
        run_frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                },
            ],
        );
        run_frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                },
            ],
        );
        run_frame(ctx, app, Vec::new());
    }

    fn type_text(ctx: &egui::Context, app: &mut HarpsiApp, text: &str) {
        run_frame(ctx, app, vec![egui::Event::Text(text.to_string())]);
        run_frame(ctx, app, Vec::new());
    }

    fn headless_app() -> HarpsiApp {
        let mut song = Song::empty("Headless browser", 128.0);
        song.add_track(DeviceKind::DrumRack);
        song.add_track(DeviceKind::SubBass);
        song.ensure_scenes(1);
        for track in 0..2 {
            let mut clip = Clip::new("Audition", 0, 0.0, 4.0);
            for i in 0..4 {
                clip.notes.push(Note::new(i as f64, 0.4, 36, 0.9));
            }
            song.tracks[track].clips.push(clip);
        }
        let (shared, _song_arc, _rx) = Shared::new(&song);
        HarpsiApp {
            song,
            shared,
            audio: None,
            audio_error: None,
            recorder: None,
            record_style: VocalStyle::Dubstep,
            vocal_style: VocalStyle::Dubstep,
            midi: None,
            midi_name: "No MIDI".to_string(),
            sr: 48000,
            device_name: "Headless".to_string(),
            view: View::Session,
            main_view: View::Session,
            clip_open: false,
            device_open: false,
            browser_open: true,
            browser: BrowserTab::Starters,
            sel_track: 0,
            sel_scene: 0,
            clip_edit: None,
            dev_sel: DevSel::Instrument,
            note_len: 0.25,
            note_vel: 0.9,
            sel_notes: BTreeSet::new(),
            sel_anchor: None,
            note_drag_origins: Vec::new(),
            note_clipboard: Vec::new(),
            piano_cursor: 0.0,
            launched: vec![None; 2],
            files: Vec::new(),
            drag: DragState::None,
            status: String::new(),
            status_until: 0.0,
            time: 0.0,
            dirty: false,
            keyboard_octave: 4,
            key_notes: [None; 13],
            capture: None,
            pattern_genre: None,
            pattern_role: None,
            pattern_key: None,
            pattern_sel: 0,
            pattern_scroll: 0,
            pattern_preview_until: 0.0,
            library_mode: LibraryMode::Sounds,
            sound_category: None,
            plugin_category: None,
            sound_search: String::new(),
            plugin_search: String::new(),
            sound_scroll: 0,
            plugin_scroll: 0,
            sound_sel: Some(0),
            plugin_sel: Some(0),
        }
    }

    #[test]
    fn snds_browser_modes_filters_load_and_insert_use_the_shipped_ui() {
        let ctx = egui::Context::default();
        let mut app = headless_app();
        run_frame(&ctx, &mut app, Vec::new());

        // Open the SNDS tab in the real browser layout.
        click_at(&ctx, &mut app, Pos2::new(125.0, 52.0));
        assert_eq!(app.browser, BrowserTab::Sounds);
        assert_eq!(app.library_mode, LibraryMode::Sounds);
        assert_eq!(app.listed_rows().len(), sounds::count());
        assert_eq!(app.listed_rows().len(), 1_152);

        // Each mode lists its own rows.
        click_at(&ctx, &mut app, Pos2::new(125.0, 102.5));
        assert_eq!(app.library_mode, LibraryMode::Plugins);
        assert_eq!(app.listed_rows().len(), plugins::count());
        assert_eq!(app.listed_rows().len(), 100);

        click_at(&ctx, &mut app, Pos2::new(200.34, 102.5));
        assert_eq!(app.library_mode, LibraryMode::Devices);
        assert_eq!(app.listed_rows().len(), DeviceKind::ALL.len());

        click_at(&ctx, &mut app, Pos2::new(49.66, 102.5));
        assert_eq!(app.library_mode, LibraryMode::Sounds);

        // Category chip narrows the sounds to the bass instruments.
        click_at(&ctx, &mut app, Pos2::new(200.34, 128.0));
        assert_eq!(app.sound_category, Some(SoundCategory::Bass));
        let bass_rows = app.listed_rows();
        assert_eq!(bass_rows.len(), 312);
        assert!(bass_rows
            .iter()
            .all(|&i| sounds::library()[i].category == SoundCategory::Bass));

        // Typing in the real search box narrows to one sound and syncs selection.
        click_at(&ctx, &mut app, Pos2::new(125.0, 195.0));
        type_text(&ctx, &mut app, "Riddim Wobble · Gritty");
        assert_eq!(app.sound_search, "Riddim Wobble · Gritty");
        let sound_idx = sounds::library()
            .iter()
            .position(|s| s.name == "Riddim Wobble · Gritty")
            .expect("sound in library");
        assert_eq!(app.listed_rows(), vec![sound_idx]);
        assert_eq!(app.sound_sel, Some(sound_idx));

        // Select the row, then activate LOAD in the bottom action bar.
        click_at(&ctx, &mut app, Pos2::new(125.0, 211.0 + 18.5));
        assert_eq!(app.sound_sel, Some(sound_idx));
        let sound = &sounds::library()[sound_idx];
        click_at(&ctx, &mut app, Pos2::new(125.0, 828.0));
        assert_eq!(app.song.tracks[0].instrument.kind, sound.kind);
        assert_eq!(app.song.tracks[0].instrument.params, sound.values);
        assert_eq!(app.song.tracks[0].name, sound.name);

        // The loaded track renders through the engine, proving the new sound is live.
        let mut audition = Song::empty("Library audition", 128.0);
        audition.tracks.push(app.song.tracks[0].clone());
        let audio = Engine::render_song(&audition, 48000.0);
        assert!(audio.iter().all(|s| s.is_finite()));
        let peak = audio.iter().fold(0.0f32, |a, s| a.max(s.abs()));
        assert!(peak > 0.01, "loaded sound rendered peak {peak}");

        // Plugin mode: filter, type, select and insert the whole rack.
        app.sel_track = 1;
        click_at(&ctx, &mut app, Pos2::new(125.0, 102.5));
        assert_eq!(app.library_mode, LibraryMode::Plugins);
        click_at(&ctx, &mut app, Pos2::new(125.0, 146.5));
        assert_eq!(app.plugin_category, Some(PluginCategory::Bass));
        let bass_racks = app.listed_rows();
        assert_eq!(bass_racks.len(), 20);
        assert!(bass_racks
            .iter()
            .all(|&i| plugins::library()[i].category == PluginCategory::Bass));
        let second_rack = bass_racks[1];
        click_at(&ctx, &mut app, Pos2::new(125.0, 227.0 + 37.0 + 18.5));
        assert_eq!(app.plugin_sel, Some(second_rack));

        click_at(&ctx, &mut app, Pos2::new(125.0, 211.0));
        type_text(&ctx, &mut app, "Riddim Wub · Heavy");
        assert_eq!(app.plugin_search, "Riddim Wub · Heavy");
        let rack_idx = plugins::library()
            .iter()
            .position(|r| r.name == "Riddim Wub · Heavy")
            .expect("rack in library");
        assert_eq!(app.listed_rows(), vec![rack_idx]);
        assert_eq!(app.plugin_sel, Some(rack_idx));

        click_at(&ctx, &mut app, Pos2::new(125.0, 227.0 + 18.5));
        assert_eq!(app.plugin_sel, Some(rack_idx));
        let rack = &plugins::library()[rack_idx];
        let effects_before = app.song.tracks[1].effects.len();
        click_at(&ctx, &mut app, Pos2::new(125.0, 828.0));
        let inserted = &app.song.tracks[1].effects[effects_before..];
        assert_eq!(inserted.len(), rack.devices.len());
        for (actual, expected) in inserted.iter().zip(&rack.devices) {
            assert_eq!(actual.kind, expected.kind);
            assert_eq!(actual.params, expected.params);
            assert_eq!(actual.on, expected.on);
        }
        assert_eq!(app.dev_sel, DevSel::Effect(effects_before));

        // Empty plugin results clear the selection and cannot insert.
        app.plugin_search = "no-such-plugin-rack".to_string();
        run_frame(&ctx, &mut app, Vec::new());
        assert!(app.listed_rows().is_empty());
        assert_eq!(app.plugin_sel, None);
        let empty_before = app.song.tracks[1].effects.len();
        click_at(&ctx, &mut app, Pos2::new(125.0, 828.0));
        assert_eq!(app.song.tracks[1].effects.len(), empty_before);

        // Empty sound results also clear selection and cannot load.
        click_at(&ctx, &mut app, Pos2::new(49.66, 102.5));
        app.sound_search = "no-such-sound".to_string();
        run_frame(&ctx, &mut app, Vec::new());
        assert!(app.listed_rows().is_empty());
        assert_eq!(app.sound_sel, None);
        let params_before = app.song.tracks[0].instrument.params.clone();
        click_at(&ctx, &mut app, Pos2::new(125.0, 828.0));
        assert_eq!(app.song.tracks[0].instrument.params, params_before);

        // Preserved DEVICES fallback still loads raw instruments and effects.
        app.sel_track = 0;
        click_at(&ctx, &mut app, Pos2::new(200.34, 102.5));
        assert_eq!(app.library_mode, LibraryMode::Devices);
        assert_eq!(app.listed_rows().len(), 33);

        click_at(&ctx, &mut app, Pos2::new(125.0, 162.5));
        assert_eq!(app.song.tracks[0].instrument.kind, DeviceKind::SubBass);
        assert_eq!(app.song.tracks[0].name, "Sub Bass");

        let raw_effects_before = app.song.tracks[0].effects.len();
        click_at(&ctx, &mut app, Pos2::new(125.0, 430.5));
        assert_eq!(app.song.tracks[0].effects.len(), raw_effects_before + 1);
        assert_eq!(
            app.song.tracks[0].effects.last().map(|d| d.kind),
            Some(DeviceKind::Filter)
        );
    }

    fn piano_frame(ctx: &egui::Context, app: &mut HarpsiApp, events: Vec<egui::Event>) {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 800.0));
        let raw = egui::RawInput {
            screen_rect: Some(rect),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw, |ctx| {
            egui::Area::new(egui::Id::unique("headless-piano"))
                .fixed_pos(Pos2::ZERO)
                .show(ctx, |ui| {
                    ui.set_min_size(rect.size());
                    crate::view_piano::show(app, ui, rect);
                });
        });
        output.textures_delta.clear();
    }

    fn piano_click(ctx: &egui::Context, app: &mut HarpsiApp, pos: Pos2) {
        piano_frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
        piano_frame(ctx, app, vec![egui::Event::PointerMoved(pos)]);
        piano_frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::default(),
                },
            ],
        );
        piano_frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                },
            ],
        );
        piano_frame(ctx, app, Vec::new());
    }

    #[test]
    fn piano_selection_clipboard_and_duplicate_work() {
        let mut app = headless_app();
        app.clip_edit = Some(ClipRef {
            track: 0,
            scene: 0,
            start: 0.0,
        });
        app.sel_track = 0;
        let cr = app.clip_edit.unwrap();
        {
            let clip = clip_at_mut(&mut app.song, &cr).unwrap();
            clip.length = 16.0;
        }
        app.sel_notes = [0usize, 1].into_iter().collect();
        app.sel_anchor = Some(0);

        assert!(app.copy_selection(false));
        assert_eq!(app.note_clipboard.len(), 2);
        app.piano_cursor = 4.0;
        assert!(app.paste_clipboard());
        let clip = clip_at(&app.song, &cr).unwrap();
        assert_eq!(clip.notes.len(), 6);
        assert_eq!(clip.notes[4].start, 4.0);
        assert_eq!(clip.notes[5].start, 5.0);
        assert_eq!(app.sel_notes.len(), 2);

        assert!(app.duplicate_selection());
        let clip = clip_at(&app.song, &cr).unwrap();
        assert_eq!(clip.notes.len(), 8);
        assert_eq!(clip.notes[6].start, 5.5);
        assert_eq!(clip.notes[7].start, 6.5);
        assert_eq!(app.sel_notes.len(), 2);

        assert!(app.delete_selected_notes());
        let clip = clip_at(&app.song, &cr).unwrap();
        assert_eq!(clip.notes.len(), 6);
        assert!(app.sel_notes.is_empty());
    }

    #[test]
    fn piano_scale_snap_applies_to_pasted_notes() {
        let mut app = headless_app();
        app.clip_edit = Some(ClipRef {
            track: 0,
            scene: 0,
            start: 0.0,
        });
        app.sel_track = 0;
        app.song.scale_root = 0;
        app.song.scale_mode = 0;
        app.song.scale_snap = true;
        let cr = app.clip_edit.unwrap();
        {
            let clip = clip_at_mut(&mut app.song, &cr).unwrap();
            clip.notes.clear();
            clip.notes.push(Note::new(0.0, 0.5, 61, 0.9));
        }
        app.select_only_note(0);
        assert!(app.copy_selection(false));
        app.piano_cursor = 2.0;
        assert!(app.paste_clipboard());
        let clip = clip_at(&app.song, &cr).unwrap();
        assert_eq!(clip.notes.len(), 2);
        assert_eq!(clip.notes[1].start, 2.0);
        assert_eq!(clip.notes[1].pitch, 60);
    }

    #[test]
    fn create_part_opens_an_empty_editable_track() {
        let mut app = headless_app();
        let before = app.song.tracks.len();
        app.act(Act::CreatePart(DeviceKind::DrumRack));
        assert_eq!(app.song.tracks.len(), before + 1);
        assert_eq!(app.view, View::Piano);
        let cr = app.clip_edit.expect("new part selected");
        assert_eq!(cr.track, before);
        assert_eq!(cr.scene, 0);
        assert_eq!(app.song.tracks[before].name, "Drums");
        assert_eq!(
            app.song.tracks[before].instrument.kind,
            DeviceKind::DrumRack
        );
        let clip = clip_at(&app.song, &cr).expect("new clip");
        assert!(clip.notes.is_empty());
    }

    #[test]
    fn piano_ui_click_draws_a_note_and_selects_it() {
        let ctx = egui::Context::default();
        let mut app = headless_app();
        app.clip_edit = Some(ClipRef {
            track: 0,
            scene: 0,
            start: 0.0,
        });
        app.sel_track = 0;
        app.view = View::Piano;
        let cr = app.clip_edit.unwrap();
        let before = clip_at(&app.song, &cr).unwrap().notes.len();
        piano_click(&ctx, &mut app, Pos2::new(420.0, 300.0));
        let clip = clip_at(&app.song, &cr).unwrap();
        assert_eq!(clip.notes.len(), before + 1);
        assert_eq!(app.sel_notes.len(), 1);
        assert_eq!(app.sel_anchor, app.sel_notes.iter().next().copied());
    }

    #[test]
    fn scale_settings_survive_project_round_trip() {
        let mut app = headless_app();
        app.song.scale_root = 7;
        app.song.scale_mode = 3;
        app.song.scale_snap = true;
        let dir = std::env::temp_dir().join(format!("harpsistudio-scale-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("scale.harp");
        project::save_song_to(&app.song, &path).expect("save");
        let loaded = project::load_song(&path).expect("load");
        assert_eq!(loaded.scale_root, 7);
        assert_eq!(loaded.scale_mode, 3);
        assert!(loaded.scale_snap);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn live_launch_and_follow_controls_survive_project_round_trip() {
        let mut app = headless_app();
        app.song.launch_quantum = 8.0;
        app.act(Act::CycleClipFollow(ClipRef {
            track: 0,
            scene: 0,
            start: 0.0,
        }));
        app.act(Act::CycleClipFollowAfter(ClipRef {
            track: 0,
            scene: 0,
            start: 0.0,
        }));
        assert_eq!(app.song.tracks[0].clips[0].follow_action, 1);
        assert_eq!(app.song.tracks[0].clips[0].follow_after, 8.0);

        let dir = std::env::temp_dir().join(format!("harpsistudio-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("live.harp");
        project::save_song_to(&app.song, &path).expect("save");
        let loaded = project::load_song(&path).expect("load");
        assert_eq!(loaded.launch_quantum, 8.0);
        assert_eq!(loaded.tracks[0].clips[0].follow_action, 1);
        assert_eq!(loaded.tracks[0].clips[0].follow_after, 8.0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn clip_and_device_details_stack_and_keep_the_workspace() {
        let mut app = headless_app();
        app.show_main(View::Arrangement);
        assert_eq!(app.view, View::Arrangement);

        app.open_detail(View::Piano);
        assert_eq!(app.view, View::Piano);
        assert_eq!(app.main_view, View::Arrangement);
        assert!(app.clip_open);
        assert!(!app.device_open);

        app.show_main(View::Session);
        assert_eq!(app.view, View::Piano);
        assert_eq!(app.main_view, View::Session);

        app.open_detail(View::Device);
        assert_eq!(app.view, View::Device);
        assert!(app.clip_open && app.device_open);

        app.close_device();
        assert_eq!(app.view, View::Piano);
        assert!(app.clip_open);
        assert!(!app.device_open);

        app.close_clip();
        assert_eq!(app.view, View::Session);
        assert!(!app.detail_open());

        app.open_detail(View::Device);
        assert_eq!(app.view, View::Device);
        app.toggle_mixer();
        assert_eq!(app.view, View::Mixer);
        app.toggle_mixer();
        assert_eq!(app.view, View::Device);

        app.close_clip();
        app.close_device();
        assert_eq!(app.view, View::Session);
        assert!(!app.detail_open());
    }
}
