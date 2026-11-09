//! Regression: publishing an edited clip must refresh the engine's MIDI note
//! cache. The cache was keyed only by the MIDI-FX chain and clip count, so
//! piano-roll note edits stayed stale until the chain changed or a clip was
//! added or removed.

use hs_core::engine::{Engine, Shared};
use hs_core::params::{Device, DeviceKind};
use hs_core::song::{Clip, Note, Song};
use std::sync::Arc;

const SR: f32 = 48_000.0;

fn song_with_pitch(pitch: u8, arpeggiated: bool) -> Song {
    let mut song = Song::empty("Cache regression", 120.0);
    let ti = song.add_track(DeviceKind::Supersaw);
    song.tracks[ti].volume = 0.8;
    if arpeggiated {
        let mut arp = Device::new(DeviceKind::Arpeggiator);
        arp.set(0, 2.0); // 1/16
        arp.set(1, 0.0); // Up
        arp.set(2, 1.0);
        arp.set(3, 0.5);
        song.tracks[ti].midi_fx.push(arp);
    }
    // One arrangement clip, so note edits do not change the clip count.
    let mut clip = Clip::new("Riff", -1, 0.0, 4.0);
    clip.notes.push(Note::new(0.0, 1.0, pitch, 0.9));
    song.tracks[ti].clips.push(clip);
    song
}

fn rms(xs: &[f32]) -> f32 {
    (xs.iter().map(|x| x * x).sum::<f32>() / xs.len().max(1) as f32).sqrt()
}

fn render(song: &Song) -> Vec<f32> {
    let mut engine = Engine::new(SR, Arc::new(song.clone()), None, None);
    engine.playing = true;
    engine.beat = 0.0;
    let mut out = vec![0.0; 48_000];
    engine.process(&mut out, 2);
    out
}

/// Build one engine from `original`, publish `edited`, and render. A second
/// engine is built directly from `edited` as the reference.
fn render_after_publish(original: &Song, edited: &Song) -> (Vec<f32>, Vec<f32>) {
    let (shared, _arc, _rx) = Shared::new(original);
    let mut live = Engine::new(SR, Arc::new(original.clone()), None, Some(shared.clone()));
    shared.publish(edited);
    let mut direct = Engine::new(SR, Arc::new(edited.clone()), None, None);
    live.playing = true;
    live.beat = 0.0;
    direct.playing = true;
    direct.beat = 0.0;
    let mut live_out = vec![0.0; 48_000];
    let mut direct_out = vec![0.0; 48_000];
    live.process(&mut live_out, 2);
    direct.process(&mut direct_out, 2);
    (live_out, direct_out)
}

#[test]
fn note_edits_reach_the_live_engine_without_a_chain_change() {
    for arpeggiated in [false, true] {
        let original = song_with_pitch(60, arpeggiated);
        let mut edited = original.clone();
        edited.tracks[0].clips[0].notes[0].pitch = 72;

        let before = render(&original);
        let (live, direct) = render_after_publish(&original, &edited);

        let pitch_changed = before
            .iter()
            .zip(&direct)
            .any(|(a, b)| (a - b).abs() > 1e-4);
        assert!(
            pitch_changed,
            "editing the pitch did not change the reference render (arp {arpeggiated})"
        );
        assert!(
            rms(&live) > 0.001,
            "live render is silent (arp {arpeggiated})"
        );

        let max_diff = live
            .iter()
            .zip(&direct)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(
            max_diff < 1e-6,
            "live engine rendered stale notes after publish (arp {arpeggiated}, max diff {max_diff})"
        );
    }
}
