//! Per-one-shot render coverage and kit-to-kit level measurements.
//!
//! These probes use the real Drum Rack voice path at 48 kHz. The master bus is
//! bypassed while measuring so the limiter cannot hide a hot sample or make a
//! spread look narrower than it really is.

use hs_core::devices::DRUM_LANES;
use hs_core::engine::Engine;
use hs_core::params::DeviceKind;
use hs_core::samples;
use hs_core::song::{Clip, Note, Song};
use std::sync::OnceLock;

const PROBE_SR: f32 = 48_000.0;
const PROBE_BPM: f64 = 480.0;
const NOTE_VELOCITY: f32 = 0.9;
const SHORT_RMS_SECONDS: f32 = 0.050;

#[derive(Clone, Copy)]
struct Probe {
    kit: usize,
    lane: usize,
    sample: usize,
    peak: f32,
    short_rms: f32,
    dc: f32,
}

fn probe_song(kit: usize, lane: usize, bypass_master: bool) -> Song {
    let preset = &DeviceKind::DrumRack.presets()[kit];
    let mut song = Song::empty("Drum measurement", PROBE_BPM);
    if bypass_master {
        song.master_fx.clear();
        song.return_fx.on = false;
        song.limiter = false;
        song.master_volume = 1.0;
    }

    let track = song.add_track_preset(DeviceKind::DrumRack, preset);
    song.tracks[track].instrument.set(8, kit as f32);
    song.tracks[track].send = 0.0;

    let mut clip = Clip::new("probe", -1, 0.0, 0.5);
    clip.notes
        .push(Note::new(0.0, 0.5, DRUM_LANES[lane].1, NOTE_VELOCITY));
    song.tracks[track].clips.push(clip);
    song
}

fn measure(samples: &[f32]) -> (f32, f32, f32) {
    assert_eq!(samples.len() % 2, 0, "engine output must be stereo");
    let frames = samples.len() / 2;
    assert!(frames > 0, "engine rendered no frames");

    let mut peak = 0.0f32;
    let mut sum = 0.0f64;
    let mut energy = Vec::with_capacity(frames);
    for frame in 0..frames {
        let l = samples[frame * 2];
        let r = samples[frame * 2 + 1];
        assert!(l.is_finite() && r.is_finite(), "render contains NaN/inf");
        peak = peak.max(l.abs()).max(r.abs());
        sum += (l as f64 + r as f64) * 0.5;
        energy.push((l * l + r * r) * 0.5);
    }

    let window = ((PROBE_SR * SHORT_RMS_SECONDS) as usize).max(1).min(frames);
    let mut running = energy[..window].iter().sum::<f32>();
    let mut max_energy = running;
    for i in window..frames {
        running += energy[i] - energy[i - window];
        max_energy = max_energy.max(running);
    }
    let short_rms = (max_energy / window as f32).sqrt();
    let dc = (sum / frames as f64) as f32;
    (peak, short_rms, dc)
}

fn probes() -> &'static [Probe] {
    static PROBES: OnceLock<Vec<Probe>> = OnceLock::new();
    PROBES.get_or_init(|| {
        samples::preload();
        let mut result = Vec::with_capacity(samples::drum_kit_count() * DRUM_LANES.len());
        for kit in 0..samples::drum_kit_count() {
            for lane in 0..DRUM_LANES.len() {
                let sample = samples::drum_sample_index(kit, lane)
                    .unwrap_or_else(|| panic!("kit {kit} lane {lane} has no sample index"));
                let output = Engine::render_song(&probe_song(kit, lane, true), PROBE_SR);
                let (peak, short_rms, dc) = measure(&output);
                result.push(Probe {
                    kit,
                    lane,
                    sample,
                    peak,
                    short_rms,
                    dc,
                });
            }
        }
        result
    })
}

fn db(value: f32) -> f32 {
    20.0 * value.max(1.0e-12).log10()
}

fn median(values: &mut [f32]) -> f32 {
    values.sort_by(f32::total_cmp);
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) * 0.5
    } else {
        values[mid]
    }
}

