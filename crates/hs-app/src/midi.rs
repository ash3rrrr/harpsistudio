//! MIDI input: opens the first available hardware input port and forwards
//! note on/off, velocity and sustain (CC64) to the realtime engine.

use hs_core::engine::{Cmd, Shared};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Keeps the midir connection alive for as long as the app is running.
pub struct MidiHandle {
    pub name: String,
    _connection: midir::MidiInputConnection<()>,
}

pub fn start(shared: Arc<Shared>) -> Result<MidiHandle, String> {
    let input =
        midir::MidiInput::new("Harpsistudio input").map_err(|_| "No MIDI device".to_string())?;
    let ports = input.ports();
    let port = match ports.first() {
        Some(p) => p,
        None => return Err("No MIDI device".to_string()),
    };
    let name = input
        .port_name(port)
        .unwrap_or_else(|_| "MIDI input".to_string());

    let mut held: HashMap<u8, usize> = HashMap::new();
    let connection = input
        .connect(
            port,
            "harpsistudio-read",
            move |_stamp, message, _| {
                handle_message(&shared, &mut held, message);
            },
            (),
        )
        .map_err(|_| "No MIDI device".to_string())?;

    Ok(MidiHandle {
        name,
        _connection: connection,
    })
}

fn handle_message(shared: &Shared, held: &mut HashMap<u8, usize>, message: &[u8]) {
    if message.len() < 3 {
        return;
    }
    let status = message[0] & 0xF0;
    let data1 = message[1] & 0x7F;
    let data2 = message[2] & 0x7F;

    match status {
        0x90 if data2 > 0 => {
            let track = shared.live_track.load(Ordering::Relaxed);
            held.insert(data1, track);
            shared.send(Cmd::NoteOn {
                track,
                note: data1,
                vel: (data2 as f32 / 127.0).clamp(0.01, 1.0),
            });
        }
        0x80 | 0x90 => {
            if let Some(track) = held.remove(&data1) {
                shared.send(Cmd::NoteOff { track, note: data1 });
            }
        }
        0xB0 => match data1 {
            64 => shared.send(Cmd::Sustain(data2 >= 64)),
            123 => {
                held.clear();
                shared.send(Cmd::AllNotesOff);
            }
            _ => {}
        },
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hs_core::song::Song;

    #[test]
    fn forwards_notes_velocity_and_sustain_to_the_engine_queue() {
        let song = Song::empty("MIDI test", 120.0);
        let (shared, _arc, rx) = Shared::new(&song);
        shared.set_live_track(3);
        let mut held = HashMap::new();

        handle_message(&shared, &mut held, &[0x90, 60, 100]);
        match rx.try_recv().expect("note-on was not queued") {
            Cmd::NoteOn { track, note, vel } => {
                assert_eq!(track, 3);
                assert_eq!(note, 60);
                assert!((vel - 100.0 / 127.0).abs() < 1e-6);
            }
            other => panic!("expected note-on, got {other:?}"),
        }

        handle_message(&shared, &mut held, &[0x80, 60, 0]);
        match rx.try_recv().expect("note-off was not queued") {
            Cmd::NoteOff { track, note } => {
                assert_eq!(track, 3);
                assert_eq!(note, 60);
            }
            other => panic!("expected note-off, got {other:?}"),
        }

        handle_message(&shared, &mut held, &[0xB0, 64, 127]);
        assert!(matches!(rx.try_recv(), Ok(Cmd::Sustain(true))));
        handle_message(&shared, &mut held, &[0xB0, 64, 0]);
        assert!(matches!(rx.try_recv(), Ok(Cmd::Sustain(false))));
    }
}
