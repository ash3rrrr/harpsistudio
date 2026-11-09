//! Factory plugin racks: 100 named effect chains that insert into a track.
//!
//! A rack is a real chain of the audio effects Harpsistudio already ships,
//! with every device set to a musical starting point. Inserting one puts all
//! of its devices into the track's effect rack, where they can be tweaked,
//! reordered or removed like hand-built effects.

use crate::params::{Device, DeviceKind};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PluginCategory {
    Dynamics,
    Drums,
    Vocal,
    Bass,
    Saturation,
    Space,
    Modulation,
    Filter,
    Creative,
    Mastering,
}

impl PluginCategory {
    pub const ALL: [PluginCategory; 10] = [
        PluginCategory::Dynamics,
        PluginCategory::Drums,
        PluginCategory::Vocal,
        PluginCategory::Bass,
        PluginCategory::Saturation,
        PluginCategory::Space,
        PluginCategory::Modulation,
        PluginCategory::Filter,
        PluginCategory::Creative,
        PluginCategory::Mastering,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PluginCategory::Dynamics => "DYNAMICS",
            PluginCategory::Drums => "DRUMS",
            PluginCategory::Vocal => "VOCAL",
            PluginCategory::Bass => "BASS",
            PluginCategory::Saturation => "SATURATION",
            PluginCategory::Space => "SPACE",
            PluginCategory::Modulation => "MODULATION",
            PluginCategory::Filter => "FILTER",
            PluginCategory::Creative => "CREATIVE",
            PluginCategory::Mastering => "MASTERING",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            PluginCategory::Dynamics => "Dynamics",
            PluginCategory::Drums => "Drums",
            PluginCategory::Vocal => "Vocal",
            PluginCategory::Bass => "Bass",
            PluginCategory::Saturation => "Saturation",
            PluginCategory::Space => "Space",
            PluginCategory::Modulation => "Modulation",
            PluginCategory::Filter => "Filter",
            PluginCategory::Creative => "Creative",
            PluginCategory::Mastering => "Mastering",
        }
    }

    pub fn color(self) -> [u8; 3] {
        match self {
            PluginCategory::Dynamics => [200, 200, 120],
            PluginCategory::Drums => [255, 122, 74],
            PluginCategory::Vocal => [255, 166, 120],
            PluginCategory::Bass => [225, 66, 138],
            PluginCategory::Saturation => [206, 166, 112],
            PluginCategory::Space => [110, 170, 255],
            PluginCategory::Modulation => [120, 220, 190],
            PluginCategory::Filter => [90, 200, 230],
            PluginCategory::Creative => [210, 110, 255],
            PluginCategory::Mastering => [150, 220, 120],
        }
    }
}

#[derive(Clone, Debug)]
pub struct PluginRack {
    pub name: String,
    pub category: PluginCategory,
    pub description: String,
    /// Complete effect chain in processing order.
    pub devices: Vec<Device>,
}

impl PluginRack {
    /// Clone the chain for insertion into a track.
    pub fn chain(&self) -> Vec<Device> {
        self.devices.clone()
    }

    /// Append the whole rack to an effect list. Returns the index of its first
    /// device so the UI can select it.
    pub fn insert_into(&self, effects: &mut Vec<Device>) -> usize {
        let start = effects.len();
        effects.extend(self.devices.iter().cloned());
        start
    }

    pub fn summary(&self) -> String {
        self.devices
            .iter()
            .map(|d| d.kind.name())
            .collect::<Vec<_>>()
            .join(" → ")
    }
}

struct Family {
    name: &'static str,
    category: PluginCategory,
    blurb: &'static str,
    build: fn() -> Vec<Device>,
}

struct Variant {
    label: &'static str,
    blurb: &'static str,
    mix: f32,
    drive: f32,
    threshold: f32,
    ratio: f32,
    width: f32,
    gain_db: f32,
    tone: f32,
    damp: f32,
    bits: f32,
    wow: f32,
    feedback: f32,
    makeup: f32,
}

