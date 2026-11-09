//! Factory sound library: 1,000+ ready-to-play instrument presets.
//!
//! Every sound is generated from a real factory preset on a playable
//! instrument and shaped by a named character. The result is a complete,
//! in-range parameter set that can be loaded straight onto a track, so the
//! library is not a static list of names: every entry renders through the
//! same engine as a hand-made preset.

use crate::params::{Device, DeviceKind, Fmt, ParamDef};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SoundCategory {
    Drums,
    Bass,
    Lead,
    Pluck,
    Keys,
    Pad,
    Fx,
}

impl SoundCategory {
    pub const ALL: [SoundCategory; 7] = [
        SoundCategory::Drums,
        SoundCategory::Bass,
        SoundCategory::Lead,
        SoundCategory::Pluck,
        SoundCategory::Keys,
        SoundCategory::Pad,
        SoundCategory::Fx,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SoundCategory::Drums => "DRUMS",
            SoundCategory::Bass => "BASS",
            SoundCategory::Lead => "LEAD",
            SoundCategory::Pluck => "PLUCK",
            SoundCategory::Keys => "KEYS",
            SoundCategory::Pad => "PAD",
            SoundCategory::Fx => "FX",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            SoundCategory::Drums => "Drums",
            SoundCategory::Bass => "Bass",
            SoundCategory::Lead => "Lead",
            SoundCategory::Pluck => "Pluck",
            SoundCategory::Keys => "Keys",
            SoundCategory::Pad => "Pad",
            SoundCategory::Fx => "FX",
        }
    }

    pub fn color(self) -> [u8; 3] {
        match self {
            SoundCategory::Drums => [255, 122, 74],
            SoundCategory::Bass => [225, 66, 138],
            SoundCategory::Lead => [255, 190, 60],
            SoundCategory::Pluck => [90, 220, 170],
            SoundCategory::Keys => [120, 200, 255],
            SoundCategory::Pad => [150, 130, 255],
            SoundCategory::Fx => [255, 100, 200],
        }
    }
}

#[derive(Clone, Debug)]
pub struct SoundPreset {
    pub name: String,
    pub kind: DeviceKind,
    pub category: SoundCategory,
    /// Factory preset the sound was built from.
    pub family: &'static str,
    /// Named character applied to the family.
    pub character: &'static str,
    pub values: Vec<f32>,
}

impl SoundPreset {
    /// Build a fresh device carrying this sound.
    pub fn to_device(&self) -> Device {
        let mut d = Device::new(self.kind);
        d.params = self.values.clone();
        d.on = true;
        d
    }

    /// Load the sound into an existing device, replacing its instrument.
    pub fn load_into(&self, device: &mut Device) {
        device.kind = self.kind;
        device.params = self.values.clone();
        device.on = true;
    }
}

#[derive(Clone, Copy)]
struct Variation {
    label: &'static str,
    /// -1 dark, +1 bright.
    brightness: f32,
    /// -1 thin, +1 heavy low end.
    weight: f32,
    /// -1 soft/long attack, +1 snappy.
    attack: f32,
    /// -1 short, +1 long decay/release.
    tail: f32,
    /// -1 still, +1 fast modulation.
    motion: f32,
    /// -1 clean, +1 saturated.
    grit: f32,
    /// -1 narrow, +1 wide/detuned.
    width: f32,
    /// -1 low, +1 high tuning.
    tune: f32,
}

const VARIATION_COUNT: usize = 24;

