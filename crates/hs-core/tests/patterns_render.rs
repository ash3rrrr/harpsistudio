use hs_core::engine::Engine;
use hs_core::midi_fx::transform_notes;
use hs_core::params::{Device, DeviceKind};
use hs_core::patterns::{self, Genre, Role};
use hs_core::song::{Clip, Note, Song};
use std::fs;
use std::path::{Path, PathBuf};

fn peak_rms(samples: &[f32]) -> (f32, f32) {
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt();
    (peak, rms)
}

/// Loudness proxy over the loudest half-second, which tracks sustained chords
/// better than whole-loop RMS (which over-penalizes short, sparse stabs).
fn dc_offset(samples: &[f32]) -> f32 {
    (samples.iter().map(|s| *s as f64).sum::<f64>() / samples.len().max(1) as f64) as f32
}

fn clipped_samples(samples: &[f32]) -> usize {
    samples.iter().filter(|s| s.abs() >= 1.0).count()
}

fn proof_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/dc-proof");
    fs::create_dir_all(dir.join("patterns")).expect("DC proof directory creates");
    dir
}

fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn write_wav(path: &Path, samples: &[f32], sample_rate: u32) {
    let channels: u16 = 2;
    let bits: u16 = 16;
    let data_len = (samples.len() * 2) as u32;
    let byte_rate = sample_rate * channels as u32 * bits as u32 / 8;
    let block_align = channels * bits / 8;
    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&block_align.to_le_bytes());
    bytes.extend_from_slice(&bits.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    fs::write(path, bytes).expect("DC proof WAV writes");
}

fn max_short_rms(samples: &[f32], sr: f32, limit: usize) -> f32 {
    let active = &samples[..limit.min(samples.len())];
    let win = (sr as usize / 2).max(1);
    if active.len() <= win {
        return (active.iter().map(|s| s * s).sum::<f32>() / active.len().max(1) as f32).sqrt();
    }
    let mut energy: f32 = active[..win].iter().map(|s| s * s).sum();
    let mut best = energy;
    for i in win..active.len() {
        energy += active[i] * active[i] - active[i - win] * active[i - win];
        best = best.max(energy);
    }
    (best / win as f32).sqrt()
}

#[test]
fn every_pattern_renders_finite_audible_audio_with_headroom() {
    let lib = patterns::library();
    assert!(lib.len() >= 60);
    let dir = proof_dir();
    let mut short_levels = Vec::new();
    let mut measurements = String::from("pattern\tshort_dbfs\tdc\tpeak\tclipped\n");
    for (index, p) in lib.iter().enumerate() {
        let samples = Engine::render_song(&p.to_song(), 48_000.0);
        assert!(
            samples.len() > 12_000,
            "{} rendered less than one second",
            p.name
        );
        assert!(
            samples.iter().all(|s| s.is_finite()),
            "{} rendered NaN/inf",
            p.name
        );
        let (peak, rms) = peak_rms(&samples);
        let dc = dc_offset(&samples);
        let clipped = clipped_samples(&samples);
        assert!(peak > 0.01, "{} rendered silence (peak {peak})", p.name);
        assert!(peak <= 1.0, "{} clips (peak {peak})", p.name);
        assert!(peak < 0.999, "{} has no headroom (peak {peak})", p.name);
        assert!(rms > 0.0004, "{} is too quiet (RMS {rms})", p.name);
        assert_eq!(clipped, 0, "{} has {clipped} clipped samples", p.name);
        assert!(
            dc.abs() < 0.001,
            "{} has DC offset {dc:+.6} (limit 0.001)",
            p.name
        );

        let active_len = ((p.length_beats() * 60.0 / p.bpm()) * 48_000.0) as usize;
        let short = max_short_rms(&samples, 48_000.0, active_len);
        let level = 20.0 * short.max(1e-9).log10();
        short_levels.push(level);

        let path = dir
            .join("patterns")
            .join(format!("{index:02}-{}.wav", slug(&p.name)));
        write_wav(&path, &samples, 48_000);
        measurements.push_str(&format!(
            "{}\t{level:.3}\t{dc:+.6}\t{peak:.6}\t{clipped}\n",
            p.name
        ));
    }
    fs::write(dir.join("pattern-levels.tsv"), measurements).expect("pattern measurements write");

    let min = short_levels.iter().copied().fold(f32::INFINITY, f32::min);
    let max = short_levels
        .iter()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max);
    let mean = short_levels.iter().sum::<f32>() / short_levels.len() as f32;
    let max_deviation = short_levels
        .iter()
        .map(|level| (level - mean).abs())
        .fold(0.0f32, f32::max);
    println!(
        "PATTERN_DC_PROOF count={} min={min:.3} max={max:.3} mean={mean:.3} spread={:.3} max_dev={max_deviation:.3} dir={}",
        short_levels.len(),
        max - min,
        dir.display()
    );
    assert!(
        max - min <= 1.0,
        "library loudness spread is {:.2} dB ({min:.2}..{max:.2})",
        max - min
    );
    assert!(
        max_deviation <= 1.0,
        "library has a level outlier {max_deviation:.2} dB from mean {mean:.2}"
    );
    assert!(
        (mean + 24.5).abs() <= 0.75,
        "library average short-term level is {mean:.2} dBFS, expected about -24.5"
    );
}

