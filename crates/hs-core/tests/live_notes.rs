use hs_core::engine::{Cmd, Engine, Shared};
use hs_core::params::DeviceKind;
use hs_core::song::{Clip, Note, Song};
use std::sync::Arc;

const SR: f32 = 48_000.0;

fn live_song() -> Song {
    let mut song = Song::empty("Live Test", 120.0);
    let track = song.add_track(DeviceKind::SubBass);
    // Keep the release test free of reverb/delay tails.
    song.tracks[track].send = 0.0;
    song.master_fx.clear();
    song.return_fx.on = false;
    song.limiter = false;
    song
}

fn rig(song: &Song) -> (Arc<Shared>, Engine) {
    let (shared, arc, rx) = Shared::new(song);
    let engine = Engine::new(SR, arc, Some(rx), Some(shared.clone()));
    (shared, engine)
}

fn render(engine: &mut Engine, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; frames * 2];
    engine.process(&mut out, 2);
    out
}

fn peak(buffer: &[f32]) -> f32 {
    buffer.iter().fold(0.0f32, |m, s| m.max(s.abs()))
}

fn assert_finite(buffer: &[f32]) {
    assert!(
        buffer.iter().all(|s| s.is_finite()),
        "buffer contained NaN/inf"
    );
}

fn assert_tail_silent(buffer: &[f32]) {
    let tail = &buffer[buffer.len().saturating_sub(9_600)..];
    let p = peak(tail);
    assert!(p < 1e-4, "note release tail still audible (peak {p})");
}

#[test]
fn live_note_renders_while_stopped_and_releases_to_silence() {
    let song = live_song();
    let (shared, mut engine) = rig(&song);
    assert!(!engine.playing);

    shared.send(Cmd::NoteOn {
        track: 0,
        note: 60,
        vel: 0.8,
    });
    let held = render(&mut engine, 9_600);
    assert_finite(&held);
    assert!(!engine.playing);
    assert!(peak(&held) > 0.005, "stopped live note was silent");

    shared.send(Cmd::NoteOff { track: 0, note: 60 });
    let tail = render(&mut engine, 96_000);
    assert_finite(&tail);
    assert_tail_silent(&tail);
}

#[test]
fn live_note_renders_while_transport_plays() {
    let song = live_song();
    let (shared, mut engine) = rig(&song);
    shared.send(Cmd::Play);
    shared.send(Cmd::NoteOn {
        track: 0,
        note: 67,
        vel: 0.8,
    });
    let held = render(&mut engine, 9_600);
    assert_finite(&held);
    assert!(engine.playing);
    assert!(peak(&held) > 0.005, "playing live note was silent");

    shared.send(Cmd::NoteOff { track: 0, note: 67 });
    let tail = render(&mut engine, 96_000);
    assert_finite(&tail);
    assert_tail_silent(&tail);
}

#[test]
fn live_note_does_not_release_a_clip_note_on_the_same_track() {
    let mut song = live_song();
    let mut clip = Clip::new("Held bass", 0, 0.0, 8.0);
    clip.notes.push(Note::new(0.0, 16.0, 60, 0.8));
    song.tracks[0].clips.push(clip);

    let (shared, mut engine) = rig(&song);
    shared.send(Cmd::Launch { track: 0, scene: 0 });
    let clip_only = render(&mut engine, 24_000);
    assert_finite(&clip_only);
    assert!(peak(&clip_only) > 0.005, "clip did not start");

    shared.send(Cmd::NoteOn {
        track: 0,
        note: 60,
        vel: 0.8,
    });
    let both = render(&mut engine, 9_600);
    assert_finite(&both);
    assert!(peak(&both) > 0.005, "live note did not play over the clip");

    shared.send(Cmd::NoteOff { track: 0, note: 60 });
    let after = render(&mut engine, 24_000);
    assert_finite(&after);
    assert!(
        peak(&after) > 0.005,
        "releasing the live note silenced the clip"
    );
}

#[test]
fn sustain_holds_a_note_until_the_pedal_is_released() {
    let song = live_song();
    let (shared, mut engine) = rig(&song);
    shared.send(Cmd::Sustain(true));
    shared.send(Cmd::NoteOn {
        track: 0,
        note: 55,
        vel: 0.8,
    });
    let held = render(&mut engine, 9_600);
    assert!(peak(&held) > 0.005);

    shared.send(Cmd::NoteOff { track: 0, note: 55 });
    let sustained = render(&mut engine, 24_000);
    assert_finite(&sustained);
    assert!(peak(&sustained) > 0.005, "sustain did not hold the note");

    shared.send(Cmd::Sustain(false));
    let tail = render(&mut engine, 96_000);
    assert_finite(&tail);
    assert_tail_silent(&tail);
}
