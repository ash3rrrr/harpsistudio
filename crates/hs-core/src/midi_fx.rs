//! MIDI-generating devices. These sit before the instrument in a track's
//! device chain and rewrite notes rather than audio.

use crate::params::{Device, DeviceKind};
use crate::song::Note;

const ARP_RATES: [f64; 4] = [1.0, 0.5, 0.25, 0.125];

const MAJOR: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const MINOR: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];
const DORIAN: [i32; 7] = [0, 2, 3, 5, 7, 9, 10];
const MIXOLYDIAN: [i32; 7] = [0, 2, 4, 5, 7, 9, 10];
const PHRYGIAN: [i32; 7] = [0, 1, 3, 5, 7, 8, 10];

/// Scale names in the same order as the chord generator's Scale parameter.
pub const SCALE_NAMES: [&str; 5] = ["Major", "Minor", "Dorian", "Mixolydian", "Phrygian"];

/// Stable fingerprint of a MIDI chain, used to avoid rebuilding note caches
/// on every publish when nothing changed.
pub fn chain_fingerprint(chain: &[Device]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for d in chain {
        h ^= DeviceKind::ALL
            .iter()
            .position(|k| *k == d.kind)
            .unwrap_or(0) as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        h ^= u64::from(d.on);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        for p in &d.params {
            h ^= u64::from(p.to_bits());
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    h
}

/// Apply every enabled MIDI device in chain order.
pub fn transform_notes(notes: &[Note], chain: &[Device], bpm: f64) -> Vec<Note> {
    let mut current = notes.to_vec();
    for dev in chain {
        if !dev.on {
            continue;
        }
        match dev.kind {
            DeviceKind::Arpeggiator => current = arpeggiate(&current, dev),
            DeviceKind::ChordGenerator => current = generate_chords(&current, dev),
            _ => {}
        }
        if current.len() > 1024 {
            current.truncate(1024);
        }
    }
    let _ = bpm;
    current
}

fn arpeggiate(notes: &[Note], dev: &Device) -> Vec<Note> {
    if notes.is_empty() {
        return Vec::new();
    }
    let rate = (dev.p(0).round().clamp(0.0, 3.0) as usize).min(3);
    let step = ARP_RATES[rate];
    let mode = (dev.p(1).round().clamp(0.0, 3.0) as usize).min(3);
    let octaves = dev.p(2).round().clamp(1.0, 4.0) as i32;
    let gate = dev.p(3).clamp(0.05, 1.0) as f64;
    let swing = dev.p(4).clamp(0.0, 0.6) as f64;

    let mut sorted = notes.to_vec();
    sorted.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.pitch.cmp(&b.pitch))
    });

    let mut out = Vec::new();
    let mut i = 0;
    let mut group_index: u32 = 0;
    while i < sorted.len() {
        let start_key = (sorted[i].start * 10_000.0).round() as i64;
        let mut j = i;
        while j < sorted.len() && (sorted[j].start * 10_000.0).round() as i64 == start_key {
            j += 1;
        }
        let group = &sorted[i..j];
        let start = group[0].start;
        let span = group.iter().map(|n| n.len).fold(0.0f64, f64::max);
        let vel = group.iter().map(|n| n.vel).fold(0.0f32, f32::max);

        let mut base: Vec<u8> = group.iter().map(|n| n.pitch).collect();
        base.sort_unstable();
        base.dedup();
        let mode_notes: Vec<u8> = match mode {
            0 => base.clone(),
            1 => {
                let mut v = base.clone();
                v.reverse();
                v
            }
            2 => {
                if base.len() <= 2 {
                    base.clone()
                } else {
                    let mut v = base.clone();
                    for p in base.iter().rev().skip(1).take(base.len() - 2) {
                        v.push(*p);
                    }
                    v
                }
            }
            _ => {
                let mut v = base.clone();
                let mut seed = 0x9e37_79b9u32 ^ group_index.wrapping_mul(2_654_435_761);
                for k in (1..v.len()).rev() {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    let swap = (seed as usize) % (k + 1);
                    v.swap(k, swap);
                }
                v
            }
        };
        if mode_notes.is_empty() {
            i = j;
            group_index = group_index.wrapping_add(1);
            continue;
        }

        let mut seq: Vec<u8> = Vec::new();
        if mode == 1 {
            for o in (0..octaves).rev() {
                for p in &mode_notes {
                    let v = *p as i32 + 12 * o;
                    if v <= 127 {
                        seq.push(v as u8);
                    }
                }
            }
        } else {
            for o in 0..octaves {
                for p in &mode_notes {
                    let v = *p as i32 + 12 * o;
                    if v <= 127 {
                        seq.push(v as u8);
                    }
                }
            }
        }
        if seq.is_empty() {
            i = j;
            group_index = group_index.wrapping_add(1);
            continue;
        }

        let steps = ((span / step).ceil() as usize).clamp(1, 256);
        for k in 0..steps {
            let offset = k as f64 * step;
            if offset >= span - 1e-9 {
                break;
            }
            let swing_off = if k % 2 == 1 { step * swing * 0.5 } else { 0.0 };
            let at = start + offset + swing_off;
            let len = (step * gate).min(span - offset).max(0.05);
            out.push(Note::new(at, len, seq[k % seq.len()], vel.clamp(0.05, 1.0)));
        }

        i = j;
        group_index = group_index.wrapping_add(1);
    }
    out.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.pitch.cmp(&b.pitch))
    });
    out
}

