//! Export the delivery proof for Harpsistudio through the same core path the
//! GUI uses:
//!
//! - `presets::demo_song` is what the START cards and LOAD DEMO actions call.
//! - `project::export_wav_to` is the renderer/writer behind the Mixdown action
//!   (`project::export_wav` simply chooses a path and calls it).
//! - `presets::add_vocal_bass_track` is the helper behind the Voice -> Bass
//!   import path.
//!
//! Run with:
//!   cargo run -p hs-core --example render_demo_artifacts
//!
//! The resulting files live in `harpsistudio/demo-artifacts/`, outside
//! `target/`, so they are part of the project tree.

use hs_core::audio_take::AudioTake;
use hs_core::devices::DRUM_LANES;
use hs_core::engine::Engine;
use hs_core::params::{Device, DeviceKind};
use hs_core::presets::{self, VocalStyle};
use hs_core::project;
use hs_core::samples::{
    self, drum_kit_count, drum_lane_label, drum_sample_count, drum_sample_index,
};
use hs_core::song::{Clip, Note, Song};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const SR: f32 = 48_000.0;

#[derive(Clone)]
struct WavInfo {
    sample_rate: u32,
    channels: u16,
    bits: u16,
    duration_s: f64,
    peak: f32,
    rms: f32,
    dc: f32,
    clipped: usize,
    low_fraction: f32,
    mono: Vec<f32>,
}

fn mono(stereo: &[f32]) -> Vec<f32> {
    stereo
        .chunks(2)
        .map(|f| f.iter().sum::<f32>() / f.len() as f32)
        .collect()
}

fn rms(xs: &[f32]) -> f32 {
    (xs.iter().map(|x| x * x).sum::<f32>() / xs.len().max(1) as f32).sqrt()
}

fn mean(xs: &[f32]) -> f32 {
    xs.iter().sum::<f32>() / xs.len().max(1) as f32
}

fn lowpass4(mut xs: Vec<f32>, cutoff: f32) -> Vec<f32> {
    let a = 1.0 - (-std::f32::consts::TAU * cutoff / SR).exp();
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
    let a = 1.0 - (-std::f32::consts::TAU * cutoff / SR).exp();
    let mut z = 0.0;
    for _ in 0..4 {
        for x in &mut xs {
            z += (*x - z) * a;
            *x -= z;
        }
    }
    xs
}

fn band(xs: &[f32], lo: f32, hi: f32) -> Vec<f32> {
    lowpass4(highpass4(xs.to_vec(), lo), hi)
}

fn section_rms(mono: &[f32], bpm: f64) -> Vec<f32> {
    let frames = ((4.0 * 4.0) * 60.0 / bpm * SR as f64) as usize;
    mono.chunks(frames.max(1)).take(4).map(rms).collect()
}

fn env_cv(stereo: &[f32], win_ms: f32) -> f32 {
    let m = mono(stereo);
    let step = (SR * win_ms / 1000.0) as usize;
    let e: Vec<f32> = m.chunks(step.max(1)).map(rms).collect();
    let avg = mean(&e);
    let var = e.iter().map(|x| (x - avg).powi(2)).sum::<f32>() / e.len().max(1) as f32;
    var.sqrt() / avg.max(1e-9)
}

fn solo(song: &Song, track: usize) -> Vec<f32> {
    let mut s = song.clone();
    for (i, t) in s.tracks.iter_mut().enumerate() {
        t.mute = i != track;
    }
    Engine::render_song(&s, SR)
}

fn track_index(song: &Song, name: &str) -> usize {
    song.tracks
        .iter()
        .position(|t| t.name == name)
        .unwrap_or_else(|| panic!("missing track {name}"))
}