#[test]
fn every_embedded_drum_sample_renders_individually() {
    let probes = probes();
    let expected_files = samples::drum_sample_count();
    assert_eq!(expected_files, 155);
    assert_eq!(samples::drum_kit_count(), 13);
    assert_eq!(probes.len(), 13 * 16);

    let mut seen = vec![false; expected_files];
    let mut min_rms = f32::MAX;
    let mut max_peak = 0.0f32;
    let mut max_dc = 0.0f32;
    let mut min_master_rms = f32::MAX;
    let mut max_master_peak = 0.0f32;
    for probe in probes {
        assert!(
            probe.sample < expected_files,
            "kit {} lane {} points at sample {}",
            probe.kit,
            probe.lane,
            probe.sample
        );
        assert!(
            probe.peak.is_finite() && probe.short_rms.is_finite() && probe.dc.is_finite(),
            "kit {} lane {} sample {} produced non-finite output",
            probe.kit,
            probe.lane,
            probe.sample
        );
        assert!(
            probe.peak > 0.001,
            "kit {} lane {} sample {} rendered silence",
            probe.kit,
            probe.lane,
            probe.sample
        );
        assert!(
            probe.short_rms > 0.0002,
            "kit {} lane {} sample {} is effectively inaudible (50 ms RMS {:.6})",
            probe.kit,
            probe.lane,
            probe.sample,
            probe.short_rms
        );
        assert!(
            probe.peak < 0.90,
            "kit {} lane {} sample {} leaves too little headroom (peak {:.4})",
            probe.kit,
            probe.lane,
            probe.sample,
            probe.peak
        );
        assert!(
            probe.dc.abs() < 0.001,
            "kit {} lane {} sample {} has excessive DC offset ({:.5})",
            probe.kit,
            probe.lane,
            probe.sample,
            probe.dc
        );
        // The shipped path adds the default master compressor and limiter.
        // Check it too, rather than only proving the isolated sampler bus. A
        // probe that reaches the limiter's 0.92 target would mean the
        // source needed limiting, so the test requires real headroom.
        let master_output =
            Engine::render_song(&probe_song(probe.kit, probe.lane, false), PROBE_SR);
        let (master_peak, master_rms, master_dc) = measure(&master_output);
        assert!(
            master_peak < 0.92,
            "kit {} lane {} sample {} reaches the master limiter (peak {:.4})",
            probe.kit,
            probe.lane,
            probe.sample,
            master_peak
        );
        assert!(
            master_rms > 0.0002,
            "kit {} lane {} sample {} is inaudible through the master path (RMS {:.6})",
            probe.kit,
            probe.lane,
            probe.sample,
            master_rms
        );
        assert!(
            master_dc.abs() < 0.001,
            "kit {} lane {} sample {} has DC offset through the master path ({:.5})",
            probe.kit,
            probe.lane,
            probe.sample,
            master_dc
        );
        seen[probe.sample] = true;
        min_rms = min_rms.min(probe.short_rms);
        max_peak = max_peak.max(probe.peak);
        max_dc = max_dc.max(probe.dc.abs());
        min_master_rms = min_master_rms.min(master_rms);
        max_master_peak = max_master_peak.max(master_peak);
    }

    let missing: Vec<_> = seen
        .iter()
        .enumerate()
        .filter_map(|(index, covered)| (!covered).then_some(index))
        .collect();
    assert!(
        missing.is_empty(),
        "embedded drum files not covered by any probe: {missing:?}"
    );

    println!(
        "PER_SAMPLE_COVERAGE unique={} probes={} min_50ms_rms={:.6} ({:.2} dBFS) max_peak={:.6} ({:.2} dBFS) max_dc={:.6} master_min_rms={:.6} ({:.2} dBFS) master_max_peak={:.6} ({:.2} dBFS)",
        seen.len(),
        probes.len(),
        min_rms,
        db(min_rms),
        max_peak,
        db(max_peak),
        max_dc,
        min_master_rms,
        db(min_master_rms),
        max_master_peak,
        db(max_master_peak)
    );
}

#[test]
fn drum_kits_have_balanced_short_term_rms() {
    let probes = probes();
    let mut medians = Vec::with_capacity(samples::drum_kit_count());

    for kit in 0..samples::drum_kit_count() {
        let lanes: Vec<_> = probes.iter().filter(|p| p.kit == kit).collect();
        let mut rms_values: Vec<f32> = lanes.iter().map(|p| p.short_rms).collect();
        let kit_rms = median(&mut rms_values);
        medians.push(kit_rms);
        let representative = lanes
            .iter()
            .min_by(|a, b| {
                (a.short_rms - kit_rms)
                    .abs()
                    .total_cmp(&(b.short_rms - kit_rms).abs())
            })
            .expect("kit has no lanes");
        let max_peak = lanes.iter().map(|p| p.peak).fold(0.0f32, f32::max);
        println!(
            "KIT_LEVEL kit={:02} name={:<23} rep_lane={:02} rep_rms_db={:7.2} median_rms_db={:7.2} rep_peak_db={:7.2} max_peak_db={:7.2} trim_db={:+.1}",
            kit + 1,
            samples::KIT_NAMES[kit],
            representative.lane,
            db(representative.short_rms),
            db(kit_rms),
            db(representative.peak),
            db(max_peak),
            samples::drum_kit_trim_db(kit),
        );
    }

    let min = medians.iter().copied().fold(f32::MAX, f32::min);
    let max = medians.iter().copied().fold(0.0f32, f32::max);
    let spread_db = db(max) - db(min);
    let target = median(&mut medians.clone());
    println!(
        "KIT_SPREAD min_median_db={:.2} max_median_db={:.2} spread_db={:.2} target_db={:.2}",
        db(min),
        db(max),
        spread_db,
        db(target)
    );

    assert!(spread_db.is_finite(), "kit spread is not finite");
    assert!(
        spread_db <= 0.5,
        "kit median RMS spread is {:.2} dB (allowed 0.5 dB)",
        spread_db
    );
}