#[test]
fn every_genre_renders_a_layered_preview_in_headroom() {
    for genre in Genre::ALL {
        let mut song = Song::empty(genre.label(), 120.0);
        let mut roles_used = 0;
        for role in [Role::Bassline, Role::ChordProgression, Role::Lead] {
            let Some(pat) = patterns::library()
                .iter()
                .find(|p| p.genre == genre && p.role == role)
            else {
                continue;
            };
            roles_used += 1;
            let idx = song.add_track(pat.instrument);
            let t = &mut song.tracks[idx];
            t.name = format!("{} {}", genre.short(), role.label());
            t.instrument = pat.preview_device();
            t.volume = 0.55;
            t.send = 0.05;
            let mut c = Clip::new(&pat.name, 0, 0.0, pat.length_beats());
            c.notes = pat.notes.clone();
            t.clips.push(c);
        }
        assert_eq!(
            roles_used,
            3,
            "{} preview is missing a layer",
            genre.label()
        );

        let samples = Engine::render_song(&song, 16_000.0);
        assert!(
            samples.iter().all(|s| s.is_finite()),
            "{} preview rendered NaN/inf",
            genre.label()
        );
        let (peak, rms) = peak_rms(&samples);
        let dc = dc_offset(&samples);
        let clipped = clipped_samples(&samples);
        assert!(
            peak > 0.02,
            "{} preview rendered silence (peak {peak})",
            genre.label()
        );
        assert!(
            rms > 0.001,
            "{} preview is too quiet (RMS {rms})",
            genre.label()
        );
        assert!(peak <= 1.0, "{} preview clips (peak {peak})", genre.label());
        assert_eq!(
            clipped,
            0,
            "{} preview has {clipped} clipped samples",
            genre.label()
        );
        assert!(
            dc.abs() < 0.001,
            "{} preview has DC offset {dc:+.6}",
            genre.label()
        );
    }
}

#[test]
fn chord_generator_and_arpeggiator_render_generated_notes() {
    let mut song = Song::empty("MIDI FX Render", 124.0);
    let ti = song.add_track(DeviceKind::Keys);
    let mut chord = Device::new(DeviceKind::ChordGenerator);
    chord.set(0, 0.0); // C
    chord.set(1, 0.0); // Major
    chord.set(2, 3.0); // 7th
    let mut arp = Device::new(DeviceKind::Arpeggiator);
    arp.set(0, 2.0); // 1/16
    arp.set(1, 0.0); // Up
    arp.set(2, 2.0);
    arp.set(3, 0.5);
    song.tracks[ti].midi_fx = vec![chord.clone(), arp.clone()];
    let input = vec![Note::new(0.0, 7.5, 60, 0.9)];
    let generated = transform_notes(&input, &song.tracks[ti].midi_fx, song.bpm);
    assert!(
        generated.len() >= 30,
        "MIDI chain produced only {} notes",
        generated.len()
    );

    let mut c = Clip::new("Generated", 0, 0.0, 8.0);
    c.notes = input;
    song.tracks[ti].clips.push(c);
    let samples = Engine::render_song(&song, 24_000.0);
    assert!(samples.iter().all(|s| s.is_finite()));
    let (peak, rms) = peak_rms(&samples);
    assert!(peak > 0.01, "MIDI-effect track rendered silence");
    assert!(rms > 0.001, "MIDI-effect track is too quiet (RMS {rms})");
    assert!(peak <= 1.0, "MIDI-effect track clips (peak {peak})");
}

#[test]
fn midi_fx_persist_with_the_project() {
    let mut song = Song::empty("Persistence", 120.0);
    let ti = song.add_track(DeviceKind::Pluck);
    song.tracks[ti].midi_fx = vec![
        Device::new(DeviceKind::Arpeggiator),
        Device::new(DeviceKind::ChordGenerator),
    ];
    song.tracks[ti].midi_fx[0].set(0, 3.0);
    song.tracks[ti].midi_fx[1].set(4, -1.0);

    let json = serde_json::to_string(&song).expect("serialize project");
    let back: Song = serde_json::from_str(&json).expect("deserialize project");
    assert_eq!(back.tracks[ti].midi_fx.len(), 2);
    assert_eq!(back.tracks[ti].midi_fx[0].kind, DeviceKind::Arpeggiator);
    assert_eq!(back.tracks[ti].midi_fx[1].kind, DeviceKind::ChordGenerator);
    assert!((back.tracks[ti].midi_fx[0].p(0) - 3.0).abs() < 1e-6);
    assert!((back.tracks[ti].midi_fx[1].p(4) + 1.0).abs() < 1e-6);
}

#[test]
fn older_projects_without_midi_fx_still_load() {
    let json = r#"{
        "name":"Old","bpm":120.0,"master_volume":0.85,"limiter":true,
        "tracks":[{
            "name":"Legacy Keys","color":[120,200,255],
            "instrument":{"kind":"Keys","params":[],"on":true},
            "effects":[],"volume":0.8,"pan":0.0,"send":0.1,
            "mute":false,"solo":false,"clips":[]
        }],
        "master_fx":[],"return_fx":{"kind":"Reverb","params":[],"on":true},
        "scenes":["Scene 1"],"loop_enabled":false,"loop_start":0.0,"loop_end":16.0
    }"#;
    let song: Song = serde_json::from_str(json).expect("legacy project");
    assert!(song.tracks[0].midi_fx.is_empty());
}