fn on_off_ratio(stereo: &[f32], bpm: f64, beats: usize) -> f32 {
    let m = mono(stereo);
    let beat = (60.0 / bpm * SR as f64) as usize;
    let mut on = 0.0;
    let mut off = 0.0;
    let mut n = 0usize;
    for b in 0..beats {
        let at = b * beat;
        let mid = (at + beat / 2).min(m.len());
        let end = (at + beat).min(m.len());
        if mid > at && end > mid {
            on += rms(&m[at..mid]).powi(2);
            off += rms(&m[mid..end]).powi(2);
            n += 1;
        }
    }
    (on / n.max(1) as f32).sqrt() / (off / n.max(1) as f32).sqrt().max(1e-9)
}

fn read_wav(path: &Path) -> WavInfo {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(bytes.len() >= 44, "short WAV: {}", path.display());
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");

    let mut pos = 12usize;
    let mut sample_rate = 0u32;
    let mut channels = 0u16;
    let mut bits = 0u16;
    let mut format = 0u16;
    let mut data_start = 0usize;
    let mut data_len = 0usize;

    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes(bytes[pos + 4..pos + 8].try_into().unwrap()) as usize;
        let start = pos + 8;
        let end = start.saturating_add(size).min(bytes.len());
        if id == b"fmt " && size >= 16 {
            format = u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap());
            channels = u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap());
            sample_rate = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap());
            bits = u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap());
        } else if id == b"data" {
            data_start = start;
            data_len = end.saturating_sub(start);
            break;
        }
        pos = start + size + (size & 1);
    }

    assert_eq!(format, 1, "expected PCM WAV: {}", path.display());
    assert_eq!(channels, 2, "expected stereo WAV: {}", path.display());
    assert_eq!(bits, 16, "expected 16-bit WAV: {}", path.display());
    assert!(sample_rate > 0, "missing sample rate: {}", path.display());

    let frame_bytes = channels as usize * 2;
    let frames = data_len / frame_bytes;
    let data = &bytes[data_start..data_start + frames * frame_bytes];
    let mut mono = Vec::with_capacity(frames);
    let mut clipped = 0usize;
    for frame in data.chunks_exact(frame_bytes) {
        let mut sum = 0i32;
        for ch in 0..channels as usize {
            let off = ch * 2;
            let raw = i16::from_le_bytes([frame[off], frame[off + 1]]);
            if raw == i16::MAX || raw == i16::MIN {
                clipped += 1;
            }
            sum += raw as i32;
        }
        mono.push(sum as f32 / channels as f32 / 32_768.0);
    }

    let dc = mean(&mono);
    let ac_rms = {
        let ac: Vec<f32> = mono.iter().map(|x| *x - dc).collect();
        rms(&ac)
    };
    let low = rms(&band(&mono, 20.0, 180.0));
    WavInfo {
        sample_rate,
        channels,
        bits,
        duration_s: frames as f64 / sample_rate as f64,
        peak: mono.iter().fold(0.0f32, |a, x| a.max(x.abs())),
        rms: ac_rms,
        dc,
        clipped,
        low_fraction: low / ac_rms.max(1e-9),
        mono,
    }
}

fn export(song: &Song, path: &Path) -> WavInfo {
    project::export_wav_to(song, path).unwrap_or_else(|e| panic!("export {}: {e}", path.display()));
    read_wav(path)
}

fn check_clean(info: &WavInfo, label: &str, min_seconds: f64) {
    assert!(
        info.duration_s >= min_seconds,
        "{label} is only {:.2}s",
        info.duration_s
    );
    assert!(
        info.mono.iter().all(|s| s.is_finite()),
        "{label} has NaN/inf"
    );
    assert!(info.peak > 0.01, "{label} is silent");
    assert!(
        info.peak < 0.9999,
        "{label} is too close to full scale (peak {})",
        info.peak
    );
    assert_eq!(info.clipped, 0, "{label} has clipped samples");
    assert!(
        info.dc.abs() < 0.001,
        "{label} has DC offset {:+.6}",
        info.dc
    );
}

