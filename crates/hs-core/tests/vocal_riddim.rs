use hs_core::engine::Engine;
use hs_core::params::DeviceKind;
use hs_core::presets::{demo_song, DEMOS};
use hs_core::project;
use hs_core::song::{Song, Track};

fn track<'a>(song: &'a Song, name: &str) -> &'a Track {
    song.tracks
        .iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("missing track {name}"))
}

fn solo(song: &Song, index: usize) -> Vec<f32> {
    let mut s = song.clone();
    for (i, t) in s.tracks.iter_mut().enumerate() {
        t.mute = i != index;
        t.solo = false;
    }
    Engine::render_song(&s, 24_000.0)
}

fn mono(stereo: &[f32]) -> Vec<f32> {
    stereo
        .chunks(2)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

fn rms(xs: &[f32]) -> f32 {
    (xs.iter().map(|x| x * x).sum::<f32>() / xs.len().max(1) as f32).sqrt()
}

fn lowpass4(mut xs: Vec<f32>, cutoff: f32) -> Vec<f32> {
    let a = 1.0 - (-std::f32::consts::TAU * cutoff / 24_000.0).exp();
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
    let a = 1.0 - (-std::f32::consts::TAU * cutoff / 24_000.0).exp();
    let mut z = 0.0;
    for _ in 0..4 {
        for x in &mut xs {
            z += (*x - z) * a;
            *x -= z;
        }
    }
    xs
}

fn band(xs: &[f32], low: f32, high: f32) -> Vec<f32> {
    lowpass4(highpass4(xs.to_vec(), low), high)
}

#[test]
fn vocal_riddim_uses_the_vocal_in_the_hook_and_the_growl() {
    let song = demo_song(10);
    assert_eq!(song.name, "Vocal Riddim");
    assert_eq!(song.name, DEMOS[10].name);
    assert_eq!(song.bpm, 140.0);
    assert_eq!(song.length_beats(), 64.0);

    let hook = track(&song, "Vocal Hook");
    assert_eq!(hook.instrument.kind, DeviceKind::VocalTake);
    let hook_take = hook.take.as_ref().expect("vocal hook has its take");
    assert!(
        hook_take.samples.len() > 60_000,
        "vocal hook take is unexpectedly short"
    );
    let hook_notes: usize = hook
        .clips
        .iter()
        .filter(|c| c.scene < 0)
        .map(|c| c.notes.len())
        .sum();
    assert!(
        hook_notes >= 64,
        "vocal hook is not chopped into a rhythm ({hook_notes} notes)"
    );
    let hook_end = hook
        .clips
        .iter()
        .filter(|c| c.scene < 0)
        .map(|c| c.end())
        .fold(0.0f64, f64::max);
    assert_eq!(hook_end, 64.0, "vocal hook is not in the full arrangement");

    let growl = track(&song, "Vocal Growl");
    assert_eq!(growl.instrument.kind, DeviceKind::VocalTake);
    assert!(growl.take.is_some(), "vocal growl lost its take");
    let chain: Vec<DeviceKind> = growl.effects.iter().map(|d| d.kind).collect();
    assert_eq!(
        chain,
        vec![
            DeviceKind::Bassify,
            DeviceKind::Wobble,
            DeviceKind::Compressor,
            DeviceKind::Eq3,
        ]
    );
    let growl_notes: usize = growl
        .clips
        .iter()
        .filter(|c| c.scene < 0)
        .map(|c| c.notes.len())
        .sum();
    assert!(
        growl_notes >= 32,
        "vocal growl has no rhythmic bassline ({growl_notes} notes)"
    );

    let mix = Engine::render_song(&song, 48_000.0);
    assert!(mix.iter().all(|s| s.is_finite()), "mix contains NaN/inf");
    let peak = mix.iter().fold(0.0f32, |a, s| a.max(s.abs()));
    assert!(peak > 0.05, "mix is silent (peak {peak})");
    assert!(peak < 1.0, "mix clips (peak {peak})");
    let seconds = mix.len() as f64 / 2.0 / 48_000.0;
    assert!(
        (28.0..=35.0).contains(&seconds),
        "unexpected mix length {seconds:.2}s"
    );

    let mix_mono = mono(&mix);
    let low = rms(&band(&mix_mono, 20.0, 180.0));
    let total = rms(&mix_mono);
    assert!(
        low / total.max(1e-9) > 0.12,
        "dubstep mix has no low-end weight"
    );

    let hook_solo = mono(&solo(&song, 0));
    let hook_mid = rms(&band(&hook_solo, 300.0, 4000.0));
    let hook_low = rms(&band(&hook_solo, 20.0, 180.0));
    assert!(
        hook_mid > hook_low * 1.2,
        "vocal hook is buried below its low end (mid {hook_mid:.4}, low {hook_low:.4})"
    );

    let growl_index = song
        .tracks
        .iter()
        .position(|t| t.name == "Vocal Growl")
        .expect("vocal growl index");
    let growl_solo = mono(&solo(&song, growl_index));
    let growl_low = rms(&band(&growl_solo, 20.0, 180.0));
    let growl_total = rms(&growl_solo);
    assert!(
        growl_low / growl_total.max(1e-9) > 0.15,
        "vocal growl did not become a bass"
    );
}

#[test]
fn vocal_riddim_project_saves_with_its_take_and_loads_again() {
    let song = demo_song(10);
    let path = std::env::temp_dir().join(format!(
        "harpsistudio-vocal-riddim-{}.harp",
        std::process::id()
    ));
    project::save_song_to(&song, &path).expect("save vocal riddim project");
    let loaded = project::load_song(&path).expect("load vocal riddim project");
    let _ = std::fs::remove_file(&path);

    assert_eq!(loaded.name, "Vocal Riddim");
    assert_eq!(loaded.tracks.len(), song.tracks.len());
    assert_eq!(loaded.length_beats(), 64.0);
    let hook = track(&loaded, "Vocal Hook");
    assert_eq!(
        hook.take.as_ref().expect("loaded hook take").samples.len(),
        track(&song, "Vocal Hook")
            .take
            .as_ref()
            .expect("source hook take")
            .samples
            .len()
    );
    let growl = track(&loaded, "Vocal Growl");
    assert_eq!(growl.instrument.kind, DeviceKind::VocalTake);
    assert!(growl.take.is_some());

    let samples = Engine::render_song(&loaded, 24_000.0);
    assert!(samples.iter().all(|s| s.is_finite()));
    let peak = samples.iter().fold(0.0f32, |a, s| a.max(s.abs()));
    assert!(peak > 0.02, "loaded vocal riddim is silent (peak {peak})");
}
