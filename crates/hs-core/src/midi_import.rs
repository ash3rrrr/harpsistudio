//! Standard MIDI File import for the arrangement.
//!
//! The importer reads the portable subset written by `midi_export`: tempo and
//! time-signature meta events, track names, clip markers and note on/off pairs.
//! MIDI does not carry the original instruments or effects, so melodic tracks
//! become Keys tracks and channel 10 tracks become Drum Rack tracks.

use crate::midi_export::{DRUM_CHANNEL, TICKS_PER_QUARTER};
use crate::params::DeviceKind;
use crate::song::{Clip, Note, Song};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug)]
struct ImportedNote {
    start: u64,
    end: u64,
    pitch: u8,
    velocity: u8,
    channel: u8,
}

#[derive(Debug)]
struct ImportedTrack {
    name: String,
    markers: Vec<(u64, String)>,
    notes: Vec<ImportedNote>,
    tempo_us: Option<u32>,
}

struct ParsedMidi {
    division: u16,
    tracks: Vec<ImportedTrack>,
}

/// Read a MIDI file and build a new song named after the file.
pub fn read(path: &Path) -> Result<Song, String> {
    let data = fs::read(path).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Imported MIDI");
    from_bytes(&data, name).map_err(|e| format!("could not import {}: {e}", path.display()))
}

/// Parse a format 0 or 1 Standard MIDI File into the Harpsistudio song model.
pub fn from_bytes(data: &[u8], name: &str) -> Result<Song, String> {
    let parsed = parse(data)?;
    let tempo_us = parsed
        .tracks
        .iter()
        .find_map(|track| track.tempo_us)
        .unwrap_or(500_000);
    if tempo_us == 0 {
        return Err("MIDI tempo is zero".to_string());
    }
    let bpm = 60_000_000.0 / tempo_us as f64;
    let title = if name.trim().is_empty() {
        "Imported MIDI"
    } else {
        name.trim()
    };
    let mut song = Song::empty(title, bpm);
    let mut note_count = 0usize;
    let mut imported_tracks = 0usize;

    for (index, track) in parsed.tracks.iter().enumerate() {
        if track.notes.is_empty() && track.markers.is_empty() {
            // The conductor track written by our exporter carries only tempo,
            // time signature and the song name.
            continue;
        }
        let kind = if track.notes.iter().any(|note| note.channel == DRUM_CHANNEL) {
            DeviceKind::DrumRack
        } else {
            DeviceKind::Keys
        };
        let track_index = song.add_track(kind);
        let destination = &mut song.tracks[track_index];
        destination.name = if track.name.trim().is_empty() {
            format!("Track {}", index + 1)
        } else {
            track.name.trim().to_string()
        };
        destination.clips = build_clips(track, parsed.division);
        note_count += track.notes.len();
        imported_tracks += 1;
    }

    if imported_tracks == 0 {
        return Err("MIDI file contains no tracks with notes or clip markers".to_string());
    }
    if note_count == 0 {
        return Err("MIDI file contains no note events".to_string());
    }
    song.normalize();
    Ok(song)
}

fn parse(data: &[u8]) -> Result<ParsedMidi, String> {
    if data.len() < 14 {
        return Err("MIDI header is truncated".to_string());
    }
    if &data[0..4] != b"MThd" {
        return Err("missing MThd header".to_string());
    }
    let header_len = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if header_len != 6 {
        return Err(format!("unsupported MIDI header length {header_len}"));
    }
    let format = u16::from_be_bytes([data[8], data[9]]);
    if format > 1 {
        return Err(format!(
            "unsupported MIDI format {format}; only 0 and 1 work"
        ));
    }
    let track_count = u16::from_be_bytes([data[10], data[11]]) as usize;
    if track_count == 0 {
        return Err("MIDI file contains no tracks".to_string());
    }
    let division = u16::from_be_bytes([data[12], data[13]]);
    if division & 0x8000 != 0 {
        return Err("SMPTE MIDI timing is not supported".to_string());
    }
    if division == 0 {
        return Err("MIDI ticks-per-quarter division is zero".to_string());
    }

    let mut pos = 14usize;
    let mut tracks = Vec::with_capacity(track_count);
    for index in 0..track_count {
        if pos + 8 > data.len() {
            return Err(format!("MIDI is truncated before track {}", index + 1));
        }
        if &data[pos..pos + 4] != b"MTrk" {
            return Err(format!("track {} is missing an MTrk header", index + 1));
        }
        let length =
            u32::from_be_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                as usize;
        let start = pos + 8;
        let end = start
            .checked_add(length)
            .filter(|end| *end <= data.len())
            .ok_or_else(|| format!("track {} is truncated", index + 1))?;
        tracks.push(parse_track(&data[start..end], index + 1)?);
        pos = end;
    }

    Ok(ParsedMidi { division, tracks })
}

