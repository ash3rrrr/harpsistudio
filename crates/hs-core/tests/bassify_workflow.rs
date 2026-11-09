//! End-to-end proof of the Voice -> Bass workflow. The test uses the same
//! AudioTake import, add_vocal_bass_track, project save/load and offline
//! mixdown helpers that the START panel and Save/Mixdown actions call.

use hs_core::audio_take::AudioTake;
use hs_core::params::DeviceKind;
use hs_core::presets::{add_vocal_bass_track, VocalStyle};
use hs_core::project;
use hs_core::samples;
use hs_core::song::Song;
use std::f32::consts::TAU;
use std::path::{Path, PathBuf};

const SR: u32 = 48_000;
const DURATION: f32 = 2.6;
const ANALYSIS_SECONDS: f32 = 2.4;

#[derive(Clone, Copy, Debug)]
struct Metrics {
    full_peak: f32,
    raw_peak: i16,
    rms: f32,
    bass_rms: f32,
    bass_fraction: f32,
    min_window_rms: f32,
    dc: f32,
}

fn proof_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/bassify-proof");
    std::fs::create_dir_all(&dir).expect("proof directory creates");
    dir
}

fn formant(freq: f32, center: f32, width: f32) -> f32 {
    (-0.5 * ((freq - center) / width).powi(2)).exp()
}

/// A deterministic, continuously sounding formant-rich voice with a rising
/// pitch contour and a little breath noise. It is deliberately not a fixture
/// baked into the repository, so this test generates a new WAV on every run.
fn synthetic_voice() -> Vec<f32> {
    let frames = (DURATION * SR as f32) as usize;
    let mut out = Vec::with_capacity(frames);
    let mut phase = 0.0f32;
    let mut lcg = 0x1234_5678u32;
    for i in 0..frames {
        let t = i as f32 / SR as f32;
        let progress = t / DURATION;
        let f0 = 165.0 + 60.0 * progress + 2.5 * (TAU * 5.1 * t).sin();
        phase = (phase + TAU * f0 / SR as f32) % TAU;
        let envelope = 0.82 + 0.12 * (TAU * 2.0 * t + 0.4).sin();
        let mut voice = 0.0;
        for h in 1..=16 {
            let freq = f0 * h as f32;
            let amp = 1.0 / (h as f32).powf(0.95)
                + 1.15 * formant(freq, 720.0, 135.0)
                + 0.75 * formant(freq, 1220.0, 220.0)
                + 0.35 * formant(freq, 2600.0, 360.0);
            voice += (phase * h as f32).sin() * amp;
        }
        lcg = lcg.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = ((lcg >> 8) as f32 / 8_388_607.5 - 1.0) * 0.012;
        out.push(voice * 0.055 * envelope + noise);
    }
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let scale = 0.62 / peak.max(1e-9);
    for s in &mut out {
        *s *= scale;
    }
    out
}

fn write_pcm16_mono_wav(path: &Path, samples: &[f32]) {
    let data_len = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&SR.to_le_bytes());
    bytes.extend_from_slice(&(SR * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    std::fs::write(path, bytes).expect("synthetic input WAV writes");
}

fn rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt()
}

fn lowpass4(mut samples: Vec<f32>, cutoff: f32, sr: f32) -> Vec<f32> {
    let a = 1.0 - (-std::f32::consts::TAU * cutoff / sr).exp();
    let mut z = 0.0;
    for _ in 0..4 {
        for s in &mut samples {
            z += (*s - z) * a;
            *s = z;
        }
    }
    samples
}

fn highpass4(mut samples: Vec<f32>, cutoff: f32, sr: f32) -> Vec<f32> {
    let a = 1.0 - (-std::f32::consts::TAU * cutoff / sr).exp();
    let mut z = 0.0;
    for _ in 0..4 {
        for s in &mut samples {
            z += (*s - z) * a;
            *s -= z;
        }
    }
    samples
}

/// Read the raw interleaved 16-bit data so a clipped side channel cannot hide
/// behind a mono downmix.
fn raw_peak_i16(path: &Path) -> i16 {
    let bytes = std::fs::read(path).expect("rendered WAV reads");
    let mut pos = 12usize;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let start = pos + 8;
        let end = start.saturating_add(size).min(bytes.len());
        if id == b"data" {
            return bytes[start..end]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| i16::from_le_bytes(*b).unsigned_abs().min(32767) as i16)
                .max()
                .unwrap_or(0);
        }
        pos = start + size + (size & 1);
    }
    0
}