const VARIANTS: [Variant; 5] = [
    Variant {
        label: "Subtle",
        blurb: "Gentle settings for stacking several racks.",
        mix: 0.65,
        drive: -0.12,
        threshold: 3.0,
        ratio: 0.9,
        width: 0.0,
        gain_db: 0.0,
        tone: 0.0,
        damp: 0.0,
        bits: 0.0,
        wow: 0.0,
        feedback: -0.05,
        makeup: -0.5,
    },
    Variant {
        label: "Classic",
        blurb: "The balanced factory starting point.",
        mix: 1.0,
        drive: 0.0,
        threshold: 0.0,
        ratio: 1.0,
        width: 0.0,
        gain_db: 0.0,
        tone: 0.0,
        damp: 0.0,
        bits: 0.0,
        wow: 0.0,
        feedback: 0.0,
        makeup: 0.0,
    },
    Variant {
        label: "Heavy",
        blurb: "More drive, firmer compression and a hotter output.",
        mix: 1.15,
        drive: 0.22,
        threshold: -5.0,
        ratio: 1.6,
        width: 0.0,
        gain_db: -1.0,
        tone: 0.08,
        damp: -0.1,
        bits: -3.0,
        wow: 0.0,
        feedback: 0.08,
        makeup: 2.0,
    },
    Variant {
        label: "Wide",
        blurb: "Opens the stereo image and eases the compression.",
        mix: 1.0,
        drive: 0.0,
        threshold: 0.0,
        ratio: 1.0,
        width: 0.35,
        gain_db: -0.5,
        tone: 0.02,
        damp: 0.05,
        bits: 0.0,
        wow: 0.0,
        feedback: 0.05,
        makeup: 0.0,
    },
    Variant {
        label: "Vintage",
        blurb: "Darker tone, softer transients and tape-style movement.",
        mix: 0.95,
        drive: 0.12,
        threshold: 1.5,
        ratio: 0.95,
        width: -0.1,
        gain_db: -0.75,
        tone: -0.18,
        damp: 0.2,
        bits: -2.0,
        wow: 0.18,
        feedback: 0.03,
        makeup: 0.5,
    },
];