fn parse_track(data: &[u8], number: usize) -> Result<ImportedTrack, String> {
    let mut pos = 0usize;
    let mut tick = 0u64;
    let mut running_status = None;
    let mut name = String::new();
    let mut markers = Vec::new();
    let mut notes = Vec::new();
    let mut active: HashMap<(u8, u8), Vec<(u64, u8)>> = HashMap::new();
    let mut tempo_us = None;
    let mut ended = false;

    while pos < data.len() {
        let delta = read_vlq(data, &mut pos).map_err(|error| format!("track {number}: {error}"))?;
        tick = tick
            .checked_add(delta)
            .ok_or_else(|| format!("track {number}: timestamp overflow"))?;
        if pos >= data.len() {
            return Err(format!("track {number}: truncated event at tick {tick}"));
        }

        let first = data[pos];
        let status = if first & 0x80 != 0 {
            pos += 1;
            if first < 0xF0 {
                running_status = Some(first);
            } else {
                running_status = None;
            }
            first
        } else {
            running_status
                .ok_or_else(|| format!("track {number}: running status without a channel status"))?
        };

        if status == 0xFF {
            if pos >= data.len() {
                return Err(format!("track {number}: truncated meta event"));
            }
            let meta_type = data[pos];
            pos += 1;
            let length =
                read_vlq(data, &mut pos).map_err(|error| format!("track {number}: {error}"))?;
            let length = usize::try_from(length)
                .map_err(|_| format!("track {number}: meta event is too large"))?;
            if pos + length > data.len() {
                return Err(format!("track {number}: truncated meta event payload"));
            }
            let payload = &data[pos..pos + length];
            pos += length;
            match meta_type {
                0x03 => name = String::from_utf8_lossy(payload).into_owned(),
                0x06 => markers.push((tick, String::from_utf8_lossy(payload).into_owned())),
                0x51 => {
                    if length != 3 {
                        return Err(format!("track {number}: tempo event must be 3 bytes"));
                    }
                    let value = u32::from_be_bytes([0, payload[0], payload[1], payload[2]]);
                    tempo_us = Some(value);
                }
                0x58 => {
                    if length < 2 {
                        return Err(format!(
                            "track {number}: time-signature event must be at least 2 bytes"
                        ));
                    }
                    let denominator = 1u16.checked_shl(payload[1] as u32).ok_or_else(|| {
                        format!("track {number}: invalid time-signature denominator")
                    })?;
                    let signature = (payload[0], denominator as u8);
                    if signature != (4, 4) {
                        return Err(format!(
                            "track {number}: only 4/4 time signatures are supported"
                        ));
                    }
                }
                0x2F => {
                    if length != 0 {
                        return Err(format!("track {number}: malformed end-of-track event"));
                    }
                    ended = true;
                    break;
                }
                _ => {}
            }
        } else if status == 0xF0 || status == 0xF7 {
            let length =
                read_vlq(data, &mut pos).map_err(|error| format!("track {number}: {error}"))?;
            let length = usize::try_from(length)
                .map_err(|_| format!("track {number}: sysex event is too large"))?;
            if pos + length > data.len() {
                return Err(format!("track {number}: truncated sysex event"));
            }
            pos += length;
        } else if (0x80..=0xEF).contains(&status) {
            let message_len = match status & 0xF0 {
                0x80 | 0x90 | 0xA0 | 0xB0 | 0xE0 => 2,
                0xC0 | 0xD0 => 1,
                _ => {
                    return Err(format!(
                        "track {number}: unsupported MIDI status {status:#04x}"
                    ))
                }
            };
            if pos + message_len > data.len() {
                return Err(format!("track {number}: truncated channel message"));
            }
            let first_data = data[pos];
            let second_data = (message_len == 2).then(|| data[pos + 1]);
            pos += message_len;
            let channel = status & 0x0F;
            match status & 0xF0 {
                0x90 if second_data.unwrap_or(0) > 0 => {
                    active
                        .entry((channel, first_data))
                        .or_default()
                        .push((tick, second_data.unwrap_or(0)));
                }
                0x80 | 0x90 => {
                    if let Some(stack) = active.get_mut(&(channel, first_data)) {
                        if let Some((start, velocity)) = stack.pop() {
                            notes.push(ImportedNote {
                                start,
                                end: tick.max(start + 1),
                                pitch: first_data,
                                velocity,
                                channel,
                            });
                        }
                    }
                }
                _ => {}
            }
        } else {
            return Err(format!(
                "track {number}: unsupported MIDI status {status:#04x}"
            ));
        }
    }

    if !ended {
        return Err(format!("track {number}: missing end-of-track event"));
    }

    for ((channel, pitch), stack) in active {
        for (start, velocity) in stack {
            notes.push(ImportedNote {
                start,
                end: tick.max(start + 1),
                pitch,
                velocity,
                channel,
            });
        }
    }
    notes.sort_by_key(|note| (note.start, note.pitch, note.channel));
    markers.sort_by_key(|(tick, _)| *tick);

    Ok(ImportedTrack {
        name,
        markers,
        notes,
        tempo_us,
    })
}