fn scale_intervals(index: usize) -> &'static [i32; 7] {
    match index {
        0 => &MAJOR,
        1 => &MINOR,
        2 => &DORIAN,
        3 => &MIXOLYDIAN,
        _ => &PHRYGIAN,
    }
}

/// True when `pitch` belongs to the selected key and scale. Used by the piano
/// roll for row highlighting and optional note snapping.
pub fn scale_contains(root: i32, scale: usize, pitch: i32) -> bool {
    in_scale(pitch, root.rem_euclid(12), scale_intervals(scale.min(4)))
}

/// Nearest pitch in the selected key and scale (preferring the lower one).
pub fn nearest_scale_pitch(root: i32, scale: usize, pitch: i32) -> i32 {
    snap_to_scale(
        pitch.clamp(0, 127),
        root.rem_euclid(12),
        scale_intervals(scale.min(4)),
    )
}

fn in_scale(pitch: i32, key: i32, scale: &[i32; 7]) -> bool {
    scale.contains(&(pitch - key).rem_euclid(12))
}

fn snap_to_scale(pitch: i32, key: i32, scale: &[i32; 7]) -> i32 {
    for d in 0..12 {
        let down = pitch - d;
        if down >= 0 && in_scale(down, key, scale) {
            return down;
        }
        let up = pitch + d;
        if up <= 127 && in_scale(up, key, scale) {
            return up;
        }
    }
    pitch
}

fn next_scale_pitch(pitch: i32, key: i32, scale: &[i32; 7]) -> i32 {
    let mut p = pitch + 1;
    while p <= 127 && !in_scale(p, key, scale) {
        p += 1;
    }
    p
}

fn degree_up(root: i32, steps: usize, key: i32, scale: &[i32; 7]) -> i32 {
    let mut p = root;
    for _ in 0..steps {
        p = next_scale_pitch(p, key, scale);
        if p > 127 {
            return 127;
        }
    }
    p
}