fn main() {
    let app_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = app_dir.join("demo-artifacts");
    let renders = out.join("renders");
    let reports = out.join("reports");
    let source = out.join("source");
    fs::create_dir_all(&renders).expect("create renders directory");
    fs::create_dir_all(&reports).expect("create reports directory");
    fs::create_dir_all(&source).expect("create source directory");

    let demo_specs = [
        (1usize, "dubstep", "dubstep-riddim-protocol"),
        (0usize, "house", "house-midnight-house"),
        (2usize, "edm", "edm-big-room-anthem"),
        (7usize, "rap", "rap-cypher-room"),
    ];
    let mut demo_rows = Vec::new();
    let mut demo_infos = Vec::new();
    let mut demo_songs = Vec::new();
    let mut tsv = String::from(
        "file\tbpm\tbars\tduration_s\tpeak\tpeak_dbfs\trms\tdc\tclipped\tlow_fraction\n",
    );

    for (idx, genre, slug) in demo_specs {
        let song = presets::demo_song(idx);
        let bars = song.length_beats() / 4.0;
        assert!(
            bars >= 16.0,
            "{genre} demo is not a full 16-bar arrangement"
        );
        let path = renders.join(format!("{slug}.wav"));
        let info = export(&song, &path);
        check_clean(&info, genre, 20.0);
        assert!(info.rms > 0.01, "{genre} mix is too quiet");
        assert_eq!(info.sample_rate, 48_000);
        assert_eq!(info.channels, 2);
        assert_eq!(info.bits, 16);
        let db = 20.0 * info.peak.max(1e-9).log10();
        writeln!(
            &mut tsv,
            "{slug}.wav\t{:.0}\t{:.0}\t{:.3}\t{:.6}\t{:+.2}\t{:.6}\t{:+.7}\t{}\t{:.3}",
            song.bpm,
            bars,
            info.duration_s,
            info.peak,
            db,
            info.rms,
            info.dc,
            info.clipped,
            info.low_fraction
        )
        .unwrap();
        println!(
            "{:<8} {:>6.2}s  peak {:+.1} dBFS  RMS {:.4}  DC {:+.6}  clips {}  bass {:.3}",
            genre, info.duration_s, db, info.rms, info.dc, info.clipped, info.low_fraction
        );
        demo_rows.push((genre.to_string(), slug.to_string(), bars, info.clone()));
        demo_infos.push(info);
        demo_songs.push(song);
    }

    // Dubstep: low-end weight plus measurable wobble movement in the growl.
    let dubstep = &demo_songs[0];
    let growl_cv = env_cv(&solo(dubstep, track_index(dubstep, "Growl Bass")), 20.0);
    assert!(
        demo_infos[0].low_fraction > 0.35,
        "dubstep lost its low-end weight ({:.3})",
        demo_infos[0].low_fraction
    );
    assert!(
        growl_cv > 0.30,
        "dubstep growl lost its wobble movement (CV {growl_cv:.3})"
    );

    // House: every quarter note has a kick, and the beat windows carry more
    // low-band energy than the offbeats.
    let house = &demo_songs[1];
    let kick_notes = house
        .tracks
        .iter()
        .flat_map(|t| t.clips.iter())
        .filter(|c| c.scene < 0)
        .flat_map(|c| c.notes.iter())
        .filter(|n| n.pitch == 36)
        .count();
    let house_ratio = on_off_ratio(&solo(house, track_index(house, "Drums")), house.bpm, 64);
    assert!(kick_notes >= 60, "house has only {kick_notes} kick notes");
    assert!(
        house_ratio > 1.15,
        "house kick is not on the four-on-the-floor (ratio {house_ratio:.2})"
    );

    // EDM: intro -> drums-out build -> drop must actually gain energy.
    let edm_sections = section_rms(&demo_infos[2].mono, demo_songs[2].bpm);
    assert_eq!(edm_sections.len(), 4);
    assert!(
        edm_sections[2] > edm_sections[1] * 1.5,
        "EDM drop is not clearly above its build ({:.4} vs {:.4})",
        edm_sections[2],
        edm_sections[1]
    );
    assert!(
        edm_sections[2] > edm_sections[0] * 1.15,
        "EDM drop is not clearly above its intro ({:.4} vs {:.4})",
        edm_sections[2],
        edm_sections[0]
    );

    // Rap: the flow is a foreground part over the second half of the beat.
    let rap = &demo_songs[3];
    let rap_flow = track_index(rap, "Rap Flow");
    let flow_solo = mono(&solo(rap, rap_flow));
    let flow_start = (32.0 * 60.0 / rap.bpm * SR as f64) as usize;
    let flow_tail = &flow_solo[flow_start.min(flow_solo.len())..];
    let full_tail = &demo_infos[3].mono[flow_start.min(demo_infos[3].mono.len())..];
    let flow_ratio = rms(flow_tail) / rms(full_tail).max(1e-9);
    let flow_mid = rms(&band(flow_tail, 300.0, 4000.0));
    let flow_low = rms(&band(flow_tail, 20.0, 180.0));
    assert!(
        flow_ratio > 0.25,
        "rap flow is buried ({flow_ratio:.3} of the full mix)"
    );
    assert!(
        flow_mid > flow_low * 2.0,
        "rap flow has no vocal-range presence (mid {flow_mid:.4}, low {flow_low:.4})"
    );

    // Supplied vocal -> AudioTake -> clip -> Bassify/Wobble -> export.
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/voice-note.wav");
    let vocal_source = source.join("voice-note.wav");
    fs::copy(&fixture, &vocal_source).expect("copy supplied vocal fixture");
    let take = AudioTake::from_wav_file(&vocal_source).expect("supplied vocal imports");
    let mut vocal_song = Song::empty("Vocal Bass Delivery", 140.0);
    presets::add_vocal_bass_track(&mut vocal_song, take, VocalStyle::Dubstep);
    let vocal_track = vocal_song.tracks.last().unwrap();
    assert_eq!(vocal_track.instrument.kind, DeviceKind::VocalTake);
    assert!(vocal_track.take.is_some());
    let vocal_chain: Vec<DeviceKind> = vocal_track.effects.iter().map(|e| e.kind).collect();
    assert_eq!(
        vocal_chain,
        vec![
            DeviceKind::Bassify,
            DeviceKind::Wobble,
            DeviceKind::Compressor,
            DeviceKind::Eq3
        ]
    );
    let vocal_path = renders.join("vocal-to-dubstep-bass.wav");
    let vocal_info = export(&vocal_song, &vocal_path);
    check_clean(&vocal_info, "vocal bass", 1.0);
    assert!(
        vocal_info.low_fraction > 0.20,
        "vocal bass has no low-end energy ({:.3})",
        vocal_info.low_fraction
    );
    let vocal_db = 20.0 * vocal_info.peak.max(1e-9).log10();
    writeln!(
        &mut tsv,
        "vocal-to-dubstep-bass.wav\t{:.0}\t{:.2}\t{:.3}\t{:.6}\t{:+.2}\t{:.6}\t{:+.7}\t{}\t{:.3}",
        vocal_song.bpm,
        vocal_song.length_beats() / 4.0,
        vocal_info.duration_s,
        vocal_info.peak,
        vocal_db,
        vocal_info.rms,
        vocal_info.dc,
        vocal_info.clipped,
        vocal_info.low_fraction
    )
    .unwrap();

    // The shipped drum machine: 25 kits x 16 lanes, one hit for every mapping,
    // rendered through the same Mixdown writer. The report below proves all 155
    // embedded samples are reachable.
    let mut drum_song = Song::empty("Drum Library Audition", 120.0);
    for kit in 0..drum_kit_count() {
        let ti = drum_song.add_track(DeviceKind::DrumRack);
        let t = &mut drum_song.tracks[ti];
        t.name = format!("Kit {} · {}", kit + 1, samples::KIT_NAMES[kit]);
        t.volume = 0.55;
        t.send = 0.0;
        t.instrument.set(8, kit as f32);
        let mut c = Clip::new(&t.name, -1, kit as f64 * 8.0, 8.0);
        for (lane, (_name, note)) in DRUM_LANES.iter().enumerate() {
            c.notes
                .push(Note::new(lane as f64 * 0.5, 0.15, *note, 0.85));
        }
        t.clips.push(c);
    }
    let drum_path = renders.join("drum-library-audition.wav");
    let drum_info = export(&drum_song, &drum_path);
    check_clean(&drum_info, "drum-library audition", 40.0);
    let drum_db = 20.0 * drum_info.peak.max(1e-9).log10();
    writeln!(
        &mut tsv,
        "drum-library-audition.wav\t120\t{:.0}\t{:.3}\t{:.6}\t{:+.2}\t{:.6}\t{:+.7}\t{}\t{:.3}",
        drum_song.length_beats() / 4.0,
        drum_info.duration_s,
        drum_info.peak,
        drum_db,
        drum_info.rms,
        drum_info.dc,
        drum_info.clipped,
        drum_info.low_fraction
    )
    .unwrap();

    let mut drum_tsv = String::from("kit\tkit_name\tlane\tlabel\tmidi_note\tsample_index\n");
    let mut seen = std::collections::BTreeSet::new();
    for kit in 0..drum_kit_count() {
        for (lane, (_, note)) in DRUM_LANES.iter().enumerate() {
            let label = drum_lane_label(kit, lane);
            let index = drum_sample_index(kit, lane).expect("kit lane has a sample");
            assert!(samples::drum(kit, lane).is_some(), "missing sample");
            seen.insert(index);
            writeln!(
                &mut drum_tsv,
                "{kit}\t{}\t{lane}\t{label}\t{}\t{index}",
                samples::KIT_NAMES[kit],
                note
            )
            .unwrap();
        }
    }
    assert_eq!(drum_kit_count(), 25);
    assert_eq!(drum_sample_count(), 155);
    assert_eq!(
        seen.len(),
        155,
        "not every embedded drum sample is reachable"
    );

    // Salamander piano + Chord Generator + Arpeggiator, playing alongside a
    // real drum kit through the export path.
    let mut midi_song = Song::empty("Salamander MIDI Groove", 120.0);
    let drums_ti = midi_song.add_track(DeviceKind::DrumRack);
    midi_song.tracks[drums_ti].name = "House Drums".to_string();
    midi_song.tracks[drums_ti].volume = 0.65;
    midi_song.tracks[drums_ti].send = 0.06;
    midi_song.tracks[drums_ti].instrument.set(8, 1.0); // House Club
    let mut drum_clip = Clip::new("House Drums", -1, 0.0, 32.0);
    for bar in 0..8 {
        let o = bar as f64 * 4.0;
        for b in 0..4 {
            drum_clip.notes.push(Note::new(o + b as f64, 0.2, 36, 0.95));
        }
        for b in [1.0, 3.0] {
            drum_clip.notes.push(Note::new(o + b, 0.2, 38, 0.85));
        }
        for s in 0..8 {
            drum_clip
                .notes
                .push(Note::new(o + s as f64 * 0.5, 0.08, 42, 0.22));
        }
    }
    midi_song.tracks[drums_ti].clips.push(drum_clip);

    let piano_ti = midi_song.add_track(DeviceKind::Salamander);
    midi_song.tracks[piano_ti].name = "Salamander Arp".to_string();
    midi_song.tracks[piano_ti].volume = 0.70;
    midi_song.tracks[piano_ti].send = 0.22;
    midi_song.tracks[piano_ti].instrument =
        Device::from_preset(DeviceKind::Salamander, &DeviceKind::Salamander.presets()[0]);
    let mut chord = Device::new(DeviceKind::ChordGenerator);
    chord.set(0, 0.0); // C
    chord.set(1, 0.0); // Major
    chord.set(2, 3.0); // 7th
    let mut arp = Device::new(DeviceKind::Arpeggiator);
    arp.set(0, 2.0); // 1/16
    arp.set(1, 0.0); // Up
    arp.set(2, 2.0); // two octaves
    arp.set(3, 0.5); // gate
    midi_song.tracks[piano_ti].midi_fx = vec![chord, arp];
    let mut piano_clip = Clip::new("Salamander Arp", -1, 0.0, 32.0);
    piano_clip.notes.push(Note::new(0.0, 31.5, 60, 0.9));
    midi_song.tracks[piano_ti].clips.push(piano_clip);

    let midi_path = renders.join("salamander-midi-groove.wav");
    let midi_info = export(&midi_song, &midi_path);
    check_clean(&midi_info, "Salamander MIDI groove", 10.0);
    assert!(midi_info.rms > 0.005, "Salamander MIDI groove is too quiet");
    let midi_db = 20.0 * midi_info.peak.max(1e-9).log10();
    writeln!(
        &mut tsv,
        "salamander-midi-groove.wav\t120\t8\t{:.3}\t{:.6}\t{:+.2}\t{:.6}\t{:+.7}\t{}\t{:.3}",
        midi_info.duration_s,
        midi_info.peak,
        midi_db,
        midi_info.rms,
        midi_info.dc,
        midi_info.clipped,
        midi_info.low_fraction
    )
    .unwrap();

    fs::write(reports.join("render-metrics.tsv"), tsv).expect("write render metrics");
    fs::write(reports.join("drum-library.tsv"), drum_tsv).expect("write drum library report");

    let mut report = String::new();
    report.push_str("# Harpsistudio delivery report\n\n");
    report.push_str("Generated by `cargo run -p hs-core --example render_demo_artifacts`.\n\n");
    report.push_str("## Path used\n\n");
    report.push_str("- Starters/demos: `presets::demo_song(index)` — the same factory songs loaded by the START cards and `Act::LoadDemo`.\n");
    report.push_str("- Mixdown: `project::export_wav_to` — the function called by `project::export_wav`, which `Act::Export` invokes.\n");
    report.push_str("- Vocal: `AudioTake::from_wav_file` → `presets::add_vocal_bass_track` — the same import/install path as Voice → Bass.\n");
    writeln!(
        report,
        "- Drum library: {} Drum Rack kits × 16 lanes through `Engine::render_song` and the same WAV writer.\n",
        drum_kit_count()
    )
    .unwrap();
    report.push_str("## Render contract\n\n");
    report.push_str(
        "| file | bpm | bars | seconds | peak dBFS | RMS | DC | clipped | low fraction |\n",
    );
    report.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for (i, (genre, slug, bars, info)) in demo_rows.iter().enumerate() {
        writeln!(
            report,
            "| `{slug}.wav` ({genre}) | {:.0} | {:.0} | {:.2} | {:+.1} | {:.4} | {:+.6} | {} | {:.3} |",
            demo_songs[i].bpm,
            bars,
            info.duration_s,
            20.0 * info.peak.max(1e-9).log10(),
            info.rms,
            info.dc,
            info.clipped,
            info.low_fraction
        )
        .unwrap();
    }
    writeln!(
        report,
        "| `vocal-to-dubstep-bass.wav` | 140 | {:.2} | {:.2} | {:+.1} | {:.4} | {:+.6} | {} | {:.3} |",
        vocal_song.length_beats() / 4.0,
        vocal_info.duration_s,
        vocal_db,
        vocal_info.rms,
        vocal_info.dc,
        vocal_info.clipped,
        vocal_info.low_fraction
    )
    .unwrap();
    writeln!(
        report,
        "| `drum-library-audition.wav` | 120 | {:.0} | {:.2} | {:+.1} | {:.4} | {:+.6} | {} | {:.3} |",
        drum_song.length_beats() / 4.0,
        drum_info.duration_s,
        drum_db,
        drum_info.rms,
        drum_info.dc,
        drum_info.clipped,
        drum_info.low_fraction
    )
    .unwrap();
    writeln!(
        report,
        "| `salamander-midi-groove.wav` | 120 | 8 | {:.2} | {:+.1} | {:.4} | {:+.6} | {} | {:.3} |",
        midi_info.duration_s,
        midi_db,
        midi_info.rms,
        midi_info.dc,
        midi_info.clipped,
        midi_info.low_fraction
    )
    .unwrap();
    report.push_str("\nAll rendered files are 48 kHz, 16-bit stereo PCM. Peak must be below 0 dBFS and `clipped` counts raw PCM samples at i16 full scale.\n\n");

    report.push_str("## Genre evidence\n\n");
    writeln!(
        report,
        "- **Dubstep** — 16 bars / 30.9 s, low-band (20–180 Hz) RMS fraction {:.3}, growl-stem 20 ms envelope modulation CV {:.3}, Salamander piano hook in the last eight bars. The growl device's 4 Hz wobble is the movement behind the CV.\n",
        demo_infos[0].low_fraction, growl_cv
    )
    .unwrap();
    writeln!(
        report,
        "- **House** — 16 bars / 34.8 s, {} kick notes on the arrangement's quarter-note grid; drum-stem on-beat/offbeat low-band energy ratio {:.2}; 8 bars of supersaw lead over the offbeat sub and Am7 stabs.\n",
        kick_notes, house_ratio
    )
    .unwrap();
    writeln!(
        report,
        "- **EDM** — 16 bars / 33.8 s. Section RMS (intro, drums-out build, drop, drop tail): {:.4}, {:.4}, {:.4}, {:.4}. The drop is {:.1}× the build RMS; full drums, sub, supersaw chords and the pluck lead return at bar 9.\n",
        edm_sections[0],
        edm_sections[1],
        edm_sections[2],
        edm_sections[3],
        edm_sections[2] / edm_sections[1].max(1e-9)
    )
    .unwrap();
    writeln!(
        report,
        "- **Rap** — 16 bars / 48.0 s. The dedicated Rap Flow enters at bar 9 and is {:.3} of the full-mix RMS over that section; its 300–4000 Hz RMS {:.4} is {:.2}× its 20–180 Hz RMS. Boom-bap drums, felt Salamander piano, sample bass and the sliding 808 stay underneath.\n",
        flow_ratio, flow_mid, flow_mid / flow_low.max(1e-9)
    )
    .unwrap();
    writeln!(
        report,
        "- **Vocal → dubstep bass** — supplied `voice-note.wav` imported as an `AudioTake`, then `Vocal Take → Bassify → Wobble → Compressor → Eq3`; output low-band fraction {:.3}, peak {:.1} dBFS, DC {:+.7}, zero clipped samples.\n",
        vocal_info.low_fraction, vocal_db, vocal_info.dc
    )
    .unwrap();
    writeln!(
        report,
        "- **Drum machine** — {} kits, {} embedded samples, {} kit-lane mappings; this run reached all {} unique samples. The audition file is one 8-beat pass per kit. The report `drum-library.tsv` lists every mapping.\n",
        drum_kit_count(), drum_sample_count(), drum_kit_count() * 16, seen.len()
    )
    .unwrap();
    writeln!(
        report,
        "- **Salamander + MIDI FX** — the `salamander-midi-groove.wav` proof combines a House Club Drum Rack with a Salamander Grand track through Chord Generator → Arpeggiator. Peak {:.1} dBFS, RMS {:.4}, DC {:+.7}, zero clipped samples.\n",
        midi_db, midi_info.rms, midi_info.dc
    )
    .unwrap();
    report.push_str("\n## Verification\n\n");
    report.push_str("- Every demo is at least 16 bars (64 beats) and is rendered with the same 8-beat tail as Mixdown.\n");
    report.push_str("- Every file is finite, non-silent, below full scale, has zero clipped PCM samples, and DC below 0.001.\n");
    report.push_str("- The example asserts these properties before writing the reports, so a future regression fails the proof command rather than silently shipping worse audio.\n");
    fs::write(reports.join("delivery-report.md"), report).expect("write delivery report");

    println!("\nartifacts: {}", out.display());
    println!("reports:   {}", reports.display());
}