const VARIATIONS: [Variation; VARIATION_COUNT] = [
    Variation {
        label: "Deep",
        brightness: -0.6,
        weight: 0.9,
        attack: -0.1,
        tail: 0.2,
        motion: 0.0,
        grit: 0.1,
        width: 0.0,
        tune: -0.5,
    },
    Variation {
        label: "Bright",
        brightness: 0.9,
        weight: -0.3,
        attack: 0.2,
        tail: -0.1,
        motion: 0.1,
        grit: 0.0,
        width: 0.1,
        tune: 0.0,
    },
    Variation {
        label: "Warm",
        brightness: -0.35,
        weight: 0.6,
        attack: -0.1,
        tail: 0.2,
        motion: 0.05,
        grit: 0.15,
        width: 0.1,
        tune: -0.1,
    },
    Variation {
        label: "Wide",
        brightness: 0.1,
        weight: 0.0,
        attack: 0.0,
        tail: 0.2,
        motion: 0.1,
        grit: 0.0,
        width: 0.95,
        tune: 0.0,
    },
    Variation {
        label: "Tight",
        brightness: 0.2,
        weight: -0.1,
        attack: 0.7,
        tail: -0.6,
        motion: 0.0,
        grit: 0.1,
        width: -0.1,
        tune: 0.0,
    },
    Variation {
        label: "Loose",
        brightness: -0.1,
        weight: 0.2,
        attack: -0.7,
        tail: 0.7,
        motion: 0.15,
        grit: 0.0,
        width: 0.2,
        tune: -0.1,
    },
    Variation {
        label: "Dark",
        brightness: -0.9,
        weight: 0.5,
        attack: -0.2,
        tail: 0.3,
        motion: 0.1,
        grit: 0.05,
        width: 0.0,
        tune: -0.4,
    },
    Variation {
        label: "Airy",
        brightness: 0.7,
        weight: -0.5,
        attack: -0.2,
        tail: 0.5,
        motion: 0.2,
        grit: -0.2,
        width: 0.5,
        tune: 0.2,
    },
    Variation {
        label: "Gritty",
        brightness: 0.3,
        weight: 0.1,
        attack: 0.3,
        tail: -0.2,
        motion: 0.0,
        grit: 0.95,
        width: 0.0,
        tune: 0.0,
    },
    Variation {
        label: "Clean",
        brightness: 0.2,
        weight: 0.0,
        attack: 0.0,
        tail: 0.0,
        motion: 0.0,
        grit: -0.9,
        width: 0.1,
        tune: 0.0,
    },
    Variation {
        label: "Soft",
        brightness: -0.3,
        weight: 0.2,
        attack: -0.6,
        tail: 0.4,
        motion: 0.0,
        grit: -0.5,
        width: 0.2,
        tune: 0.0,
    },
    Variation {
        label: "Hard",
        brightness: 0.4,
        weight: 0.0,
        attack: 0.9,
        tail: -0.4,
        motion: 0.0,
        grit: 0.5,
        width: 0.0,
        tune: 0.0,
    },
    Variation {
        label: "Vintage",
        brightness: -0.4,
        weight: 0.3,
        attack: -0.2,
        tail: 0.3,
        motion: 0.3,
        grit: 0.3,
        width: -0.3,
        tune: -0.2,
    },
    Variation {
        label: "Modern",
        brightness: 0.5,
        weight: 0.1,
        attack: 0.3,
        tail: 0.1,
        motion: 0.1,
        grit: 0.1,
        width: 0.4,
        tune: 0.0,
    },
    Variation {
        label: "Plucked",
        brightness: 0.2,
        weight: 0.0,
        attack: 0.95,
        tail: -0.5,
        motion: 0.0,
        grit: 0.1,
        width: 0.0,
        tune: 0.0,
    },
    Variation {
        label: "Sustained",
        brightness: -0.1,
        weight: 0.2,
        attack: -0.4,
        tail: 0.9,
        motion: 0.2,
        grit: 0.0,
        width: 0.2,
        tune: 0.0,
    },
    Variation {
        label: "Moving",
        brightness: 0.1,
        weight: 0.0,
        attack: 0.0,
        tail: 0.1,
        motion: 0.95,
        grit: 0.0,
        width: 0.3,
        tune: 0.0,
    },
    Variation {
        label: "Static",
        brightness: 0.0,
        weight: 0.0,
        attack: 0.1,
        tail: 0.0,
        motion: -0.8,
        grit: 0.0,
        width: 0.0,
        tune: 0.0,
    },
    Variation {
        label: "Detuned",
        brightness: 0.0,
        weight: 0.0,
        attack: 0.0,
        tail: 0.1,
        motion: 0.1,
        grit: 0.1,
        width: 0.8,
        tune: 0.6,
    },
    Variation {
        label: "Focused",
        brightness: 0.3,
        weight: -0.2,
        attack: 0.4,
        tail: -0.3,
        motion: -0.4,
        grit: 0.1,
        width: -0.5,
        tune: 0.0,
    },
    Variation {
        label: "Hollow",
        brightness: -0.2,
        weight: -0.6,
        attack: 0.0,
        tail: 0.2,
        motion: 0.1,
        grit: 0.0,
        width: 0.3,
        tune: 0.0,
    },
    Variation {
        label: "Lush",
        brightness: 0.2,
        weight: 0.2,
        attack: -0.3,
        tail: 0.6,
        motion: 0.2,
        grit: -0.2,
        width: 0.7,
        tune: 0.0,
    },
    Variation {
        label: "Punchy",
        brightness: 0.3,
        weight: 0.3,
        attack: 0.85,
        tail: -0.3,
        motion: 0.0,
        grit: 0.3,
        width: 0.0,
        tune: 0.0,
    },
    Variation {
        label: "Subtle",
        brightness: 0.05,
        weight: 0.05,
        attack: 0.05,
        tail: 0.05,
        motion: 0.05,
        grit: 0.05,
        width: 0.05,
        tune: 0.0,
    },
];

