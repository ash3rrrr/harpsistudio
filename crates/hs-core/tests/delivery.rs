use hs_core::audio_take::AudioTake;
use hs_core::devices::DRUM_LANES;
use hs_core::engine::Engine;
use hs_core::params::{Device, DeviceKind};
use hs_core::presets::{add_vocal_bass_track, demo_song, VocalStyle};
use hs_core::project;
use hs_core::samples;
use hs_core::song::{Clip, Note, Song};
use std::fs;
use std::path::{Path, PathBuf};

const SR: f32 = 48_000.0;

struct WavStats {
    duration_s: f64,
    peak: f32,
    rms: f32,
    dc: f32,
    clipped: usize,
    low_fraction: f32,
    mono: Vec<f32>,
}

fn mono(stereo: &[f32]) -> Vec<f32> {
    stereo
        .chunks(2)
        .map(|f| f.iter().sum::<f32>() / f.len() as f32)
        .collect()
}

fn rms(xs: &[f32]) -> f32 {
    (xs.iter().map(|x| x * x).sum::<f32>() / xs.len().max(1) as f32).sqrt()
}

fn mean(xs: &[f32]) -> f32 {
    xs.iter().sum::<f32>() / xs.len().max(1) as f32
}

fn lowpass4(mut xs: Vec<f32>, cutoff: f32) -> Vec<f32> {
    let a = 1.0 - (-std::f32::consts::TAU * cutoff / SR).exp();
    let mut z = 0.0;
    for _ in 0..4 {
        for x in &mut xs {
            z += (*x - z) * a;
            *x = z;
        }
    }
    xs
}

fn highpass4(mut xs: Vec<f32>, cutoff: f32) -> Vec<f32> {
    let a = 1.0 - (-std::f32::consts::TAU * cutoff / SR).exp();
    let mut z = 0.0;
    for _ in 0..4 {
        for x in &mut xs {
            z += (*x - z) * a;
            *x -= z;
        }
    }
    xs
}

fn band(xs: &[f32], lo: f32, hi: f32) -> Vec<f32> {
    lowpass4(highpass4(xs.to_vec(), lo), hi)
}

fn read_wav(path: &Path) -> WavStats {
    let bytes = fs::read(path).expect("read exported WAV");
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let mut pos = 12usize;
    let mut rate = 0u32;
    let mut channels = 0u16;
    let mut bits = 0u16;
    let mut format = 0u16;
    let mut data_start = 0usize;
    let mut data_len = 0usize;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let start = pos + 8;
        let end = start.saturating_add(size).min(bytes.len());
        if id == b"fmt " && size >= 16 {
            format = u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap());
            channels = u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap());
            rate = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap());
            bits = u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap());
        } else if id == b"data" {
            data_start = start;
            data_len = end.saturating_sub(start);
            break;
        }
        pos = start + size + (size & 1);
    }
    assert_eq!(format, 1);
    assert_eq!(channels, 2);
    assert_eq!(bits, 16);
    assert_eq!(rate, 48_000);
    let frame_bytes = 4usize;
    let frames = data_len / frame_bytes;
    let data = &bytes[data_start..data_start + frames * frame_bytes];
    let mut mono = Vec::with_capacity(frames);
    let mut clipped = 0usize;
    for frame in data.chunks_exact(frame_bytes) {
        let mut sum = 0i32;
        for ch in 0..2 {
            let off = ch * 2;
            let raw = i16::from_le_bytes([frame[off], frame[off + 1]]);
            if raw == i16::MAX || raw == i16::MIN {
                clipped += 1;
            }
            sum += raw as i32;
        }
        mono.push(sum as f32 / 2.0 / 32_768.0);
    }
    let dc = mean(&mono);
    let ac: Vec<f32> = mono.iter().map(|x| *x - dc).collect();
    let ac_rms = rms(&ac);
    let low = rms(&band(&mono, 20.0, 180.0));
    WavStats {
        duration_s: frames as f64 / rate as f64,
        peak: mono.iter().fold(0.0f32, |a, x| a.max(x.abs())),
        rms: ac_rms,
        dc,
        clipped,
        low_fraction: low / ac_rms.max(1e-9),
        mono,
    }
}

fn assert_clean(stats: &WavStats, label: &str, min_seconds: f64) {
    assert!(
        stats.duration_s >= min_seconds,
        "{label} duration {:.2}s is too short",
        stats.duration_s
    );
    assert!(
        stats.mono.iter().all(|s| s.is_finite()),
        "{label} has NaN/inf"
    );
    assert!(stats.peak > 0.01, "{label} is silent");
    assert!(stats.peak < 0.9999, "{label} peaks too close to full scale");
    assert_eq!(stats.clipped, 0, "{label} has clipped samples");
    assert!(stats.dc.abs() < 0.001, "{label} has DC {:+.6}", stats.dc);
    assert!(stats.rms > 0.005, "{label} is too quiet");
}

