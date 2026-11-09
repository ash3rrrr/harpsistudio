# Changelog

All notable changes to Harpsistudio are documented here. The project follows
[Semantic Versioning](https://semver.org/) for the application version.

## [Unreleased]

### Added

- **Versioned `.harp` projects.** Saves now include a top-level
  `format_version: 1` field. Unversioned projects written by earlier builds
  still load unchanged, while a project from a newer schema is rejected with an
  explicit update message instead of being mis-parsed.
- **Bounded undo/redo** across the editor. Notes, instruments, effects and
  parameters, mixer moves, arrangement/track structure and tempo become one
  undo step per completed gesture. The history retains at most 64 snapshots,
  invalidates redo after a new edit, and republishes restored songs without
  stopping playback. `CTRL/CMD+Z`, `CTRL/CMD+SHIFT+Z`, `CTRL+Y` and the
  bottom-rail **UNDO**/**REDO** controls are available.
- Standard MIDI File export for arrangement clips from the desktop **FILES**
  tab and `harpsistudio-cli midi`. It preserves tempo, track names, clip
  markers, note timing and velocity. Session-only projects export their first
  session clip per track, matching the WAV renderer's fallback.
- Standard MIDI File import through **FILES → IMPORT MIDI** and
  `harpsistudio-cli import-midi`. It reads format 0/1 files, restores tempo,
  track names, clip markers, notes, velocities and channel 10 Drum Rack tracks,
  then saves the result as a regular `.harp` project. Malformed or truncated
  files fail with a clear error.
- Twelve new Drum Rack kits (25 total) voiced at load time from the embedded,
  already-licensed one-shots: Tape Crunch, Vinyl Dust, Sub Pressure, Glass
  Room, Warehouse, Dub Chamber, Pocket Drums, Neon Electro, Broken Beats, Deep
  909, Hard Techno and Cinematic Hits. No new audio files or sample licenses.
- Drum Rack presets and the Kit parameter now cover all 25 kits. The factory
  sound library grows from 1,152 to 1,440 sounds (60 families) because each
  Drum Rack preset contributes its own family.

### Changed

- Added undo/redo regression tests for ordering, redo invalidation, the
  snapshotted depth bound, gesture batching, both redo chords, mixed-edit round
  trips and live-engine playback. Normalized three MIDI importer test
  assertions so the strict Clippy gate passes.
- README now shows current screenshots of the Session, Arrangement and Mixer
  views and labels the build-from-source section explicitly. The checked-in
  captures are regenerated from the release build.
- CI now smoke-tests `list-demos`, `inspect`, MIDI export and MIDI import
  through the release `harpsistudio-cli` binary on Ubuntu.
- Added MIDI import round-trip coverage for the **Vocal Riddim** factory demo,
  plus hand-built fixtures for running status, channel 10 and malformed input.
- Reorganized the contributor guides under `.github/`, added bug, feature and
  pull-request templates, and expanded the README with verified build and CLI
  instructions plus a gap-based roadmap.
- Regenerated the checked-in delivery artifacts; the drum-library audition now
  covers all 25 kits (400 mappings) and the generated reports use the current
  counts and bar lengths.

## [0.1.0] - 2026-11-09

This is the first open-source release candidate. The source was previously
unlicensed; the code is now released under the MIT License. The embedded audio
is not relicensed: its Creative Commons and public-domain terms are recorded in
`NOTICE` and `crates/hs-core/assets/audio/ATTRIBUTION.md`.

### Added

- **Headless CLI** (`hs-cli`, binary `harpsistudio-cli`) for listing factory
  demos, inspecting `.harp` projects, rendering saved projects, and rendering
  demos through the same 48 kHz/16-bit WAV path as the GUI Mixdown action.
- **Open-source contributor documentation**: `CONTRIBUTING.md`,
  `CODE_OF_CONDUCT.md`, `SECURITY.md`, `NOTICE`, and a root `.gitignore`.
- **CI** for Windows and Linux on every push and pull request, covering
  formatting, whole-workspace Clippy with warnings denied, the full test suite,
  and release builds of both `hs-app` and `hs-cli`. `rust-toolchain.toml` pins
  the stable toolchain plus the `rustfmt` and `clippy` components the gates use.
- A clear Ableton trademark disclaimer; Harpsistudio is independent and does
  not include Ableton artwork or proprietary UI assets.

### Changed

- Relicensed the Rust source code from unlicensed to **MIT**.
- Preserved the original sample licenses and attribution instead of applying
  MIT to third-party audio. The MIT code boundary is documented in `NOTICE`.
- Made `hs-app` the workspace default member so `cargo run --release` and
  `run.bat` continue to launch the desktop GUI after the CLI binary was added.
- Removed the workspace's last Clippy suppression and the dead code it hid, so
  the strict `-D warnings` gate passes without `#[allow]` overrides.
- Updated the README with the open-source policy, headless CLI examples,
  contributor links, and the corrected factory-project count.

### Verification

- 88 workspace tests pass, including three CLI tests for argument parsing,
  demo resolution, and demo summaries.
- `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets --all-features -- -D warnings` are
  clean.
- Release builds for `hs-app` and `hs-cli` succeed.
- `cargo run --release` still launches the desktop GUI; the capture harness was
  used to verify startup without leaving a process running.
- The CLI renders a real MIDI/WAV project and reports a valid RIFF/WAVE result.