static LIBRARY: OnceLock<Vec<SoundPreset>> = OnceLock::new();

/// Every factory sound, grouped by instrument then by family preset.
pub fn library() -> &'static [SoundPreset] {
    LIBRARY.get_or_init(build).as_slice()
}

pub fn count() -> usize {
    library().len()
}

pub fn find(name: &str) -> Option<&'static SoundPreset> {
    library().iter().find(|s| s.name.eq_ignore_ascii_case(name))
}

/// Number of factory instrument presets the library is built from.
pub fn family_count() -> usize {
    library().len() / VARIATION_COUNT
}

fn build() -> Vec<SoundPreset> {
    let mut out = Vec::with_capacity(48 * VARIATION_COUNT);
    for kind in DeviceKind::ALL {
        // Vocal Take needs a recorded take, so it is not a playable factory
        // sound. Every other instrument renders on its own.
        if !kind.is_instrument() || kind == DeviceKind::VocalTake {
            continue;
        }
        let category = category_for(kind);
        let defs = kind.defs();
        for base in kind.presets() {
            for var in VARIATIONS.iter() {
                let mut values = Vec::with_capacity(defs.len());
                for (i, def) in defs.iter().enumerate() {
                    let base_value = base.values.get(i).copied().unwrap_or(def.def);
                    values.push(shape(def, base_value, var));
                }
                out.push(SoundPreset {
                    name: format!("{} · {}", base.name, var.label),
                    kind,
                    category,
                    family: base.name,
                    character: var.label,
                    values,
                });
            }
        }
    }
    out
}

fn category_for(kind: DeviceKind) -> SoundCategory {
    use DeviceKind::*;
    match kind {
        DrumRack => SoundCategory::Drums,
        SubBass | ReeseBass | GrowlBass | VoxBass => SoundCategory::Bass,
        Supersaw => SoundCategory::Lead,
        Pluck => SoundCategory::Pluck,
        Keys | Organ | Salamander => SoundCategory::Keys,
        Pad => SoundCategory::Pad,
        Riser => SoundCategory::Fx,
        // The caller filters these out; keep a sensible fallback.
        _ => SoundCategory::Fx,
    }
}