const FAMILIES: [Family; 20] = [
    Family {
        name: "Bus Glue",
        category: PluginCategory::Dynamics,
        blurb: "Compressor, EQ and tape that make a drum bus or mix sit together.",
        build: bus_glue,
    },
    Family {
        name: "Drum Punch",
        category: PluginCategory::Drums,
        blurb: "Transient shaping and fast compression for upfront drums.",
        build: drum_punch,
    },
    Family {
        name: "Drum Room",
        category: PluginCategory::Drums,
        blurb: "Adds a short room and tone shaping around a drum bus.",
        build: drum_room,
    },
    Family {
        name: "Vocal Focus",
        category: PluginCategory::Vocal,
        blurb: "Presence EQ, controlled dynamics and warm tape for lead vocals.",
        build: vocal_focus,
    },
    Family {
        name: "Vocal Air",
        category: PluginCategory::Vocal,
        blurb: "Top-end lift, width and a short slap echo for vocal hooks.",
        build: vocal_air,
    },
    Family {
        name: "Bass Weight",
        category: PluginCategory::Bass,
        blurb: "Low-end EQ, punch compression and warm drive for bass lines.",
        build: bass_weight,
    },
    Family {
        name: "Sub Control",
        category: PluginCategory::Bass,
        blurb: "Tames highs and tightens the very bottom of a sub bass.",
        build: sub_control,
    },
    Family {
        name: "Growl Stack",
        category: PluginCategory::Bass,
        blurb: "Vocal formant bass, wobble, drive and multiband glue.",
        build: growl_stack,
    },
    Family {
        name: "Riddim Wub",
        category: PluginCategory::Bass,
        blurb: "Fast wobble, bitcrush and a resonant sweep for riddim bass.",
        build: riddim_wub,
    },
    Family {
        name: "Trance Gate",
        category: PluginCategory::Modulation,
        blurb: "Rhythmic panning, ping-pong delay and a club hall.",
        build: trance_gate,
    },
    Family {
        name: "Space Echo",
        category: PluginCategory::Space,
        blurb: "Dub echo, filtered repeats and a long ambient tail.",
        build: space_echo,
    },
    Family {
        name: "Hall Bloom",
        category: PluginCategory::Space,
        blurb: "A large hall with gentle EQ for pads and atmospheres.",
        build: hall_bloom,
    },
    Family {
        name: "Tape Warmth",
        category: PluginCategory::Saturation,
        blurb: "Tape saturation and a musical EQ curve for any source.",
        build: tape_warmth,
    },
    Family {
        name: "Lo-Fi Dust",
        category: PluginCategory::Creative,
        blurb: "Bitcrush, band-limited filtering and dusty tape wobble.",
        build: lofi_dust,
    },
    Family {
        name: "Filter Motion",
        category: PluginCategory::Filter,
        blurb: "A moving filter, phaser and auto pan for animated textures.",
        build: filter_motion,
    },
    Family {
        name: "Stereo Bloom",
        category: PluginCategory::Modulation,
        blurb: "Chorus width, stereo utility and a final tonal polish.",
        build: stereo_bloom,
    },
    Family {
        name: "Pump Machine",
        category: PluginCategory::Dynamics,
        blurb: "Rhythmic ducking plus glue compression for four-on-the-floor.",
        build: pump_machine,
    },
    Family {
        name: "Master Polish",
        category: PluginCategory::Mastering,
        blurb: "Multiband glue, compression, EQ and stereo control for a master.",
        build: master_polish,
    },
    Family {
        name: "Transient Sculpt",
        category: PluginCategory::Drums,
        blurb: "Softens or sharpens transients before punchy compression.",
        build: transient_sculpt,
    },
    Family {
        name: "Formant Dream",
        category: PluginCategory::Creative,
        blurb: "Talking formant motion, slow phasing and a wide hall.",
        build: formant_dream,
    },
];

static LIBRARY: OnceLock<Vec<PluginRack>> = OnceLock::new();

pub fn library() -> &'static [PluginRack] {
    LIBRARY.get_or_init(build).as_slice()
}

pub fn count() -> usize {
    library().len()
}

pub fn find(name: &str) -> Option<&'static PluginRack> {
    library().iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

fn build() -> Vec<PluginRack> {
    let mut out = Vec::with_capacity(FAMILIES.len() * VARIANTS.len());
    for family in &FAMILIES {
        let base = (family.build)();
        for variant in &VARIANTS {
            let mut devices = base.clone();
            apply_variant(&mut devices, variant);
            out.push(PluginRack {
                name: format!("{} · {}", family.name, variant.label),
                category: family.category,
                description: format!("{} {}", family.blurb, variant.blurb),
                devices,
            });
        }
    }
    out
}

fn fx(kind: DeviceKind, preset: &str) -> Device {
    let found = kind
        .presets()
        .iter()
        .find(|p| p.name == preset)
        .copied()
        .unwrap_or_else(|| kind.default_preset());
    Device::from_preset(kind, &found)
}

fn set(device: &mut Device, param: &str, value: f32) {
    if let Some(i) = device.defs().iter().position(|d| d.name == param) {
        device.set(i, value);
    }
}

fn utility() -> Device {
    fx(DeviceKind::Utility, "Reset")
}

fn bus_glue() -> Vec<Device> {
    let mut eq = fx(DeviceKind::Eq3, "Smile");
    set(&mut eq, "Low", 1.5);
    set(&mut eq, "High", 1.0);
    vec![
        fx(DeviceKind::Compressor, "Glue"),
        eq,
        fx(DeviceKind::Tape, "Master Glue"),
        utility(),
    ]
}

