use hs_core::midi_export;
use hs_core::midi_import;
use hs_core::params::DeviceKind;
use hs_core::presets::{self, DEMOS};
use hs_core::song::Note;

fn sorted_notes(notes: &[Note]) -> Vec<Note> {
    let mut sorted = notes.to_vec();
    sorted.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.pitch.cmp(&b.pitch))
            .then_with(|| {
                a.len
                    .partial_cmp(&b.len)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    sorted
}

#[test]
fn vocal_riddim_survives_a_midi_round_trip() {
    let index = DEMOS
        .iter()
        .position(|demo| demo.name == "Vocal Riddim")
        .expect("Vocal Riddim factory demo");
    let original = presets::demo_song(index);
    let bytes = midi_export::to_bytes(&original).expect("MIDI export");
    let imported = midi_import::from_bytes(&bytes, "Vocal Riddim").expect("MIDI import");

    assert!((imported.bpm - original.bpm).abs() < 0.01);
    assert_eq!(imported.tracks.len(), original.tracks.len());

    let mut original_note_count = 0usize;
    let mut imported_note_count = 0usize;
    let mut clip_count = 0usize;
    for (original_track, imported_track) in original.tracks.iter().zip(&imported.tracks) {
        assert_eq!(imported_track.name, original_track.name);
        assert_eq!(
            imported_track.instrument.kind,
            if original_track.instrument.kind == DeviceKind::DrumRack {
                DeviceKind::DrumRack
            } else {
                DeviceKind::Keys
            }
        );
        let original_clips: Vec<_> = original_track.arrangement_clips().collect();
        let imported_clips: Vec<_> = imported_track.arrangement_clips().collect();
        assert_eq!(imported_clips.len(), original_clips.len());
        clip_count += imported_clips.len();

        for (original_clip, imported_clip) in original_clips.iter().zip(&imported_clips) {
            assert_eq!(imported_clip.name, original_clip.name);
            assert!((imported_clip.start - original_clip.start).abs() < 0.0025);
            let original_notes = sorted_notes(&original_clip.notes);
            let imported_notes = sorted_notes(&imported_clip.notes);
            assert_eq!(imported_notes.len(), original_notes.len());
            original_note_count += original_notes.len();
            imported_note_count += imported_notes.len();

            for (original_note, imported_note) in original_notes.iter().zip(&imported_notes) {
                assert!(
                    (imported_note.start - original_note.start).abs() < 0.0025,
                    "note start changed in {}: {} vs {}",
                    original_clip.name,
                    imported_note.start,
                    original_note.start
                );
                assert!(
                    (imported_note.len - original_note.len).abs() < 0.0025,
                    "note length changed in {}: {} vs {}",
                    original_clip.name,
                    imported_note.len,
                    original_note.len
                );
                assert_eq!(imported_note.pitch, original_note.pitch);
                assert!((imported_note.vel - original_note.vel).abs() < 0.01);
            }
        }
    }

    assert_eq!(clip_count, 15);
    assert_eq!(imported_note_count, original_note_count);
    assert_eq!(imported_note_count, 424);
}