fn solo(song: &Song, track: usize) -> Vec<f32> {
    let mut s = song.clone();
    for (i, t) in s.tracks.iter_mut().enumerate() {
        t.mute = i != track;
    }
    Engine::render_song(&s, SR)
}

fn track_index(song: &Song, name: &str) -> usize {
    song.tracks
        .iter()
        .position(|t| t.name == name)
        .unwrap_or_else(|| panic!("missing track {name}"))
}

fn section_rms(mono: &[f32], bpm: f64) -> Vec<f32> {
    let frames = ((4.0 * 4.0) * 60.0 / bpm * SR as f64) as usize;
    mono.chunks(frames.max(1)).take(4).map(rms).collect()
}

fn env_cv(stereo: &[f32], win_ms: f32) -> f32 {
    let m = mono(stereo);
    let step = (SR * win_ms / 1000.0) as usize;
    let e: Vec<f32> = m.chunks(step.max(1)).map(rms).collect();
    let avg = mean(&e);
    let var = e.iter().map(|x| (x - avg).powi(2)).sum::<f32>() / e.len().max(1) as f32;
    var.sqrt() / avg.max(1e-9)
}

fn on_off_ratio(stereo: &[f32], bpm: f64, beats: usize) -> f32 {
    let m = mono(stereo);
    let beat = (60.0 / bpm * SR as f64) as usize;
    let mut on = 0.0;
    let mut off = 0.0;
    let mut n = 0usize;
    for b in 0..beats {
        let at = b * beat;
        let mid = (at + beat / 2).min(m.len());
        let end = (at + beat).min(m.len());
        if mid > at && end > mid {
            on += rms(&m[at..mid]).powi(2);
            off += rms(&m[mid..end]).powi(2);
            n += 1;
        }
    }
    (on / n.max(1) as f32).sqrt() / (off / n.max(1) as f32).sqrt().max(1e-9)
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "harpsistudio-delivery-{}-{tag}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn shipped_four_genres_export_clean_full_length_mixdowns() {
    let dir = temp_dir("genres");
    let specs = [
        (1usize, "dubstep"),
        (0usize, "house"),
        (2usize, "edm"),
        (7usize, "rap"),
    ];
    let mut songs = Vec::new();
    let mut stats = Vec::new();
    for (idx, genre) in specs {
        let song = demo_song(idx);
        assert_eq!(song.length_beats(), 64.0, "{genre} is not 16 bars");
        let path = dir.join(format!("{genre}.wav"));
        project::export_wav_to(&song, &path).expect("UI-equivalent mixdown");
        let s = read_wav(&path);
        assert_clean(&s, genre, 20.0);
        songs.push(song);
        stats.push(s);
    }

    let growl_cv = env_cv(&solo(&songs[0], track_index(&songs[0], "Growl Bass")), 20.0);
    assert!(stats[0].low_fraction > 0.35, "dubstep low end is weak");
    assert!(
        growl_cv > 0.30,
        "dubstep growl is not moving (CV {growl_cv:.3})"
    );

    let kick_notes = songs[1]
        .tracks
        .iter()
        .flat_map(|t| t.clips.iter())
        .filter(|c| c.scene < 0)
        .flat_map(|c| c.notes.iter())
        .filter(|n| n.pitch == 36)
        .count();
    let house_ratio = on_off_ratio(
        &solo(&songs[1], track_index(&songs[1], "Drums")),
        songs[1].bpm,
        64,
    );
    assert!(kick_notes >= 60, "house has only {kick_notes} kicks");
    assert!(
        house_ratio > 1.15,
        "house is not four-on-the-floor ({house_ratio:.2})"
    );

    let edm_sections = section_rms(&stats[2].mono, songs[2].bpm);
    assert_eq!(edm_sections.len(), 4);
    assert!(
        edm_sections[2] > edm_sections[1] * 1.5,
        "EDM drop is not above build ({:.4} vs {:.4})",
        edm_sections[2],
        edm_sections[1]
    );
    assert!(
        edm_sections[2] > edm_sections[0] * 1.15,
        "EDM drop is not above intro ({:.4} vs {:.4})",
        edm_sections[2],
        edm_sections[0]
    );

    let flow = mono(&solo(&songs[3], track_index(&songs[3], "Rap Flow")));
    let start = (32.0 * 60.0 / songs[3].bpm * SR as f64) as usize;
    let flow_tail = &flow[start.min(flow.len())..];
    let full_tail = &stats[3].mono[start.min(stats[3].mono.len())..];
    let flow_ratio = rms(flow_tail) / rms(full_tail).max(1e-9);
    let flow_mid = rms(&band(flow_tail, 300.0, 4000.0));
    let flow_low = rms(&band(flow_tail, 20.0, 180.0));
    assert!(flow_ratio > 0.25, "rap flow is buried ({flow_ratio:.3})");
    assert!(
        flow_mid > flow_low * 2.0,
        "rap flow has no vocal-range presence"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn supplied_vocal_imports_bassifies_and_exports_clean_bass() {
    let dir = temp_dir("vocal");
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/voice-note.wav");
    let take = AudioTake::from_wav_file(&fixture).expect("fixture imports");
    let mut song = Song::empty("Delivery Vocal", 140.0);
    add_vocal_bass_track(&mut song, take, VocalStyle::Dubstep);
    let track = song.tracks.last().unwrap();
    assert_eq!(track.instrument.kind, DeviceKind::VocalTake);
    assert!(track.take.is_some());
    let chain: Vec<DeviceKind> = track.effects.iter().map(|e| e.kind).collect();
    assert_eq!(
        chain,
        vec![
            DeviceKind::Bassify,
            DeviceKind::Wobble,
            DeviceKind::Compressor,
            DeviceKind::Eq3
        ]
    );

    let path = dir.join("vocal-to-bass.wav");
    project::export_wav_to(&song, &path).expect("vocal mixdown");
    let stats = read_wav(&path);
    assert_clean(&stats, "vocal bass", 1.0);
    assert!(
        stats.low_fraction > 0.20,
        "vocal bass has no low end ({:.3})",
        stats.low_fraction
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn every_drum_kit_lane_and_embedded_sample_is_reachable() {
    assert_eq!(samples::drum_kit_count(), 13);
    assert_eq!(samples::drum_sample_count(), 155);
    let mut seen = std::collections::BTreeSet::new();
    for kit in 0..samples::drum_kit_count() {
        for lane in 0..16 {
            let sample = samples::drum(kit, lane).expect("kit lane sample");
            assert!(!sample.data.is_empty());
            assert!(sample.data.iter().all(|x| x.is_finite()));
            seen.insert(samples::drum_sample_index(kit, lane).unwrap());
        }
    }
    assert_eq!(seen.len(), 155);
    assert_eq!(DRUM_LANES.len(), 16);
}

#[test]
fn salamander_midi_chain_exports_with_a_real_drum_kit() {
    let dir = temp_dir("salamander-midi");
    let mut song = Song::empty("Salamander MIDI Delivery", 120.0);

    let drums_ti = song.add_track(DeviceKind::DrumRack);
    song.tracks[drums_ti].instrument.set(8, 1.0); // House Club
    song.tracks[drums_ti].volume = 0.65;
    let mut drum_clip = Clip::new("House Drums", -1, 0.0, 16.0);
    for bar in 0..4 {
        let o = bar as f64 * 4.0;
        for b in 0..4 {
            drum_clip.notes.push(Note::new(o + b as f64, 0.2, 36, 0.95));
        }
        for b in [1.0, 3.0] {
            drum_clip.notes.push(Note::new(o + b, 0.2, 38, 0.85));
        }
        for s in 0..8 {
            drum_clip
                .notes
                .push(Note::new(o + s as f64 * 0.5, 0.08, 42, 0.22));
        }
    }
    song.tracks[drums_ti].clips.push(drum_clip);

    let piano_ti = song.add_track(DeviceKind::Salamander);
    song.tracks[piano_ti].instrument =
        Device::from_preset(DeviceKind::Salamander, &DeviceKind::Salamander.presets()[0]);
    song.tracks[piano_ti].volume = 0.7;
    let mut chord = Device::new(DeviceKind::ChordGenerator);
    chord.set(0, 0.0);
    chord.set(1, 0.0);
    chord.set(2, 3.0);
    let mut arp = Device::new(DeviceKind::Arpeggiator);
    arp.set(0, 2.0);
    arp.set(1, 0.0);
    arp.set(2, 2.0);
    arp.set(3, 0.5);
    song.tracks[piano_ti].midi_fx = vec![chord, arp];
    let mut piano_clip = Clip::new("Salamander Arp", -1, 0.0, 16.0);
    piano_clip.notes.push(Note::new(0.0, 15.5, 60, 0.9));
    song.tracks[piano_ti].clips.push(piano_clip);

    let path = dir.join("salamander-midi.wav");
    project::export_wav_to(&song, &path).expect("Salamander MIDI mixdown");
    let stats = read_wav(&path);
    assert_clean(&stats, "Salamander MIDI", 5.0);
    assert!(stats.low_fraction > 0.20);
    let _ = fs::remove_dir_all(dir);
}