fn build_clips(track: &ImportedTrack, division: u16) -> Vec<Clip> {
    let mut seeds = track.markers.clone();
    if seeds.is_empty() {
        seeds.push((0, "Imported MIDI".to_string()));
    } else if track.notes.iter().any(|note| note.start < seeds[0].0) {
        seeds.insert(0, (0, "Imported MIDI".to_string()));
    }
    seeds.sort_by_key(|(tick, _)| *tick);

    let ticks_per_bar = TICKS_PER_QUARTER as u64 * 4;
    let mut clips = Vec::with_capacity(seeds.len());
    for (index, (start, name)) in seeds.iter().enumerate() {
        let next = seeds.get(index + 1).map(|(tick, _)| *tick);
        let mut clip_notes = Vec::new();
        let mut end = next.unwrap_or_else(|| start.saturating_add(ticks_per_bar));
        for note in &track.notes {
            let belongs = note.start >= *start
                && match next {
                    Some(next) => note.start < next,
                    None => true,
                };
            if !belongs {
                continue;
            }
            let local_start = note.start - *start;
            let note_length = note.end.max(note.start + 1) - note.start;
            clip_notes.push(Note::new(
                local_start as f64 / division as f64,
                note_length as f64 / division as f64,
                note.pitch,
                (note.velocity as f32 / 127.0).clamp(0.05, 1.0),
            ));
            end = end.max(note.end.max(note.start + 1));
        }
        let length = ((end.saturating_sub(*start)) as f64 / division as f64).max(0.25);
        let clip_name = if name.trim().is_empty() {
            "Imported Clip"
        } else {
            name.trim()
        };
        let mut clip = Clip::new(clip_name, -1, *start as f64 / division as f64, length);
        clip.notes = clip_notes;
        clips.push(clip);
    }
    clips
}