/// Apply a character to one parameter. Enumerations and integer switches keep
/// their factory value so a generated sound can never land between modes.
fn shape(def: &ParamDef, base: f32, v: &Variation) -> f32 {
    if matches!(def.fmt, Fmt::Enum(_) | Fmt::Int(_)) {
        return base.clamp(def.min, def.max);
    }
    let n = if def.max - def.min > 1e-9 {
        ((base - def.min) / (def.max - def.min)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let out = match def.name {
        "Level" => base * (1.0 + v.weight * 0.12 + v.brightness * 0.04),
        "Cutoff" => base * 2.0f32.powf(v.brightness * 1.35 + v.weight * 0.25),
        "Reso" => n + v.brightness * 0.14 + v.grit * 0.08 - v.weight * 0.05,
        "Tone" => n + v.brightness * 0.18 + v.weight * 0.08,
        "Attack" => base * 2.0f32.powf(-v.attack * 2.0),
        "Decay" => base * 2.0f32.powf(v.tail * 0.85 - v.attack * 0.35),
        "Release" => base * 2.0f32.powf(v.tail * 1.1),
        "Sustain" => n + v.weight * 0.12 + v.tail * 0.08,
        "Detune" => base * (1.0 + v.width * 0.7 + v.tune * 0.25),
        "Width" => n + v.width * 0.28,
        "Drive" | "Growl" | "FM Amt" | "Snap" | "Click" => n + v.grit * 0.2 + v.weight * 0.04,
        "Punch" => n + v.attack * 0.18 + v.grit * 0.1,
        "Noise" => n + v.grit * 0.15 + v.brightness * 0.08,
        "Wobble" | "LFO Rate" => base * 2.0f32.powf(v.motion * 1.15),
        "LFO Amt" | "Movement" | "Rotary" | "Depth" => n + v.motion * 0.28,
        "Drawbars" => n + v.brightness * 0.15 + v.weight * 0.1,
        "Rise" => n + v.motion * 0.16 + v.brightness * 0.1,
        "Length" => base * 2.0f32.powf(v.tail * 0.5),
        "Ratio" => base * (1.0 + v.brightness * 0.15 + v.grit * 0.1),
        "Tune" => base + v.tune * 2.0,
        _ => base,
    };
    if out.is_finite() {
        out.clamp(def.min, def.max)
    } else {
        def.def
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn library_has_over_a_thousand_sounds() {
        assert!(count() >= 1000, "only {} sounds", count());
        assert_eq!(count(), family_count() * VARIATION_COUNT);
        assert_eq!(family_count(), 48);
        assert_eq!(VARIATION_COUNT, 24);
    }

    #[test]
    fn names_are_unique() {
        let mut names = HashSet::new();
        for sound in library() {
            assert!(
                names.insert(sound.name.as_str()),
                "duplicate sound {}",
                sound.name
            );
        }
    }

    #[test]
    fn every_sound_is_a_valid_device_state() {
        for sound in library() {
            let defs = sound.kind.defs();
            assert_eq!(sound.values.len(), defs.len(), "{}", sound.name);
            assert!(sound.kind.is_instrument());
            assert_ne!(sound.kind, DeviceKind::VocalTake);
            for (i, (value, def)) in sound.values.iter().zip(defs).enumerate() {
                assert!(value.is_finite(), "{} param {i}", sound.name);
                assert!(
                    *value >= def.min - 1e-6 && *value <= def.max + 1e-6,
                    "{} param {} = {} outside {}..{}",
                    sound.name,
                    def.name,
                    value,
                    def.min,
                    def.max
                );
                if matches!(def.fmt, Fmt::Enum(_) | Fmt::Int(_)) {
                    assert_eq!(*value, value.round(), "{} param {}", sound.name, def.name);
                }
            }
            let device = sound.to_device();
            assert_eq!(device.kind, sound.kind);
            assert!(device.on);
            assert!(!device.label().is_empty());
        }
    }

    #[test]
    fn every_playable_instrument_contributes_sounds() {
        for kind in DeviceKind::ALL {
            if !kind.is_instrument() || kind == DeviceKind::VocalTake {
                continue;
            }
            let n = library().iter().filter(|s| s.kind == kind).count();
            assert_eq!(n, kind.presets().len() * VARIATION_COUNT, "{kind:?}");
        }
        for cat in SoundCategory::ALL {
            assert!(library().iter().any(|s| s.category == cat), "{cat:?}");
        }
    }

    #[test]
    fn sound_settings_are_unique_across_the_whole_library() {
        let mut settings = HashSet::new();
        for sound in library() {
            let key = (
                crate::params::device_index(sound.kind),
                sound.values.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
            );
            assert!(
                settings.insert(key),
                "duplicate sound settings hidden behind {}",
                sound.name
            );
        }
        assert_eq!(settings.len(), count());
    }

    #[test]
    fn every_family_has_distinct_characters() {
        let mut families: std::collections::HashMap<(usize, &str), HashSet<Vec<u32>>> =
            std::collections::HashMap::new();
        for sound in library() {
            families
                .entry((crate::params::device_index(sound.kind), sound.family))
                .or_default()
                .insert(sound.values.iter().map(|v| v.to_bits()).collect());
        }
        assert_eq!(families.len(), 48);
        for (key, variants) in families {
            assert_eq!(variants.len(), VARIATION_COUNT, "{key:?} collided");
        }
    }
}
