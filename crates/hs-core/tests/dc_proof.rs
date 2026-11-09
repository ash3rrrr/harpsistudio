//! End-to-end DC proof for the instruments and the three headline genres.
//!
//! The pattern-library proof lives in `patterns_render.rs`; this file checks
//! the individual oscillator/filter voices with the master bypassed, then the
//! real DUBSTEP, HOUSE and EDM demo songs through the full master/return path.
//! All artifacts are written under `target/dc-proof`.

use hs_core::engine::Engine;
use hs_core::params::{Device, DeviceKind};
use hs_core::presets::demo_song;
use hs_core::song::{Clip, Note, Song};
use std::fs;
use std::path::{Path, PathBuf};

const SR: f32 = 48_000.0;

fn proof_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/dc-proof");
    fs::create_dir_all(dir.join("instruments")).expect("instrument proof directory creates");
    fs::create_dir_all(dir.join("mixes")).expect("mix proof directory creates");
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

fn db(value: f32) -> f32 {
    20.0 * value.max(1e-12).log10()
}

fn dc_offset(samples: &[f32]) -> f32 {
    (samples.iter().map(|s| *s as f64).sum::<f64>() / samples.len().max(1) as f64) as f32
}

fn clipped_samples(samples: &[f32]) -> usize {
    samples.iter().filter(|s| s.abs() >= 1.0).count()
}

fn peak_abs(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0f32, |m, s| m.max(s.abs()))
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

fn assert_clean(label: &str, samples: &[f32]) {
    assert!(!samples.is_empty(), "{label} rendered no samples");
    assert!(
        samples.iter().all(|s| s.is_finite()),
        "{label} rendered NaN/inf"
    );
    let clipped = clipped_samples(samples);
    assert_eq!(clipped, 0, "{label} has {clipped} clipped samples");
    let dc = dc_offset(samples);
    assert!(
        dc.abs() < 0.001,
        "{label} has DC offset {dc:+.6} (limit 0.001)"
    );
}

#[test]
fn source_instrument_voices_are_dc_free() {
    let kinds = [
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
    ];
    let dir = proof_dir();
    let mut report = String::from("instrument\tdc\tpeak_dbfs\tclipped\n");
    for kind in kinds {
        let mut song = Song::empty(&format!("DC source {}", kind.name()), 120.0);
        // Bypass the return and master so this proves the instrument itself,
        // not a later corrective stage.
        song.master_fx.clear();
        song.return_fx.on = false;
        song.limiter = false;
        song.master_volume = 1.0;
        let idx = song.add_track(kind);
        song.tracks[idx].instrument = Device::new(kind);
        song.tracks[idx].send = 0.0;
        let mut clip = Clip::new("DC note", -1, 0.0, 2.0);
        clip.notes.push(Note::new(
            0.0,
            2.0,
            if kind == DeviceKind::DrumRack { 36 } else { 60 },
            0.9,
        ));
        song.tracks[idx].clips.push(clip);

        let samples = Engine::render_song(&song, SR);
        assert_clean(kind.name(), &samples);
        let dc = dc_offset(&samples);
        let source_peak = peak_abs(&samples);
        report.push_str(&format!(
            "{}\t{dc:+.6}\t{:.2}\t{}\n",
            kind.name(),
            db(source_peak),
            clipped_samples(&samples)
        ));
        let path = dir
            .join("instruments")
            .join(format!("{}.wav", slug(kind.name())));
        write_wav(&path, &samples, SR as u32);
        println!(
            "SOURCE_DC instrument={} dc={dc:+.6} peak_dbfs={:.2} clip={} path={}",
            kind.name(),
            db(source_peak),
            clipped_samples(&samples),
            path.display()
        );
    }
    fs::write(dir.join("instrument-dc.tsv"), report).expect("instrument report writes");
}

#[test]
fn representative_genre_mixes_are_dc_free() {
    let cases = [
        (1usize, "dubstep", "DUBSTEP"),
        (0usize, "house", "HOUSE"),
        (2usize, "edm", "EDM"),
    ];
    let dir = proof_dir();
    let mut report = String::from("case\ttrack\tsr\tdc\tpeak_dbfs\tclipped\n");

    for (index, slug_name, label) in cases {
        let song = demo_song(index);
        let samples = Engine::render_song(&song, SR);
        assert_clean(label, &samples);
        let dc = dc_offset(&samples);
        let mix_peak = peak_abs(&samples);
        let path = dir.join("mixes").join(format!("{slug_name}.wav"));
        write_wav(&path, &samples, SR as u32);
        report.push_str(&format!(
            "mix\t{label}\t48000\t{dc:+.6}\t{:.2}\t{}\n",
            db(mix_peak),
            clipped_samples(&samples)
        ));
        println!(
            "MIX_DC case={label} dc={dc:+.6} peak_dbfs={:.2} clip={} path={}",
            db(mix_peak),
            clipped_samples(&samples),
            path.display()
        );

        // Check the individual tracks too. The stems are rendered at 24 kHz
        // to keep the proof affordable; the full mix above stays at 48 kHz.
        for (track_index, track) in song.tracks.iter().enumerate() {
            let mut solo = song.clone();
            for (i, candidate) in solo.tracks.iter_mut().enumerate() {
                candidate.solo = i == track_index;
                candidate.mute = false;
            }
            let stem = Engine::render_song(&solo, 24_000.0);
            let stem_label = format!("{label}/{}", track.name);
            assert_clean(&stem_label, &stem);
            let stem_dc = dc_offset(&stem);
            let stem_peak = peak_abs(&stem);
            report.push_str(&format!(
                "stem\t{stem_label}\t24000\t{stem_dc:+.6}\t{:.2}\t{}\n",
                db(stem_peak),
                clipped_samples(&stem)
            ));
            println!(
                "STEM_DC case={} track={} dc={stem_dc:+.6} peak_dbfs={:.2} clip={}",
                label,
                track.name,
                db(stem_peak),
                clipped_samples(&stem)
            );
        }
    }

    fs::write(dir.join("mix-analysis.tsv"), report).expect("mix report writes");
}