fn drum_punch() -> Vec<Device> {
    vec![
        fx(DeviceKind::Transient, "Punch"),
        fx(DeviceKind::Compressor, "Punch"),
        fx(DeviceKind::Eq3, "Smile"),
        utility(),
    ]
}

fn drum_room() -> Vec<Device> {
    let mut reverb = fx(DeviceKind::Reverb, "Tight Room");
    set(&mut reverb, "Mix", 0.18);
    vec![
        fx(DeviceKind::Transient, "Punch"),
        reverb,
        fx(DeviceKind::Eq3, "Smile"),
        utility(),
    ]
}

fn vocal_focus() -> Vec<Device> {
    let mut eq = fx(DeviceKind::Eq3, "Smile");
    set(&mut eq, "Low", -2.0);
    set(&mut eq, "Mid", 2.5);
    set(&mut eq, "High", 2.0);
    set(&mut eq, "Freq", 2600.0);
    let mut comp = fx(DeviceKind::Compressor, "Glue");
    set(&mut comp, "Threshold", -24.0);
    set(&mut comp, "Ratio", 3.5);
    set(&mut comp, "Attack", 0.006);
    set(&mut comp, "Release", 0.14);
    set(&mut comp, "Makeup", 4.0);
    let mut tape = fx(DeviceKind::Tape, "Warm Tape");
    set(&mut tape, "Mix", 0.45);
    vec![eq, comp, tape, utility()]
}

fn vocal_air() -> Vec<Device> {
    let mut eq = fx(DeviceKind::Eq3, "Smile");
    set(&mut eq, "High", 4.0);
    set(&mut eq, "Low", -1.5);
    let mut chorus = fx(DeviceKind::Chorus, "Width");
    set(&mut chorus, "Mix", 0.25);
    let mut delay = fx(DeviceKind::Delay, "Slap");
    set(&mut delay, "Mix", 0.18);
    let mut reverb = fx(DeviceKind::Reverb, "Tight Room");
    set(&mut reverb, "Mix", 0.22);
    vec![eq, chorus, delay, reverb, utility()]
}

fn bass_weight() -> Vec<Device> {
    let mut eq = fx(DeviceKind::Eq3, "Smile");
    set(&mut eq, "Low", 5.0);
    set(&mut eq, "Mid", -1.5);
    set(&mut eq, "High", -3.0);
    set(&mut eq, "Freq", 320.0);
    vec![
        eq,
        fx(DeviceKind::Compressor, "Punch"),
        fx(DeviceKind::Drive, "Warm"),
        utility(),
    ]
}

fn sub_control() -> Vec<Device> {
    let mut filter = fx(DeviceKind::Filter, "Init");
    set(&mut filter, "Cutoff", 120.0);
    set(&mut filter, "Reso", 0.1);
    set(&mut filter, "Drive", 0.05);
    let mut comp = fx(DeviceKind::Compressor, "Glue");
    set(&mut comp, "Threshold", -20.0);
    set(&mut comp, "Ratio", 5.0);
    let mut out = utility();
    set(&mut out, "Gain", 1.0);
    vec![filter, comp, out]
}

fn growl_stack() -> Vec<Device> {
    vec![
        fx(DeviceKind::Bassify, "Vocal Wub"),
        fx(DeviceKind::Wobble, "Classic Wub"),
        fx(DeviceKind::Drive, "Rage"),
        fx(DeviceKind::Multiband, "Glue"),
        utility(),
    ]
}

fn riddim_wub() -> Vec<Device> {
    vec![
        fx(DeviceKind::Wobble, "Riddim Chop"),
        fx(DeviceKind::Bitcrush, "Lo-Fi"),
        fx(DeviceKind::Filter, "Dubstep Sweep"),
        fx(DeviceKind::Drive, "Fold"),
        utility(),
    ]
}

