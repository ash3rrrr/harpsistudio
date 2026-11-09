use hs_core::devices::DRUM_LANES;
use hs_core::engine::Engine;
use hs_core::params::DeviceKind;
use hs_core::presets::{demo_song, DEMOS};
use hs_core::song::{Clip, Note, Song};

#[test]
fn house_demo_renders_audible_audio() {
    let song = demo_song(0);
    let samples = Engine::render_song(&song, 48_000.0);

    assert!(
        samples.len() > 48_000 * 2,
        "render is shorter than one second"
    );
    assert!(
        samples.iter().all(|s| s.is_finite()),
        "render contains NaN/inf"
    );
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(peak > 0.01, "demo rendered silence (peak {peak})");
}

#[test]
fn organ_formant_demo_renders_clean_signal() {
    let song = demo_song(8);
    let samples = Engine::render_song(&song, 48_000.0);

    assert!(
        samples.len() > 48_000 * 2,
        "Organ House render is shorter than one second"
    );
    assert!(
        samples.iter().all(|s| s.is_finite()),
        "Organ House contains NaN/inf"
    );
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
    assert!(peak > 0.02, "Organ House rendered silence (peak {peak})");
    assert!(rms > 0.004, "Organ House is too quiet (RMS {rms})");
    assert!(peak <= 1.0, "Organ House clips (peak {peak})");
}

#[test]
fn every_new_drum_kit_renders_audible_non_clipping_hits() {
    for kit in 6..13 {
        let mut song = Song::empty("Kit Render", 120.0);
        let ti = song.add_track(DeviceKind::DrumRack);
        song.tracks[ti].instrument.set(8, kit as f32);
        let mut clip = Clip::new("all-lanes", -1, 0.0, 4.0);
        for (lane, (_name, note)) in DRUM_LANES.iter().enumerate() {
            clip.notes
                .push(Note::new(lane as f64 * 0.25, 0.2, *note, 0.9));
        }
        song.tracks[ti].clips.push(clip);

        let samples = Engine::render_song(&song, 24_000.0);
        assert!(
            samples.len() > 24_000,
            "kit {kit} rendered less than one second"
        );
        assert!(
            samples.iter().all(|s| s.is_finite()),
            "kit {kit} contains NaN/inf"
        );
        let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        assert!(peak > 0.01, "kit {kit} rendered silence (peak {peak})");
        assert!(rms > 0.001, "kit {kit} is too quiet (RMS {rms})");
        assert!(peak <= 1.0, "kit {kit} clips (peak {peak})");
    }
}

#[test]
fn sample_forward_demos_render_audible_audio() {
    for (index, demo) in DEMOS.iter().enumerate().skip(5) {
        let song = demo_song(index);
        let samples = Engine::render_song(&song, 24_000.0);
        assert!(
            samples.len() > 24_000 * 2,
            "{} rendered less than one second",
            demo.name
        );
        assert!(
            samples.iter().all(|s| s.is_finite()),
            "{} contains NaN/inf",
            demo.name
        );
        let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.01, "{} rendered silence (peak {peak})", demo.name);
    }
}
