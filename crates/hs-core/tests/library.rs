//! End-to-end checks for the factory sound and plugin libraries.

use hs_core::engine::Engine;
use hs_core::params::{Device, DeviceKind};
use hs_core::plugins;
use hs_core::project;
use hs_core::song::{Clip, Note, Song};
use hs_core::sounds;
use std::collections::HashSet;

#[test]
fn catalogues_meet_the_delivery_quota() {
    assert!(
        sounds::count() >= 1_000,
        "sound library has only {} entries",
        sounds::count()
    );
    assert!(
        plugins::count() >= 100,
        "plugin library has only {} entries",
        plugins::count()
    );

    for sound in sounds::library() {
        assert!(sound.kind.is_instrument());
        assert_ne!(sound.kind, DeviceKind::VocalTake);
        assert_eq!(sound.values.len(), sound.kind.defs().len());
    }
    for rack in plugins::library() {
        assert!(!rack.devices.is_empty());
        assert!(rack.devices.iter().all(|d| d.kind.is_audio_effect()));
    }
}

#[test]
fn every_sound_loads_and_every_rack_inserts() {
    let mut device = Device::new(DeviceKind::SubBass);
    for sound in sounds::library() {
        sound.load_into(&mut device);
        assert_eq!(device.kind, sound.kind);
        assert_eq!(device.params, sound.values);
    }

    let mut effects = Vec::new();
    for rack in plugins::library() {
        let start = rack.insert_into(&mut effects);
        assert_eq!(start + rack.devices.len(), effects.len());
    }
    assert_eq!(
        effects.len(),
        plugins::library()
            .iter()
            .map(|r| r.devices.len())
            .sum::<usize>()
    );
}

#[test]
fn every_family_representative_survives_project_save_and_load() {
    let path = std::env::temp_dir().join(format!(
        "harpsistudio-library-family-roundtrip-{}.harp",
        std::process::id()
    ));

    let mut sound_families = HashSet::new();
    for sound in sounds::library() {
        let key = (hs_core::params::device_index(sound.kind), sound.family);
        if !sound_families.insert(key) {
            continue;
        }

        let mut song = Song::empty("Sound family roundtrip", 128.0);
        let track = song.add_track(sound.kind);
        song.tracks[track].name = sound.name.clone();
        song.tracks[track].instrument = sound.to_device();
        let mut clip = Clip::new("Probe", 0, 0.0, 4.0);
        clip.notes.push(Note::new(0.0, 0.5, 60, 0.9));
        song.tracks[track].clips.push(clip);

        project::save_song_to(&song, &path).expect("save sound family project");
        let loaded = project::load_song(&path).expect("load sound family project");
        assert_eq!(loaded.tracks.len(), 1, "{}", sound.name);
        assert_eq!(loaded.tracks[0].instrument.kind, sound.kind);
        assert_eq!(loaded.tracks[0].instrument.params, sound.values);
        assert!(loaded.tracks[0].instrument.on);
        assert_eq!(loaded.tracks[0].clips[0].notes.len(), 1);
    }

    let mut plugin_families = HashSet::new();
    for rack in plugins::library() {
        let family = rack
            .name
            .split(" · ")
            .next()
            .unwrap_or_default()
            .to_string();
        if !plugin_families.insert(family) {
            continue;
        }

        let mut song = Song::empty("Plugin family roundtrip", 128.0);
        let track = song.add_track(DeviceKind::SubBass);
        let chain = rack.chain();
        rack.insert_into(&mut song.tracks[track].effects);

        project::save_song_to(&song, &path).expect("save plugin family project");
        let loaded = project::load_song(&path).expect("load plugin family project");
        assert_eq!(loaded.tracks.len(), 1, "{}", rack.name);
        assert_eq!(loaded.tracks[0].effects.len(), chain.len());
        for (actual, expected) in loaded.tracks[0].effects.iter().zip(&chain) {
            assert_eq!(actual.kind, expected.kind, "{}", rack.name);
            assert_eq!(actual.params, expected.params, "{}", rack.name);
            assert_eq!(actual.on, expected.on, "{}", rack.name);
        }
    }

    let _ = std::fs::remove_file(&path);
    assert_eq!(sound_families.len(), 48);
    assert_eq!(plugin_families.len(), 20);
}

#[test]
fn library_sound_through_library_rack_renders_audio() {
    let sound = sounds::find("Riddim Wobble · Gritty").expect("factory sound");
    let rack = plugins::find("Riddim Wub · Heavy").expect("factory plugin");

    let mut song = Song::empty("Library audition", 140.0);
    let track = song.add_track(sound.kind);
    song.tracks[track].instrument = sound.to_device();
    rack.insert_into(&mut song.tracks[track].effects);
    song.ensure_scenes(1);

    let mut clip = Clip::new("Audition", 0, 0.0, 4.0);
    for i in 0..4 {
        clip.notes.push(Note::new(i as f64, 0.4, 36, 0.9));
    }
    song.tracks[track].clips.push(clip);

    let audio = Engine::render_song(&song, 48000.0);
    assert!(audio.iter().all(|s| s.is_finite()));
    let peak = audio.iter().fold(0.0f32, |a, s| a.max(s.abs()));
    let rms = (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
    assert!(peak > 0.01, "rendered peak {peak}");
    assert!(rms > 0.001, "rendered RMS {rms}");
}