fn trance_gate() -> Vec<Device> {
    let mut reverb = fx(DeviceKind::Reverb, "Club Hall");
    set(&mut reverb, "Mix", 0.22);
    vec![
        fx(DeviceKind::AutoPan, "Trance Gate"),
        fx(DeviceKind::Delay, "Ping Pong"),
        reverb,
        utility(),
    ]
}

fn space_echo() -> Vec<Device> {
    let mut filter = fx(DeviceKind::Filter, "Dubstep Sweep");
    set(&mut filter, "Cutoff", 900.0);
    let mut reverb = fx(DeviceKind::Reverb, "Infinite");
    set(&mut reverb, "Mix", 0.35);
    vec![fx(DeviceKind::Delay, "Dub Echo"), filter, reverb, utility()]
}

fn hall_bloom() -> Vec<Device> {
    let mut reverb = fx(DeviceKind::Reverb, "Club Hall");
    set(&mut reverb, "Size", 0.85);
    set(&mut reverb, "Mix", 0.4);
    let mut eq = fx(DeviceKind::Eq3, "Smile");
    set(&mut eq, "Low", -1.0);
    set(&mut eq, "High", 2.0);
    vec![reverb, eq, utility()]
}

fn tape_warmth() -> Vec<Device> {
    let mut out = utility();
    set(&mut out, "Gain", -1.0);
    vec![
        fx(DeviceKind::Tape, "Warm Tape"),
        fx(DeviceKind::Eq3, "Smile"),
        out,
    ]
}

fn lofi_dust() -> Vec<Device> {
    let mut out = utility();
    set(&mut out, "Gain", -2.0);
    vec![
        fx(DeviceKind::Bitcrush, "Lo-Fi"),
        fx(DeviceKind::Filter, "Telephone"),
        fx(DeviceKind::Tape, "Dusty Loop"),
        out,
    ]
}

fn filter_motion() -> Vec<Device> {
    vec![
        fx(DeviceKind::Filter, "Dubstep Sweep"),
        fx(DeviceKind::Phaser, "Jet"),
        fx(DeviceKind::AutoPan, "Slow Drift"),
        utility(),
    ]
}

fn stereo_bloom() -> Vec<Device> {
    let mut chorus = fx(DeviceKind::Chorus, "Juno");
    set(&mut chorus, "Mix", 0.45);
    vec![
        chorus,
        fx(DeviceKind::Utility, "Wide"),
        fx(DeviceKind::Eq3, "Smile"),
    ]
}

fn pump_machine() -> Vec<Device> {
    vec![
        fx(DeviceKind::Sidechain, "House Pump"),
        fx(DeviceKind::Compressor, "Glue"),
        utility(),
    ]
}

fn master_polish() -> Vec<Device> {
    vec![
        fx(DeviceKind::Multiband, "Glue"),
        fx(DeviceKind::Compressor, "Glue"),
        fx(DeviceKind::Eq3, "Smile"),
        fx(DeviceKind::Utility, "Wide"),
    ]
}

fn transient_sculpt() -> Vec<Device> {
    vec![
        fx(DeviceKind::Transient, "Soft Touch"),
        fx(DeviceKind::Compressor, "Punch"),
        fx(DeviceKind::Eq3, "Smile"),
        utility(),
    ]
}

fn formant_dream() -> Vec<Device> {
    vec![
        fx(DeviceKind::Formant, "Talking Bass"),
        fx(DeviceKind::Phaser, "Slow Sweep"),
        fx(DeviceKind::Reverb, "Club Hall"),
        utility(),
    ]
}

