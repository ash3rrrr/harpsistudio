use hs_core::audio_take::AudioTake;
use hs_core::capture;
use hs_core::engine::Engine;
use hs_core::params::DeviceKind;
use hs_core::presets::{add_vocal_bass_track, VocalStyle};
use hs_core::project;
use hs_core::samples;
use hs_core::song::Song;
use std::path::PathBuf;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/voice-note.wav")
}

fn mono(samples: &[f32]) -> Vec<f32> {
    samples
        .chunks(2)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
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

fn mean(samples: &[f32]) -> f32 {
    samples.iter().sum::<f32>() / samples.len().max(1) as f32
}

/// RMS of the 20–180 Hz bass band and of the mean-removed full signal.
fn bass_band(samples: &[f32], sr: f32) -> (f32, f32) {
    let dc = mean(samples);
    let ac: Vec<f32> = samples.iter().map(|s| *s - dc).collect();
    let total = rms(&ac);
    let bass = rms(&lowpass4(highpass4(ac, 20.0, sr), 180.0, sr));
    (total, bass)
}

fn imported_song() -> Song {
    let take = AudioTake::from_wav_file(&fixture()).expect("fixture WAV imports");
    assert!(take.duration_seconds() > 0.5);
    let mut song = Song::empty("Vocal Bass Test", 140.0);
    add_vocal_bass_track(&mut song, take, VocalStyle::Dubstep);
    song
}

#[test]
fn vocal_import_clip_bassify_renders_audible_bass_band() {
    let song = imported_song();
    let track = song.tracks.last().unwrap();
    assert_eq!(track.instrument.kind, DeviceKind::VocalTake);
    assert!(track.take.is_some(), "take was not attached to the track");
    assert!(track
        .clips
        .iter()
        .any(|c| c.scene == 0 && !c.notes.is_empty()));
    assert_eq!(track.effects[0].kind, DeviceKind::Bassify);
    assert_eq!(track.effects[1].kind, DeviceKind::Wobble);

    let rendered = Engine::render_song(&song, 48_000.0);
    assert!(
        rendered.len() > 48_000 * 2,
        "render was shorter than one second"
    );
    assert!(
        rendered.iter().all(|s| s.is_finite()),
        "render contains NaN/inf"
    );

    let frames = (rendered.len() / 2).min(48_000 * 3);
    let active = mono(&rendered[..frames * 2]);
    let peak = active.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let dc = mean(&active);
    assert!(dc.abs() < 0.01, "processed vocal has DC ({dc})");
    let (total, bass_rms) = bass_band(&active, 48_000.0);

    assert!(peak > 0.02, "processed vocal was silent (peak {peak})");
    assert!(peak <= 0.9999, "processed vocal clips (peak {peak})");
    assert!(total > 0.008, "processed vocal RMS too quiet ({total})");
    assert!(bass_rms > 0.004, "no audible bass-band energy ({bass_rms})");
    assert!(
        bass_rms > total * 0.20,
        "bass band is buried (bass {bass_rms}, total {total})"
    );
}

/// Drives the exact conversion the CPAL input callback uses on real device
/// buffers: an i16 stereo stream becomes mono f32, becomes a take, and renders
/// through Bassify/Wobble. No high-level import helper is involved.
#[test]
fn recorded_capture_path_renders_audible_bass_band() {
    let bytes = std::fs::read(fixture()).expect("fixture WAV reads");
    let decoded = samples::decode_wav(&bytes).expect("fixture WAV decodes");
    let frames: Vec<i16> = decoded
        .data
        .iter()
        .map(|s| (s.clamp(-1.0, 1.0) * 32767.0).round() as i16)
        .collect();
    let mut stereo = Vec::with_capacity(frames.len() * 2);
    for s in &frames {
        stereo.push(*s);
        stereo.push(*s);
    }

    let captured = capture::mono_from_i16(&stereo, 2);
    assert_eq!(captured.len(), frames.len(), "downmix dropped frames");
    // The WASAPI loopback fallback delivers f32 packets; its conversion must
    // agree with the i16 microphone path.
    let float_stereo: Vec<f32> = stereo.iter().map(|s| *s as f32 / 32_768.0).collect();
    let loopback = capture::mono_from_f32(&float_stereo, 2);
    assert_eq!(loopback.len(), frames.len());
    assert!((loopback[0] - captured[0]).abs() < 1e-6);
    assert!(
        captured.iter().all(|s| s.is_finite()),
        "capture has NaN/inf"
    );
    let capture_peak = captured.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(capture_peak > 0.02, "captured buffer was silent");

    let take = AudioTake::from_samples("Recorded take", decoded.rate as u32, &captured);
    assert_eq!(take.samples.len(), captured.len());
    assert!(take.duration_seconds() > 0.5);

    // The take is inline PCM, not a path reference.
    let json = serde_json::to_string(&take).expect("take serializes");
    assert!(json.contains("\"samples\":\""), "take JSON has no samples");
    let round_tripped: AudioTake = serde_json::from_str(&json).expect("take reloads");
    assert_eq!(round_tripped.samples, take.samples);

    let mut song = Song::empty("Captured Vocal", 140.0);
    add_vocal_bass_track(&mut song, take, VocalStyle::Dubstep);
    let rendered = Engine::render_song(&song, 48_000.0);
    assert!(
        rendered.len() > 48_000 * 2,
        "render was shorter than one second"
    );
    assert!(
        rendered.iter().all(|s| s.is_finite()),
        "render contains NaN/inf"
    );

    let active = mono(&rendered[..(48_000 * 3).min(rendered.len() / 2) * 2]);
    let peak = active.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let dc = mean(&active);
    assert!(dc.abs() < 0.01, "captured take has DC ({dc})");
    let (total, bass_rms) = bass_band(&active, 48_000.0);
    assert!(peak > 0.02, "captured take rendered silent (peak {peak})");
    assert!(peak <= 0.9999, "captured take clips (peak {peak})");
    assert!(total > 0.008, "captured take RMS too quiet ({total})");
    assert!(bass_rms > 0.004, "no bass-band energy ({bass_rms})");
    assert!(
        bass_rms > total * 0.20,
        "bass band is buried (bass {bass_rms}, total {total})"
    );
}

#[test]
fn imported_vocal_survives_project_save_and_load() {
    let song = imported_song();
    let expected = song
        .tracks
        .last()
        .and_then(|t| t.take.clone())
        .expect("take present before save");
    let json = serde_json::to_string_pretty(&song).expect("song serializes");
    let path = std::env::temp_dir().join(format!(
        "harpsistudio-vocal-take-{}.harp",
        std::process::id()
    ));
    std::fs::write(&path, json).expect("temp project writes");
    let loaded = project::load_song(&path).expect("project loads");
    let _ = std::fs::remove_file(&path);

    let loaded_take = loaded
        .tracks
        .last()
        .and_then(|t| t.take.as_ref())
        .expect("take survived save/load");
    assert_eq!(loaded_take.samples, expected.samples);
    assert_eq!(loaded_take.sample_rate, expected.sample_rate);
    assert_eq!(loaded_take.duration_seconds(), expected.duration_seconds());
}