fn generate_chords(notes: &[Note], dev: &Device) -> Vec<Note> {
    let key = (dev.p(0).round() as i32).rem_euclid(12);
    let scale = scale_intervals((dev.p(1).round().clamp(0.0, 4.0) as usize).min(4));
    let quality = (dev.p(2).round().clamp(0.0, 5.0) as usize).min(5);
    let inversion = dev.p(3).round().clamp(0.0, 3.0) as usize;
    let octave = dev.p(4).round().clamp(-2.0, 2.0) as i32 * 12;

    let mut out = Vec::new();
    for n in notes {
        let root = snap_to_scale(n.pitch as i32, key, scale);
        let fifth = degree_up(root, 4, key, scale);
        let mut pitches: Vec<i32> = match quality {
            0 => vec![root, degree_up(root, 2, key, scale), fifth],
            1 => vec![root, root + 4, fifth],
            2 => vec![root, root + 3, fifth],
            3 => vec![
                root,
                degree_up(root, 2, key, scale),
                fifth,
                degree_up(root, 6, key, scale),
            ],
            4 => vec![root, degree_up(root, 3, key, scale), fifth],
            _ => vec![
                root,
                degree_up(root, 2, key, scale),
                fifth,
                degree_up(root, 8, key, scale),
            ],
        };
        pitches.sort_unstable();
        pitches.dedup();
        let inv = inversion.min(pitches.len().saturating_sub(1));
        for _ in 0..inv {
            if let Some(low) = pitches.first_mut() {
                *low += 12;
            }
            pitches.sort_unstable();
        }
        for p in pitches {
            let pitch = (p + octave).clamp(0, 127);
            out.push(Note::new(n.start, n.len, pitch as u8, n.vel));
        }
    }
    out.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.pitch.cmp(&b.pitch))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arp_device(rate: f32, mode: f32, octaves: f32, gate: f32) -> Device {
        let mut d = Device::new(DeviceKind::Arpeggiator);
        d.set(0, rate);
        d.set(1, mode);
        d.set(2, octaves);
        d.set(3, gate);
        d.set(4, 0.0);
        d
    }

    fn chord_device(quality: f32, inversion: f32) -> Device {
        let mut d = Device::new(DeviceKind::ChordGenerator);
        d.set(0, 0.0); // C
        d.set(1, 0.0); // Major
        d.set(2, quality);
        d.set(3, inversion);
        d.set(4, 0.0);
        d
    }

    #[test]
    fn arpeggiator_up_runs_scale_notes_in_time() {
        let notes = vec![
            Note::new(0.0, 2.0, 60, 0.8),
            Note::new(0.0, 2.0, 64, 0.8),
            Note::new(0.0, 2.0, 67, 0.8),
        ];
        let d = arp_device(2.0, 0.0, 1.0, 0.5);
        let out = arpeggiate(&notes, &d);
        assert_eq!(out.len(), 8);
        let pitches: Vec<u8> = out.iter().map(|n| n.pitch).collect();
        assert_eq!(pitches, vec![60, 64, 67, 60, 64, 67, 60, 64]);
        assert!((out[0].len - 0.125).abs() < 1e-9);
        assert!((out[1].start - 0.25).abs() < 1e-9);
        assert!(out.iter().all(|n| n.start + n.len <= 2.0 + 1e-9));
    }

    #[test]
    fn arpeggiator_down_updown_and_octaves_stay_musical() {
        let notes = vec![
            Note::new(0.0, 1.0, 60, 0.8),
            Note::new(0.0, 1.0, 64, 0.8),
            Note::new(0.0, 1.0, 67, 0.8),
        ];
        let down = arpeggiate(&notes, &arp_device(2.0, 1.0, 1.0, 0.5));
        assert_eq!(
            down.iter().map(|n| n.pitch).collect::<Vec<_>>(),
            vec![67, 64, 60, 67]
        );
        let long = vec![
            Note::new(0.0, 2.0, 60, 0.8),
            Note::new(0.0, 2.0, 64, 0.8),
            Note::new(0.0, 2.0, 67, 0.8),
        ];
        let updown = arpeggiate(&long, &arp_device(2.0, 2.0, 1.0, 0.5));
        assert_eq!(
            updown.iter().map(|n| n.pitch).collect::<Vec<_>>(),
            vec![60, 64, 67, 64, 60, 64, 67, 64]
        );
        let oct = arpeggiate(&long, &arp_device(2.0, 0.0, 2.0, 0.5));
        assert_eq!(oct[3].pitch, 72);
        assert!(oct.iter().all(|n| n.pitch >= 60 && n.pitch <= 79));
    }

    #[test]
    fn chord_generator_builds_diatonic_triads() {
        let notes = vec![Note::new(0.0, 2.0, 60, 0.8)];
        let triad = generate_chords(&notes, &chord_device(0.0, 0.0));
        assert_eq!(
            triad.iter().map(|n| n.pitch).collect::<Vec<_>>(),
            vec![60, 64, 67]
        );
        let seventh = generate_chords(&notes, &chord_device(3.0, 0.0));
        assert_eq!(
            seventh.iter().map(|n| n.pitch).collect::<Vec<_>>(),
            vec![60, 64, 67, 71]
        );
        let sus = generate_chords(&notes, &chord_device(4.0, 0.0));
        assert_eq!(
            sus.iter().map(|n| n.pitch).collect::<Vec<_>>(),
            vec![60, 65, 67]
        );
        let minor = generate_chords(&notes, &chord_device(2.0, 0.0));
        assert_eq!(
            minor.iter().map(|n| n.pitch).collect::<Vec<_>>(),
            vec![60, 63, 67]
        );
    }

    #[test]
    fn chord_generator_inverts_and_stays_in_key() {
        let notes = vec![Note::new(0.0, 2.0, 60, 0.8), Note::new(2.0, 2.0, 65, 0.8)];
        let inverted = generate_chords(&notes, &chord_device(0.0, 1.0));
        assert_eq!(
            inverted[0..3].iter().map(|n| n.pitch).collect::<Vec<_>>(),
            vec![64, 67, 72]
        );
        // Every generated root/fifth is diatonic to C major.
        for n in inverted {
            let pc = n.pitch % 12;
            assert!([0, 2, 4, 5, 7, 9, 11].contains(&pc));
        }
    }

    #[test]
    fn chord_generator_works_in_every_key() {
        for key in 0..12 {
            let mut d = chord_device(0.0, 0.0);
            d.set(0, key as f32);
            let notes = vec![Note::new(0.0, 1.0, 61, 0.8)];
            let out = generate_chords(&notes, &d);
            assert!(!out.is_empty(), "key {key} produced no chord");
            let pc = (out[0].pitch as i32 - key).rem_euclid(12);
            assert!(
                [0, 2, 4, 5, 7, 9, 11].contains(&pc),
                "key {key}: root {pc} is not in the major scale"
            );
        }
    }

    #[test]
    fn chain_order_chord_then_arp_generates_sixteenths() {
        let mut arp = arp_device(2.0, 0.0, 1.0, 0.5);
        arp.set(4, 0.0);
        let chain = vec![chord_device(0.0, 0.0), arp];
        let out = transform_notes(&[Note::new(0.0, 2.0, 60, 0.8)], &chain, 128.0);
        assert_eq!(out.len(), 8);
        assert!(out.iter().all(|n| n.len <= 0.125 + 1e-9));
        assert!(out.iter().any(|n| n.pitch == 64));
        assert!(out.iter().any(|n| n.pitch == 67));
    }

    #[test]
    fn scale_helpers_match_the_chord_generator() {
        for root in 0..12 {
            assert!(scale_contains(root, 0, root + 60));
            assert!(scale_contains(root, 0, root + 64));
            assert!(!scale_contains(root, 0, root + 61));
            assert!(scale_contains(root, 1, root + 63));
            assert!(!scale_contains(root, 1, root + 64));
        }
        assert_eq!(nearest_scale_pitch(0, 0, 61), 60);
        assert_eq!(nearest_scale_pitch(0, 0, 66), 65);
        assert_eq!(nearest_scale_pitch(9, 1, 60), 60);
        assert_eq!(nearest_scale_pitch(0, 0, 126), 125);
        assert_eq!(nearest_scale_pitch(0, 0, 127), 127);
    }

    #[test]
    fn bypassed_devices_do_not_change_notes() {
        let mut d = chord_device(0.0, 0.0);
        d.on = false;
        let notes = vec![Note::new(0.0, 1.0, 60, 0.8)];
        assert_eq!(transform_notes(&notes, &[d], 120.0), notes);
    }
}