fn apply_variant(devices: &mut [Device], v: &Variant) {
    for device in devices.iter_mut() {
        for i in 0..device.defs().len() {
            let def = device.defs()[i];
            let current = device.p(i);
            let next = match def.name {
                "Mix" => current * v.mix,
                "Drive" => current + v.drive,
                "Threshold" => current + v.threshold,
                "Ratio" => current * v.ratio,
                "Width" => current + v.width,
                "Size" => current + v.width * 0.5,
                "Depth" => current + v.width * 0.6,
                "Damp" => current + v.damp,
                "Tone" => current + v.tone,
                "Cutoff" => current * 2.0f32.powf(v.tone * 2.0),
                "Gain" => current + v.gain_db,
                "Output" => current * 2.0f32.powf(v.gain_db / 6.0),
                "Bits" => current + v.bits,
                "Wow" => current + v.wow,
                "Feedback" => current + v.feedback,
                "Makeup" => current + v.makeup,
                _ => current,
            };
            if (next - current).abs() > 1e-6 {
                device.set(i, next);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_has_one_hundred_plugins() {
        assert_eq!(count(), 100);
        assert_eq!(FAMILIES.len() * VARIANTS.len(), 100);
        assert_eq!(
            library()
                .iter()
                .filter(|p| p.name.contains(" · Classic"))
                .count(),
            FAMILIES.len()
        );
    }

    #[test]
    fn names_are_unique() {
        let mut names = HashSet::new();
        for rack in library() {
            assert!(names.insert(rack.name.as_str()), "duplicate {}", rack.name);
        }
    }

    #[test]
    fn every_rack_is_a_valid_audio_effect_chain() {
        for rack in library() {
            assert!(!rack.devices.is_empty(), "{}", rack.name);
            assert!(!rack.description.is_empty(), "{}", rack.name);
            for device in &rack.devices {
                assert!(device.kind.is_audio_effect(), "{}", rack.name);
                let defs = device.defs();
                assert_eq!(device.params.len(), defs.len(), "{}", rack.name);
                for (value, def) in device.params.iter().zip(defs) {
                    assert!(value.is_finite(), "{} {}", rack.name, def.name);
                    assert!(
                        *value >= def.min - 1e-6 && *value <= def.max + 1e-6,
                        "{} {} = {} outside {}..{}",
                        rack.name,
                        def.name,
                        value,
                        def.min,
                        def.max
                    );
                }
            }
        }
    }

    #[test]
    fn plugin_chains_are_unique_across_the_whole_catalog() {
        let mut chains = HashSet::new();
        for rack in library() {
            let key: Vec<(usize, Vec<u32>, bool)> = rack
                .devices
                .iter()
                .map(|d| {
                    (
                        crate::params::device_index(d.kind),
                        d.params.iter().map(|v| v.to_bits()).collect(),
                        d.on,
                    )
                })
                .collect();
            assert!(
                chains.insert(key),
                "duplicate plugin chain hidden behind {}",
                rack.name
            );
        }
        assert_eq!(chains.len(), count());
    }

    #[test]
    fn variants_are_distinct_and_insertable() {
        for family in &FAMILIES {
            let start = library()
                .iter()
                .position(|r| r.name.starts_with(&format!("{} · ", family.name)))
                .expect("family in library");
            let racks = &library()[start..start + VARIANTS.len()];
            let chains: HashSet<Vec<(usize, Vec<u32>)>> = racks
                .iter()
                .map(|r| {
                    r.devices
                        .iter()
                        .map(|d| {
                            (
                                crate::params::device_index(d.kind),
                                d.params.iter().map(|v| v.to_bits()).collect(),
                            )
                        })
                        .collect()
                })
                .collect();
            assert_eq!(
                chains.len(),
                VARIANTS.len(),
                "{} variants collide",
                family.name
            );

            let mut effects = Vec::new();
            let first = racks[0].insert_into(&mut effects);
            assert_eq!(first, 0);
            assert_eq!(effects.len(), racks[0].devices.len());
        }
    }

    #[test]
    fn categories_are_populated() {
        for cat in PluginCategory::ALL {
            assert!(library().iter().any(|r| r.category == cat), "{cat:?}");
        }
    }
}
