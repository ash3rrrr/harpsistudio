# Harpsistudio delivery artifacts

This directory contains the end-to-end delivery proof for the shipped
Harpsistudio factory content. It is kept in the project tree, not in `target/`.

## What is here

| File | What it proves |
| --- | --- |
| `renders/dubstep-riddim-protocol.wav` | Riddim Protocol starter: half-time drums, moving growl bass, sub, dark pad, riser and Salamander piano hook |
| `renders/house-midnight-house.wav` | Midnight House starter: four-on-the-floor drums, offbeat sub, Am7 stabs, pad and supersaw lead |
| `renders/edm-big-room-anthem.wav` | Big Room Anthem starter: intro, drums-out build, bar-9 drop, supersaw chords, sub and pluck lead |
| `renders/rap-cypher-room.wav` | Cypher Room starter: boom-bap drums, felt piano, sample bass, sliding 808 and the dedicated Rap Flow |
| `renders/vocal-to-dubstep-bass.wav` | A supplied vocal imported as an `AudioTake` and processed through Vocal Take → Bassify → Wobble → Compressor → Eq3 |
| `renders/vocal-riddim.wav` | The 16-bar Vocal Riddim factory demo: chopped vocal hook, vocal growl, sub and synth growl over a half-time double drop |
| `renders/vocal-riddim-vocal-stem.wav` | The same arrangement with only the vocal hook, vocal growl and vocal harmony audible |
| `projects/Vocal Riddim.harp` | The loadable project for Vocal Riddim; the vocal take is embedded in the JSON |
| `reports/vocal-riddim.md` | Track roles, arrangement map and export paths for the vocal demo |
| `renders/drum-library-audition.wav` | One 8-beat pass through all 25 Drum Rack kits, all 16 lanes per kit, 400 mappings and all 155 embedded one-shots |
| `renders/salamander-midi-groove.wav` | House Club drums alongside a Salamander Grand track driven by Chord Generator → Arpeggiator |
| `reports/render-metrics.tsv` | Exact duration, peak, RMS, DC, clipping count and 20–180 Hz fraction for every render |
| `reports/delivery-report.md` | Genre evidence and the assertions that produced these files |
| `reports/drum-library.tsv` | Every kit/lane label, MIDI note and embedded sample index |
| `source/voice-note.wav` | The supplied vocal source used by the Bassify proof (the existing CC0 test fixture, copied here for traceability) |

## The real interface path

The proof deliberately does not use a separate renderer. The example calls the
same core functions that the GUI calls:

- `presets::demo_song(index)` is what the START cards and `Act::LoadDemo` load.
  The four genre cards are `DUBSTEP` (1), `HOUSE` (0), `EDM` (2) and `RAP` (7);
  **Vocal Riddim** is index 10 in **FACTORY DEMOS** and uses an embedded take.
- `project::save_song_to` writes the same `.harp` JSON as the UI's Save action;
  `project::export_wav_to` is the renderer and 16-bit/48 kHz WAV writer behind
  Mixdown. The vocal project is saved under `projects/` and reloaded before its
  mix is rendered.
- `project::export_wav_to` is the renderer and 16-bit/48 kHz WAV writer behind
  `project::export_wav`. The GUI's Mixdown action (`Act::Export`) calls
  `project::export_wav`.
- `AudioTake::from_wav_file` and `presets::add_vocal_bass_track` are the import
  and install path used by the Voice → Bass panel and drag-and-drop WAV import.
- Every render is the full arrangement plus the normal eight-beat Mixdown tail;
  the four genre starters are 16 bars each.

## Auditioning the Drum Rack in the GUI

Load any starter, open **DEVICE** on its Drum Rack, and click a factory preset
chip or set the **Kit** knob. The 25 kit chips wrap onto multiple rows. The
**PAD MAP** below the knobs lists all 16 lane labels and their MIDI notes. With
the Drum Rack track selected, use the computer keys `A W S E D F T G Y H U J K`
to audition; `Z`/`X` change octave. Between the C2, C3 and C4 octave positions
the map covers the full 16-note range (35–70), including the Sub 808, shaker and
high tom. The `drum-library-audition.wav` file is the offline equivalent: one
8-beat pass for every kit and lane.

## Measured result

| Render | Duration | Peak | DC | Clipped | Low-band fraction |
| --- | ---: | ---: | ---: | ---: | ---: |
| Dubstep | 30.86 s | −3.75 dBFS | +0.000216 | 0 | 0.498 |
| House | 34.84 s | −0.72 dBFS | +0.000680 | 0 | 0.482 |
| EDM | 33.75 s | −0.81 dBFS | −0.000001 | 0 | 0.343 |
| Rap | 48.00 s | −0.73 dBFS | +0.000524 | 0 | 0.471 |
| Vocal bass | 5.14 s | −5.80 dBFS | −0.000008 | 0 | 0.426 |
| Drum library | 104.00 s | −4.75 dBFS | +0.000001 | 0 | 0.426 |
| Salamander + MIDI | 20.00 s | −0.74 dBFS | −0.000048 | 0 | 0.484 |

Genre checks:

- Dubstep low-band fraction 0.498; the growl stem has 20 ms envelope
  modulation CV 0.666 from its 4 Hz wobble.
- House has 64 quarter-note kick notes; its drum-stem on-beat/offbeat low-band
  energy ratio is 1.25.
- EDM section RMS is 0.0385 (intro), 0.0138 (drums-out build), 0.0515 (drop)
  and 0.0516 (drop tail); the drop is 3.7× the build.
- Rap Flow is 0.348 of the full-mix RMS over its section; its 300–4000 Hz RMS is
  5.67× its 20–180 Hz RMS.
- The audition render covers all 25 kits × 16 lanes = 400 mappings (13
  recorded, 12 voiced), reaching 155 of 155 embedded source samples.
- Salamander + MIDI FX renders cleanly alongside a real House Club drum kit.

## Regenerate and verify

From `harpsistudio/`:

```bat
cargo run -p hs-core --example render_demo_artifacts
cargo run -p hs-core --example render_vocal_riddim
```

The example asserts the full render contract before writing the reports:
finite, non-silent audio; at least 16 bars for the four starters; peak below
full scale; zero raw PCM samples at i16 full scale; DC below 0.001; and the
genre-specific low-end, groove, build/drop, vocal-forward and sample-library
checks. A regression fails the command rather than silently replacing these
files with worse audio.

See `reports/delivery-report.md` for the generated evidence and
`reports/render-metrics.tsv` for the exact numbers.