fn read_vlq(data: &[u8], pos: &mut usize) -> Result<u64, String> {
    let mut value = 0u64;
    for _ in 0..4 {
        let byte = *data
            .get(*pos)
            .ok_or_else(|| "truncated variable-length quantity".to_string())?;
        *pos += 1;
        value = (value << 7) | u64::from(byte & 0x7F);
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err("variable-length quantity is longer than four bytes".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vlq(mut value: u64) -> Vec<u8> {
        let mut bytes = vec![(value & 0x7F) as u8];
        value >>= 7;
        while value > 0 {
            bytes.push(((value & 0x7F) as u8) | 0x80);
            value >>= 7;
        }
        bytes.reverse();
        bytes
    }

    fn meta(event_type: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = vec![0, 0xFF, event_type];
        out.extend(vlq(payload.len() as u64));
        out.extend_from_slice(payload);
        out
    }

    fn smf(format: u16, tracks: &[Vec<u8>]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"MThd");
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&format.to_be_bytes());
        out.extend_from_slice(&(tracks.len() as u16).to_be_bytes());
        out.extend_from_slice(&480u16.to_be_bytes());
        for track in tracks {
            out.extend_from_slice(b"MTrk");
            out.extend_from_slice(&(track.len() as u32).to_be_bytes());
            out.extend_from_slice(track);
        }
        out
    }

    fn time_and_tempo() -> Vec<u8> {
        let mut out = meta(0x51, &[0x07, 0xA1, 0x20]);
        out.extend(meta(0x58, &[4, 2, 24, 8]));
        out
    }

    #[test]
    fn parses_running_status_and_note_pairs() {
        let mut track = time_and_tempo();
        track.extend(meta(0x03, b"Test"));
        track.extend(meta(0x06, b"Clip"));
        track.extend([0x00, 0x90, 60, 100]);
        track.extend(vlq(480));
        track.extend([64, 80]);
        track.extend([0x00, 0x80, 60, 0]);
        track.extend(vlq(480));
        track.extend([64, 0]);
        track.extend([0x00, 0xFF, 0x2F, 0x00]);
        let song = from_bytes(&smf(0, &[track]), "Running status").expect("import");
        assert!((song.bpm - 120.0).abs() < 1e-9);
        assert_eq!(song.tracks.len(), 1);
        assert_eq!(song.tracks[0].clips.len(), 1);
        let notes = &song.tracks[0].clips[0].notes;
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].pitch, 60);
        assert_eq!(notes[1].pitch, 64);
        assert!((notes[0].start - 0.0).abs() < 1e-9);
        assert!((notes[1].start - 1.0).abs() < 1e-9);
        assert!((notes[0].len - 1.0).abs() < 1e-9);
        assert!((notes[1].len - 1.0).abs() < 1e-9);
        assert!((notes[0].vel - 100.0 / 127.0).abs() < 1e-6);
    }

    #[test]
    fn maps_channel_ten_to_a_drum_rack() {
        let mut track = time_and_tempo();
        track.extend(meta(0x03, b"Drums"));
        track.extend(meta(0x06, b"Beat"));
        track.extend([0x00, 0x99, 36, 127]);
        track.extend(vlq(120));
        track.extend([0x89, 36, 0]);
        track.extend([0x00, 0xFF, 0x2F, 0x00]);
        let song = from_bytes(&smf(0, &[track]), "Drums").expect("import");
        assert_eq!(song.tracks[0].instrument.kind, DeviceKind::DrumRack);
        assert_eq!(song.tracks[0].name, "Drums");
    }

    #[test]
    fn rejects_truncated_files() {
        let mut track = time_and_tempo();
        track.extend(meta(0x03, b"Broken"));
        track.extend([0x00, 0x90, 60, 100]);
        let mut bytes = smf(0, &[track]);
        bytes.truncate(bytes.len() - 2);
        let error = from_bytes(&bytes, "Broken").err().expect("must reject");
        assert!(error.contains("truncated") || error.contains("missing end-of-track"));
    }

    #[test]
    fn rejects_an_unterminated_track() {
        let mut track = time_and_tempo();
        track.extend([0x00, 0x90, 60, 100]);
        let bytes = smf(0, &[track]);
        let error = from_bytes(&bytes, "Unterminated")
            .err()
            .expect("must reject");
        assert!(error.contains("missing end-of-track"));
    }

    #[test]
    fn rejects_a_midi_file_without_notes() {
        let mut track = time_and_tempo();
        track.extend([0x00, 0xFF, 0x2F, 0x00]);
        let error = from_bytes(&smf(0, &[track]), "Empty")
            .err()
            .expect("must reject");
        assert!(error.contains("no note events") || error.contains("no tracks with notes"));
    }
}
