use hs_core::params::DeviceKind;
use hs_core::presets::{demo_song, DEMOS};
use hs_core::song::Song;

fn track_with_notes<'a>(song: &'a Song, needle: &str) -> Option<&'a hs_core::song::Track> {
    song.tracks.iter().find(|t| {
        t.name.to_lowercase().contains(&needle.to_lowercase())
            && t.clips.iter().any(|c| !c.notes.is_empty())
    })
}

fn max_chord_notes(song: &Song) -> usize {
    let mut best = 0;
    for t in &song.tracks {
        for c in &t.clips {
            let mut starts: Vec<i64> = c
                .notes
                .iter()
                .map(|n| (n.start * 1000.0).round() as i64)
                .collect();
            starts.sort_unstable();
            starts.dedup();
            for s in starts {
                let count = c
                    .notes
                    .iter()
                    .filter(|n| (n.start * 1000.0).round() as i64 == s)
                    .count();
                best = best.max(count);
            }
        }
    }
    best
}

#[test]
fn four_genre_starters_ship_bass_chords_and_a_lead() {
    let cases: [(usize, &str, &str, &str); 4] = [
        (0, "Sub Bass", "House Stabs", "Super Lead"),
        (1, "Growl Bass", "Dark Pad", "Piano Lead"),
        (2, "Sub Bass", "Supersaw Chords", "Pluck Lead"),
        (7, "Bass Sample", "Felt Piano", "Rap Flow"),
    ];
    for (idx, bass, chord, lead) in cases {
        let song = demo_song(idx);
        assert!(
            track_with_notes(&song, bass).is_some(),
            "{} starter is missing its bassline ({bass})",
            DEMOS[idx].name
        );
        assert!(
            track_with_notes(&song, chord).is_some(),
            "{} starter is missing its chord material ({chord})",
            DEMOS[idx].name
        );
        assert!(
            track_with_notes(&song, lead).is_some(),
            "{} starter is missing its melodic lead ({lead})",
            DEMOS[idx].name
        );
        assert!(
            max_chord_notes(&song) >= 3,
            "{} starter has no real chord voicing",
            DEMOS[idx].name
        );
        let pitched = song
            .tracks
            .iter()
            .filter(|t| {
                t.instrument.kind.is_instrument()
                    && t.instrument.kind != DeviceKind::DrumRack
                    && t.clips.iter().any(|c| !c.notes.is_empty())
            })
            .count();
        assert!(
            pitched >= 3,
            "{} starter has only {} pitched tracks",
            DEMOS[idx].name,
            pitched
        );
    }
}

#[test]
fn rap_starter_has_its_own_dedicated_flow_material() {
    let song = demo_song(7);
    let flow = track_with_notes(&song, "Rap Flow").expect("Rap Flow track");
    assert_eq!(flow.instrument.kind, DeviceKind::Keys);
    let notes: usize = flow.clips.iter().map(|c| c.notes.len()).sum();
    assert!(notes >= 24, "rap flow has only {notes} notes");
    // A flow needs rhythmically distinct entries, not one long pad note.
    let distinct_starts: std::collections::HashSet<i64> = flow
        .clips
        .iter()
        .flat_map(|c| c.notes.iter())
        .map(|n| (n.start * 1000.0).round() as i64)
        .collect();
    assert!(
        distinct_starts.len() >= 12,
        "rap flow is not rhythmically articulated"
    );
}

#[test]
fn all_demo_projects_still_load_with_expected_track_counts() {
    for (idx, demo) in DEMOS.iter().enumerate() {
        let song = demo_song(idx);
        assert_eq!(song.name, demo.name);
        assert!(!song.tracks.is_empty(), "{} has no tracks", demo.name);
        assert!(
            song.tracks.iter().any(|t| !t.clips.is_empty()),
            "{} has no playable clips",
            demo.name
        );
    }
}
