//! Standard MIDI File export for the arrangement.
//!
//! The exporter deliberately writes only the portable musical data: tempo,
//! track names, clip markers and notes. Instruments and effects remain in the
//! `.harp` project, which is the source of truth. The output is a format 1 MIDI
//! file with a conductor track followed by one MIDI track per song track.

use crate::params::DeviceKind;
use crate::song::{Clip, Song, Track};
use std::fs;
use std::path::Path;

pub const TICKS_PER_QUARTER: u16 = 480;
pub(crate) const DRUM_CHANNEL: u8 = 9;

struct Event {
    tick: u64,
    /// Note-offs and markers sort before note-ons at the same tick.
    order: u8,
    bytes: Vec<u8>,
}

/// Write a Standard MIDI File to `path`, creating its parent directory when
/// needed. The song is not modified.
pub fn write(song: &Song, path: &Path) -> Result<(), String> {
    let bytes = to_bytes(song)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("could not create {}: {e}", parent.display()))?;
        }
    }
    fs::write(path, bytes).map_err(|e| format!("could not write {}: {e}", path.display()))
}

/// Encode the arrangement as a format 1 Standard MIDI File. When there is no
/// arrangement, the first session clip on each track is exported, matching the
/// fallback used by the offline WAV renderer.
pub fn to_bytes(song: &Song) -> Result<Vec<u8>, String> {
    if !song.bpm.is_finite() || song.bpm <= 0.0 {
        return Err("cannot export MIDI: tempo must be a positive number".to_string());
    }
    let tempo = (60_000_000.0 / song.bpm).round();
    if tempo < 1.0 || tempo > 0xFF_FFFF as f64 {
        return Err(format!(
            "cannot export MIDI: tempo {} BPM is outside the MIDI range",
            song.bpm
        ));
    }

    let has_arrangement = song
        .tracks
        .iter()
        .any(|track| track.arrangement_clips().next().is_some());
    let mut chunks = vec![conductor_track(song, tempo as u32)];

    for track in &song.tracks {
        let clips: Vec<&Clip> = if has_arrangement {
            track.arrangement_clips().collect()
        } else {
            track
                .clips
                .iter()
                .filter(|clip| clip.scene >= 0)
                .take(1)
                .collect()
        };
        if clips.is_empty() {
            continue;
        }
        let channel = if track.instrument.kind == DeviceKind::DrumRack {
            DRUM_CHANNEL
        } else {
            0
        };
        chunks.push(render_track(track, &clips, channel, has_arrangement));
    }

    if chunks.len() == 1 {
        return Err("cannot export MIDI: the project has no clips".to_string());
    }
    if chunks.len() > u16::MAX as usize {
        return Err("cannot export MIDI: too many tracks".to_string());
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&(chunks.len() as u16).to_be_bytes());
    out.extend_from_slice(&TICKS_PER_QUARTER.to_be_bytes());

    for chunk in chunks {
        if chunk.len() > u32::MAX as usize {
            return Err("cannot export MIDI: a track is too large".to_string());
        }
        out.extend_from_slice(b"MTrk");
        out.extend_from_slice(&(chunk.len() as u32).to_be_bytes());
        out.extend_from_slice(&chunk);
    }
    Ok(out)
}

fn conductor_track(song: &Song, tempo: u32) -> Vec<u8> {
    let title = if song.name.trim().is_empty() {
        "Harpsistudio"
    } else {
        song.name.trim()
    };
    let mut events = vec![
        Event {
            tick: 0,
            order: 0,
            bytes: text_meta(0x03, title),
        },
        Event {
            tick: 0,
            order: 0,
            bytes: meta(
                0x51,
                &[(tempo >> 16) as u8, (tempo >> 8) as u8, tempo as u8],
            ),
        },
        // 4/4, 24 MIDI clocks per metronome click, eight 32nd notes per quarter.
        Event {
            tick: 0,
            order: 0,
            bytes: meta(0x58, &[4, 2, 24, 8]),
        },
    ];
    encode_track(&mut events)
}

fn render_track(track: &Track, clips: &[&Clip], channel: u8, arrangement: bool) -> Vec<u8> {
    let mut events = vec![Event {
        tick: 0,
        order: 0,
        bytes: text_meta(0x03, &track.name),
    }];

    for clip in clips {
        let clip_start = if arrangement {
            clip.start.max(0.0)
        } else {
            0.0
        };
        let clip_tick = beat_to_tick(clip_start);
        events.push(Event {
            tick: clip_tick,
            order: 0,
            bytes: text_meta(0x06, &clip.name),
        });

        for note in &clip.notes {
            let note_start = clip_start + note.start.max(0.0);
            let start_tick = beat_to_tick(note_start);
            let end_tick = beat_to_tick(note_start + note.len.max(0.0)).max(start_tick + 1);
            let pitch = note.pitch.min(127);
            let velocity = (note.vel * 127.0).round().clamp(1.0, 127.0) as u8;

            events.push(Event {
                tick: start_tick,
                order: 1,
                bytes: vec![0x90 | channel, pitch, velocity],
            });
            events.push(Event {
                tick: end_tick,
                order: 0,
                bytes: vec![0x80 | channel, pitch, 0],
            });
        }
    }

    encode_track(&mut events)
}

