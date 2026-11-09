//! Tagged melodic and rhythmic pattern bank.
//!
//! The data lives in `assets/patterns.tsv` so the browser, the tests and the
//! load action all read one real catalogue instead of hardcoded UI strings.

use crate::params::{Device, DeviceKind};
use crate::song::{Clip, Note, Song};
use std::sync::OnceLock;

/// TSV columns: name, genre, role, key, scale, bpm_min, bpm_max, bars,
/// instrument, gain_db, notes. Notes are `start:pitch:length:velocity`.
pub const PATTERN_DATA: &str = include_str!("../assets/patterns.tsv");

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Genre {
    Dubstep,
    House,
    Edm,
    Techno,
    Rap,
    MelodicBass,
    TrapHipHop,
    Ambient,
}

impl Genre {
    pub const ALL: [Genre; 8] = [
        Genre::Dubstep,
        Genre::House,
        Genre::Edm,
        Genre::Techno,
        Genre::Rap,
        Genre::MelodicBass,
        Genre::TrapHipHop,
        Genre::Ambient,
    ];

    pub fn tag(self) -> &'static str {
        match self {
            Genre::Dubstep => "dubstep",
            Genre::House => "house",
            Genre::Edm => "edm",
            Genre::Techno => "techno",
            Genre::Rap => "rap",
            Genre::MelodicBass => "melodic-bass",
            Genre::TrapHipHop => "trap-hiphop",
            Genre::Ambient => "ambient",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Genre::Dubstep => "DUBSTEP",
            Genre::House => "HOUSE",
            Genre::Edm => "EDM",
            Genre::Techno => "TECHNO",
            Genre::Rap => "RAP",
            Genre::MelodicBass => "MELODIC BASS",
            Genre::TrapHipHop => "TRAP",
            Genre::Ambient => "AMBIENT",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Genre::Dubstep => "DUBSTEP",
            Genre::House => "HOUSE",
            Genre::Edm => "EDM",
            Genre::Techno => "TECHNO",
            Genre::Rap => "RAP",
            Genre::MelodicBass => "MEL BASS",
            Genre::TrapHipHop => "TRAP",
            Genre::Ambient => "AMBIENT",
        }
    }

    pub fn from_tag(s: &str) -> Option<Self> {
        Genre::ALL.into_iter().find(|g| g.tag() == s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Bassline,
    Lead,
    ChordProgression,
    Arpeggio,
    Riff,
    Pluck,
    RapFlow,
}

impl Role {
    pub const ALL: [Role; 7] = [
        Role::Bassline,
        Role::Lead,
        Role::ChordProgression,
        Role::Arpeggio,
        Role::Riff,
        Role::Pluck,
        Role::RapFlow,
    ];

    pub fn tag(self) -> &'static str {
        match self {
            Role::Bassline => "bassline",
            Role::Lead => "lead",
            Role::ChordProgression => "chord",
            Role::Arpeggio => "arp",
            Role::Riff => "riff",
            Role::Pluck => "pluck",
            Role::RapFlow => "rap-flow",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Role::Bassline => "BASS",
            Role::Lead => "LEAD",
            Role::ChordProgression => "CHORDS",
            Role::Arpeggio => "ARP",
            Role::Riff => "RIFF",
            Role::Pluck => "PLUCK",
            Role::RapFlow => "FLOW",
        }
    }

    pub fn long_label(self) -> &'static str {
        match self {
            Role::Bassline => "Bassline",
            Role::Lead => "Lead",
            Role::ChordProgression => "Chord progression",
            Role::Arpeggio => "Arpeggio",
            Role::Riff => "Riff",
            Role::Pluck => "Pluck",
            Role::RapFlow => "Rap flow / topline",
        }
    }

    pub fn from_tag(s: &str) -> Option<Self> {
        Role::ALL.into_iter().find(|r| r.tag() == s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scale {
    Major,
    Minor,
    Dorian,
    Mixolydian,
}

impl Scale {
    pub const fn intervals(self) -> &'static [i32; 7] {
        match self {
            Scale::Major => &[0, 2, 4, 5, 7, 9, 11],
            Scale::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Scale::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Scale::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Scale::Major => "Major",
            Scale::Minor => "Minor",
            Scale::Dorian => "Dorian",
            Scale::Mixolydian => "Mixolydian",
        }
    }

    pub fn from_tag(s: &str) -> Option<Self> {
        match s {
            "major" => Some(Scale::Major),
            "minor" => Some(Scale::Minor),
            "dorian" => Some(Scale::Dorian),
            "mixolydian" => Some(Scale::Mixolydian),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub name: String,
    pub genre: Genre,
    pub role: Role,
    /// Pitch class of the key (0 = C).
    pub key: u8,
    pub scale: Scale,
    pub bpm_min: f64,
    pub bpm_max: f64,
    pub bars: f64,
    pub instrument: DeviceKind,
    pub gain_db: f32,
    pub notes: Vec<Note>,
}

impl Pattern {
    pub fn key_name(&self) -> &'static str {
        KEY_NAMES[(self.key as usize) % 12]
    }

    pub fn bpm(&self) -> f64 {
        ((self.bpm_min + self.bpm_max) * 0.5).round()
    }

    pub fn length_beats(&self) -> f64 {
        self.bars * 4.0
    }

    /// A fresh device at the instrument's first factory preset, so a preview
    /// and a loaded track use exactly the same timbre. The library's level trim
    /// is baked into the instrument's Level control, keeping previews and loaded
    /// tracks on one gain stage without pushing track faders above unity.
    pub fn preview_device(&self) -> Device {
        let mut d = Device::from_preset(self.instrument, &self.instrument.default_preset());
        if d.defs()
            .first()
            .map(|def| def.name == "Level")
            .unwrap_or(false)
        {
            let scale = 10f32.powf(self.gain_db / 20.0);
            d.set(0, d.p(0) * scale);
        }
        d
    }

    pub fn has_pitch_in_scale(&self, pitch: u8) -> bool {
        let interval = (pitch as i32 - self.key as i32).rem_euclid(12);
        self.scale.intervals().contains(&interval)
    }

    /// One-track song used by the browser preview path and the render tests.
    pub fn to_song(&self) -> Song {
        let mut song = Song::empty(&self.name, self.bpm());
        let idx = song.add_track(self.instrument);
        let t = &mut song.tracks[idx];
        t.instrument = self.preview_device();
        t.name = self.name.clone();
        t.volume = 0.8;
        let mut c = Clip::new(&self.name, 0, 0.0, self.length_beats());
        c.notes = self.notes.clone();
        t.clips.push(c);
        song.loop_enabled = false;
        song
    }
}

pub const KEY_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

fn key_from_name(s: &str) -> Option<u8> {
    KEY_NAMES.iter().position(|k| *k == s).map(|i| i as u8)
}

fn instrument_from_name(s: &str) -> Option<DeviceKind> {
    Some(match s {
        "DrumRack" => DeviceKind::DrumRack,
        "SubBass" => DeviceKind::SubBass,
        "ReeseBass" => DeviceKind::ReeseBass,
        "GrowlBass" => DeviceKind::GrowlBass,
        "Supersaw" => DeviceKind::Supersaw,
        "Pluck" => DeviceKind::Pluck,
        "Keys" => DeviceKind::Keys,
        "Organ" => DeviceKind::Organ,
        "Pad" => DeviceKind::Pad,
        "Riser" => DeviceKind::Riser,
        "Salamander" => DeviceKind::Salamander,
        "VoxBass" => DeviceKind::VoxBass,
        "VocalTake" => DeviceKind::VocalTake,
        _ => return None,
    })
}

/// Parse the shipped TSV. Public so tests can prove malformed data fails loudly.
pub fn parse_patterns(data: &str) -> Vec<Pattern> {
    let mut out = Vec::new();
    for (line_no, line) in data.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            cols.len(),
            11,
            "pattern row {} has {} columns, expected 11",
            line_no + 1,
            cols.len()
        );
        let name = cols[0].to_string();
        let genre = Genre::from_tag(cols[1])
            .unwrap_or_else(|| panic!("pattern row {}: unknown genre {:?}", line_no + 1, cols[1]));
        let role = Role::from_tag(cols[2])
            .unwrap_or_else(|| panic!("pattern row {}: unknown role {:?}", line_no + 1, cols[2]));
        let key = key_from_name(cols[3])
            .unwrap_or_else(|| panic!("pattern row {}: unknown key {:?}", line_no + 1, cols[3]));
        let scale = Scale::from_tag(cols[4])
            .unwrap_or_else(|| panic!("pattern row {}: unknown scale {:?}", line_no + 1, cols[4]));
        let parse_num = |i: usize| -> f64 {
            cols[i].parse::<f64>().unwrap_or_else(|_| {
                panic!("pattern row {}: {:?} is not a number", line_no + 1, cols[i])
            })
        };
        let bpm_min = parse_num(5);
        let bpm_max = parse_num(6);
        let bars = parse_num(7);
        let instrument = instrument_from_name(cols[8]).unwrap_or_else(|| {
            panic!(
                "pattern row {}: unknown instrument {:?}",
                line_no + 1,
                cols[8]
            )
        });
        let gain_db = cols[9].parse::<f32>().unwrap_or(0.0);
        let mut notes = Vec::new();
        for token in cols[10].split_whitespace() {
            let parts: Vec<&str> = token.split(':').collect();
            assert_eq!(
                parts.len(),
                4,
                "pattern row {}: bad note token {:?}",
                line_no + 1,
                token
            );
            let start = parts[0].parse::<f64>().unwrap_or_else(|_| {
                panic!("pattern row {}: bad note start {:?}", line_no + 1, token)
            });
            let pitch = parts[1].parse::<u8>().unwrap_or_else(|_| {
                panic!("pattern row {}: bad note pitch {:?}", line_no + 1, token)
            });
            let len = parts[2].parse::<f64>().unwrap_or_else(|_| {
                panic!("pattern row {}: bad note length {:?}", line_no + 1, token)
            });
            let vel = parts[3].parse::<f32>().unwrap_or_else(|_| {
                panic!("pattern row {}: bad note velocity {:?}", line_no + 1, token)
            });
            notes.push(Note::new(start, len, pitch, vel));
        }
        notes.sort_by(|a, b| {
            a.start
                .partial_cmp(&b.start)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.pitch.cmp(&b.pitch))
        });
        out.push(Pattern {
            name,
            genre,
            role,
            key,
            scale,
            bpm_min,
            bpm_max,
            bars,
            instrument,
            gain_db,
            notes,
        });
    }
    out
}