fn analyze(path: &Path, seconds: f32) -> Metrics {
    let bytes = std::fs::read(path).expect("rendered WAV reads");
    let decoded = samples::decode_wav(&bytes).expect("rendered WAV decodes");
    let full_peak = decoded.data.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let raw_peak = raw_peak_i16(path);
    let frames = ((seconds * decoded.rate) as usize)
        .min(decoded.data.len())
        .max(1);
    let window = &decoded.data[..frames];
    let dc = window.iter().sum::<f32>() / window.len().max(1) as f32;
    let ac: Vec<f32> = window.iter().map(|s| *s - dc).collect();
    let level = rms(&ac);
    let band = highpass4(ac, 20.0, decoded.rate);
    let bass = rms(&lowpass4(band, 180.0, decoded.rate));
    let win_len = ((0.05 * decoded.rate) as usize).max(1);
    let min_window_rms = window
        .chunks(win_len)
        .map(rms)
        .fold(f32::INFINITY, f32::min);
    Metrics {
        full_peak,
        raw_peak,
        rms: level,
        bass_rms: bass,
        bass_fraction: bass / level.max(1e-12),
        min_window_rms,
        dc,
    }
}

fn db(value: f32) -> f32 {
    20.0 * value.max(1e-12).log10()
}

#[test]
fn fresh_vocal_survives_source_deletion_and_remixes_identically_for_both_styles() {
    let dir = proof_dir();
    let input = dir.join("fresh-formant-sweep-48k.wav");
    write_pcm16_mono_wav(&input, &synthetic_voice());

    let input_metrics = analyze(&input, ANALYSIS_SECONDS);
    assert!(
        input_metrics.min_window_rms > 0.005,
        "synthetic vocal has a silent section: {:?}",
        input_metrics
    );
    assert!(input_metrics.raw_peak > 1000, "synthetic vocal too quiet");
    assert!(
        input_metrics.raw_peak < i16::MAX,
        "synthetic vocal itself is clipped"
    );

    let source_take = AudioTake::from_wav_file(&input).expect("fresh WAV imports");
    assert!(source_take.duration_seconds() > 2.5);
    assert_eq!(source_take.sample_rate, SR);

    let styles = [
        (VocalStyle::Dubstep, "dubstep", "Dubstep", -12.0, 3.2, 3.2),
        (VocalStyle::Rap, "rap", "Rap", -7.0, 1.2, 1.0),
    ];
    let mut prepared = Vec::new();

    for (style, slug, label, pitch, bassify_cycle, wobble_rate) in styles {
        let take = source_take.clone();
        let mut song = Song::empty(&format!("Bassify Proof {label}"), 140.0);
        let idx = add_vocal_bass_track(&mut song, take.clone(), style);
        assert_eq!(idx, 0);

        let track = &song.tracks[idx];
        assert_eq!(track.name, format!("{label} Vocal Bass"));
        assert_eq!(track.instrument.kind, DeviceKind::VocalTake);
        assert_eq!(
            track.take.as_ref().expect("take attached").samples,
            take.samples
        );
        assert_eq!(
            track.effects.iter().map(|e| e.kind).collect::<Vec<_>>(),
            vec![
                DeviceKind::Bassify,
                DeviceKind::Wobble,
                DeviceKind::Compressor,
                DeviceKind::Eq3,
            ]
        );
        assert!((track.instrument.p(1) - pitch).abs() < 0.01);
        assert!((track.effects[0].p(7) - bassify_cycle).abs() < 0.01);
        assert!((track.effects[1].p(0) - wobble_rate).abs() < 0.01);

        let session = track
            .clips
            .iter()
            .find(|c| c.scene == 0)
            .expect("session clip exists");
        let arrangement = track
            .clips
            .iter()
            .find(|c| c.scene == -1)
            .expect("arrangement clip exists");
        assert_eq!(arrangement.start, 0.0, "arrangement must start at bar 1");
        assert!(arrangement.length > 0.0);
        assert!(session.notes.iter().any(|n| n.start == 0.0 && n.len > 0.0));
        assert!(arrangement
            .notes
            .iter()
            .any(|n| n.start == 0.0 && n.len > 0.0));

        let before = dir.join(format!("{slug}-before-source-delete.wav"));
        project::export_wav_to(&song, &before).expect("before mixdown writes");

        let project_path = dir.join(format!("{slug}.harp"));
        project::save_song_to(&song, &project_path).expect("project saves");
        let json = std::fs::read_to_string(&project_path).expect("saved project reads");
        assert!(
            json.contains("\"samples\": \""),
            "project does not contain inline take PCM"
        );
        assert!(
            !json.contains(&input.to_string_lossy().to_string()),
            "project retained the source WAV path"
        );

        prepared.push((slug, label, project_path, before, take));
    }

    assert!(input.exists());
    std::fs::remove_file(&input).expect("source WAV deletes");
    assert!(!input.exists(), "source WAV still exists before reopen");

    for (slug, label, project_path, before, take) in prepared {
        let loaded =
            project::load_song(&project_path).expect("project reopens after source delete");
        let track = &loaded.tracks[0];
        assert_eq!(track.name, format!("{label} Vocal Bass"));
        assert_eq!(
            track
                .take
                .as_ref()
                .expect("take survived save/load")
                .samples,
            take.samples
        );
        assert_eq!(
            track.effects.iter().map(|e| e.kind).collect::<Vec<_>>(),
            vec![
                DeviceKind::Bassify,
                DeviceKind::Wobble,
                DeviceKind::Compressor,
                DeviceKind::Eq3,
            ]
        );
        assert!(track.clips.iter().any(|c| c.scene == 0));
        assert!(track.clips.iter().any(|c| c.scene == -1 && c.start == 0.0));

        let after = dir.join(format!("{slug}-after-source-delete.wav"));
        project::export_wav_to(&loaded, &after).expect("after mixdown writes");
        assert_eq!(
            std::fs::read(&before).expect("before WAV reads"),
            std::fs::read(&after).expect("after WAV reads"),
            "{label} mixdown changed after source deletion/reopen"
        );

        let metrics = analyze(&before, ANALYSIS_SECONDS);
        assert!(
            metrics.full_peak.is_finite(),
            "{label} mixdown contains NaN/inf"
        );
        assert!(
            metrics.raw_peak > 1000,
            "{label} mixdown is silent (raw peak {})",
            metrics.raw_peak
        );
        assert!(
            metrics.raw_peak < i16::MAX,
            "{label} mixdown clips at full scale (raw peak {})",
            metrics.raw_peak
        );
        assert!(
            metrics.dc.abs() < 0.002,
            "{label} mixdown has a DC offset ({:.5})",
            metrics.dc
        );
        assert!(
            metrics.rms > 0.008,
            "{label} mixdown RMS too quiet ({})",
            metrics.rms
        );
        assert!(
            metrics.bass_rms > 0.006,
            "{label} mixdown has too little bass-band energy ({})",
            metrics.bass_rms
        );
        // 20–180 Hz is the musical bass band. The Dubstep preset keeps a
        // strong 180–400 Hz harmonic body for the "talking" character, so a
        // third of RMS in the fundamental band is a bass-forward result; DC is
        // rejected separately above.
        assert!(
            metrics.bass_fraction > 0.30,
            "{label} mixdown is not bass-heavy ({:.3})",
            metrics.bass_fraction
        );
        assert!(
            metrics.bass_fraction > input_metrics.bass_fraction + 0.15,
            "{label} bass fraction {:.3} did not improve on input {:.3}",
            metrics.bass_fraction,
            input_metrics.bass_fraction
        );

        println!(
            "BASSIFY_PROOF style={label} input_bass_fraction={:.3} output_bass_fraction={:.3} \
             dc={:+.5} peak_dbfs={:.2} rms_dbfs={:.2} bass_dbfs={:.2} raw_peak={} before={} after={} project={}",
            input_metrics.bass_fraction,
            metrics.bass_fraction,
            metrics.dc,
            db(metrics.full_peak),
            db(metrics.rms),
            db(metrics.bass_rms),
            metrics.raw_peak,
            before.display(),
            after.display(),
            project_path.display()
        );
    }
}