fn encode_track(events: &mut [Event]) -> Vec<u8> {
    events.sort_by_key(|event| (event.tick, event.order));
    let mut out = Vec::new();
    let mut previous_tick = 0u64;

    for event in events.iter() {
        push_vlq(&mut out, event.tick.saturating_sub(previous_tick));
        out.extend_from_slice(&event.bytes);
        previous_tick = event.tick;
    }

    push_vlq(&mut out, 0);
    out.extend_from_slice(&[0xFF, 0x2F, 0x00]);
    out
}

fn text_meta(event_type: u8, text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    out.push(0xFF);
    out.push(event_type);
    push_vlq(&mut out, bytes.len() as u64);
    out.extend_from_slice(bytes);
    out
}

fn meta(event_type: u8, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(0xFF);
    out.push(event_type);
    push_vlq(&mut out, data.len() as u64);
    out.extend_from_slice(data);
    out
}

fn push_vlq(out: &mut Vec<u8>, value: u64) {
    let mut buffer = [0u8; 10];
    let mut index = 0;
    let mut remaining = value;

    loop {
        buffer[index] = (remaining & 0x7F) as u8;
        index += 1;
        remaining >>= 7;
        if remaining == 0 {
            break;
        }
    }

    for position in (0..index).rev() {
        out.push(buffer[position] | if position > 0 { 0x80 } else { 0 });
    }
}

fn beat_to_tick(beat: f64) -> u64 {
    if !beat.is_finite() || beat <= 0.0 {
        0
    } else {
        (beat * TICKS_PER_QUARTER as f64).round() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::{Clip, Note};

    fn test_song() -> Song {
        let mut song = Song::empty("MIDI Test", 120.0);
        song.add_track(DeviceKind::SubBass);
        let mut clip = Clip::new("Bass", -1, 0.0, 4.0);
        clip.notes.push(Note::new(0.0, 1.0, 36, 0.8));
        clip.notes.push(Note::new(2.0, 0.5, 48, 1.0));
        song.tracks[0].clips.push(clip);
        song
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    #[test]
    fn writes_a_format_one_file_with_tempo_and_notes() {
        let bytes = to_bytes(&test_song()).expect("MIDI export");
        assert_eq!(&bytes[..4], b"MThd");
        assert_eq!(u16::from_be_bytes([bytes[8], bytes[9]]), 1);
        assert_eq!(u16::from_be_bytes([bytes[10], bytes[11]]), 2);
        assert_eq!(u16::from_be_bytes([bytes[12], bytes[13]]), 480);
        assert!(contains(&bytes, &[0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20]));
        assert!(contains(&bytes, b"Bass"));
        assert!(contains(&bytes, &[0x90, 36, 102]));
        assert!(contains(&bytes, &[0x80, 36, 0]));
    }

    #[test]
    fn session_clips_are_used_when_there_is_no_arrangement() {
        let mut song = test_song();
        song.tracks[0].clips[0].scene = 0;
        let bytes = to_bytes(&song).expect("session fallback");
        assert!(contains(&bytes, &[0x90, 36, 102]));
    }

    #[test]
    fn drum_tracks_use_the_percussion_channel() {
        let mut song = Song::empty("Drums", 120.0);
        song.add_track(DeviceKind::DrumRack);
        let mut clip = Clip::new("Kick", -1, 0.0, 4.0);
        clip.notes.push(Note::new(0.0, 1.0, 36, 1.0));
        song.tracks[0].clips.push(clip);
        let bytes = to_bytes(&song).expect("drum export");
        assert!(contains(&bytes, &[0x99, 36, 127]));
    }

    #[test]
    fn rejects_a_project_without_clips() {
        let song = Song::empty("Empty", 120.0);
        assert!(to_bytes(&song).is_err());
    }

    #[test]
    fn writes_variable_length_quantities() {
        let mut bytes = Vec::new();
        push_vlq(&mut bytes, 0x80);
        assert_eq!(bytes, [0x81, 0x00]);
        bytes.clear();
        push_vlq(&mut bytes, 0x0FFF_FFFF);
        assert_eq!(bytes, [0xFF, 0xFF, 0xFF, 0x7F]);
    }
}