/// The process-wide catalogue. Parsed once; the data is embedded at compile
/// time, so this cannot fail at runtime unless the shipped asset is corrupt.
pub fn library() -> &'static [Pattern] {
    static LIB: OnceLock<Vec<Pattern>> = OnceLock::new();
    LIB.get_or_init(|| parse_patterns(PATTERN_DATA))
}

pub fn count_by_genre(patterns: &[Pattern]) -> [usize; 8] {
    let mut counts = [0; 8];
    for p in patterns {
        if let Some(i) = Genre::ALL.iter().position(|g| *g == p.genre) {
            counts[i] += 1;
        }
    }
    counts
}

pub fn count_by_role(patterns: &[Pattern]) -> [usize; 7] {
    let mut counts = [0; 7];
    for p in patterns {
        if let Some(i) = Role::ALL.iter().position(|r| *r == p.role) {
            counts[i] += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_meets_coverage_targets() {
        let lib = library();
        assert!(lib.len() >= 60, "only {} patterns", lib.len());

        let genres = count_by_genre(lib);
        for (g, count) in Genre::ALL.iter().zip(genres) {
            assert!(count >= 8, "{} has only {} patterns", g.label(), count);
        }

        let roles = count_by_role(lib);
        for (r, count) in Role::ALL.iter().zip(roles) {
            assert!(count > 0, "{} has no patterns", r.long_label());
        }
        // Rap and chord material must be substantive, not a token entry.
        assert!(roles[Role::RapFlow as usize] >= 4, "rap flow is too thin");
        assert!(
            roles[Role::ChordProgression as usize] >= 8,
            "chord progressions are too thin"
        );
        assert!(
            roles[Role::Bassline as usize] >= 8,
            "basslines are too thin"
        );
        assert!(roles[Role::Lead as usize] >= 8, "leads are too thin");
    }

    #[test]
    fn names_and_tags_are_unique_and_known() {
        let lib = library();
        let mut names = std::collections::HashSet::new();
        for p in lib {
            assert!(names.insert(p.name.clone()), "duplicate name {}", p.name);
            assert!(Genre::from_tag(p.genre.tag()).is_some());
            assert!(Role::from_tag(p.role.tag()).is_some());
            assert!(Scale::from_tag(match p.scale {
                Scale::Major => "major",
                Scale::Minor => "minor",
                Scale::Dorian => "dorian",
                Scale::Mixolydian => "mixolydian",
            })
            .is_some());
        }
    }

    #[test]
    fn every_pattern_has_playable_in_range_notes() {
        for p in library() {
            assert!(!p.notes.is_empty(), "{} is silent", p.name);
            assert!(
                p.bpm_min >= 60.0 && p.bpm_max <= 180.0 && p.bpm_min <= p.bpm_max,
                "{} has a bad tempo range {}..{}",
                p.name,
                p.bpm_min,
                p.bpm_max
            );
            assert!(p.bars >= 1.0, "{} has no bar length", p.name);
            assert!(
                p.gain_db.abs() <= 30.0,
                "{} has an extreme gain trim ({} dB)",
                p.name,
                p.gain_db
            );
            assert!(
                (0.0..=3.0).contains(&p.preview_device().p(0)),
                "{} normalization puts Level outside its range",
                p.name
            );
            let beats = p.length_beats();
            let mut peak_vel: f32 = 0.0;
            for n in &p.notes {
                assert!(
                    n.start >= -1e-9,
                    "{} has a negative note start {}",
                    p.name,
                    n.start
                );
                assert!(
                    n.start + n.len <= beats + 1e-6,
                    "{} note at {} + {} runs past the {} beat clip",
                    p.name,
                    n.start,
                    n.len,
                    beats
                );
                assert!(
                    n.len >= 0.05,
                    "{} has a {:.3} beat note (too short to play)",
                    p.name,
                    n.len
                );
                assert!(n.pitch <= 127, "{} has pitch {}", p.name, n.pitch);
                assert!(
                    (0.05..=1.0).contains(&n.vel),
                    "{} has velocity {}",
                    p.name,
                    n.vel
                );
                assert!(
                    p.has_pitch_in_scale(n.pitch),
                    "{}: pitch {} is outside {} {}",
                    p.name,
                    n.pitch,
                    p.key_name(),
                    p.scale.label()
                );
                peak_vel = peak_vel.max(n.vel);
            }
            assert!(
                peak_vel >= 0.5,
                "{} is effectively silent (peak velocity {})",
                p.name,
                peak_vel
            );
        }
    }

    #[test]
    fn patterns_use_pitched_instruments_not_only_chips() {
        for p in library() {
            assert!(
                p.instrument.is_instrument(),
                "{} uses non-instrument {:?}",
                p.name,
                p.instrument
            );
            assert!(
                p.instrument != DeviceKind::DrumRack,
                "{} is a drum pattern, expected pitched material",
                p.name
            );
        }
    }

    #[test]
    fn chord_and_arp_roles_have_real_voices() {
        for p in library() {
            if p.role == Role::ChordProgression {
                let starts: std::collections::HashSet<i64> = p
                    .notes
                    .iter()
                    .map(|n| (n.start * 1000.0).round() as i64)
                    .collect();
                assert!(
                    p.notes.len() >= starts.len() * 3,
                    "{} does not contain three-note chords",
                    p.name
                );
            }
            if p.role == Role::Arpeggio {
                let starts: std::collections::HashSet<i64> = p
                    .notes
                    .iter()
                    .map(|n| (n.start * 1000.0).round() as i64)
                    .collect();
                assert!(
                    p.notes.len() >= 16 && p.notes.len() == starts.len(),
                    "{} is not a single-line arpeggio",
                    p.name
                );
            }
        }
    }

    #[test]
    fn parser_rejects_a_bad_row_instead_of_shipping_silence() {
        let bad = "# name\tgenre\nBad\tdubstep\n";
        let result = std::panic::catch_unwind(|| parse_patterns(bad));
        assert!(result.is_err(), "malformed row was accepted");
    }
}
