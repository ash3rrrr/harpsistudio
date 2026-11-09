# Contributing to Harpsistudio

Harpsistudio is an independent, MIT-licensed beat workstation. Contributions
are welcome: bug reports, focused fixes, tests, documentation, factory patterns
and legally clean sample content.

## Development setup

Requirements:

- Windows 10/11 for the desktop UI
- Rust stable with the MSVC toolchain
- A working audio output for manual UI work

Build and run:

```bat
cargo run --release
```

Run the headless tools without the GUI:

```bat
cargo run -p hs-cli -- list-demos
cargo run -p hs-cli -- inspect "demo-artifacts/projects/Vocal Riddim.harp"
cargo run -p hs-cli -- demo "Midnight House" "target/midnight-house.wav"
```

## Before opening a pull request

Run the same checks used by CI:

```bat
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --no-fail-fast
cargo build --release -p hs-app
cargo build --release -p hs-cli
```

`rust-toolchain.toml` pins the stable toolchain and installs the `rustfmt` and
`clippy` components those checks need.

For UI changes, include a short description of what changed and a 1600x980
screenshot in the pull request. The checked-in screenshots live in
`screenshots/`; raw capture intermediates are ignored.

Keep changes focused. A pull request that changes audio behavior should include
a focused regression test, and a pull request that changes presentation should
explain why the new layout is better.

## Code layout

- `crates/hs-core`: song model, DSP, engine, devices, presets, patterns,
  project I/O, and offline WAV rendering.
- `crates/hs-app`: eframe/egui desktop shell, views, audio device, and MIDI.
- `crates/hs-cli`: headless project inspection and export.

The GUI publishes immutable song snapshots and sends edits through a command
channel. Preserve that separation when changing realtime behavior.

## Audio and asset contributions

Audio is the most license-sensitive part of this project. Do not add a sample,
recording, preset bank, or demo render unless its license permits redistribution
and you have updated the credits.

- Prefer CC0 or public-domain material.
- CC BY material is acceptable only with complete attribution, source URL,
  license URL, and a record of any conversion or trimming.
- Do not add Ableton or other proprietary sample packs, artwork, or UI assets.
- Add every new sound to `crates/hs-core/assets/audio/ATTRIBUTION.md`.
- Keep test fixtures separate from assets that ship in the application binary.

Code contributions are accepted under the MIT License. By submitting a sample
or other asset, you confirm that you have the right to license it under the
terms recorded in the attribution file.

## Adding or changing a drum kit

The Drum Rack bank lives in `crates/hs-core/src/samples.rs`. There are two ways
to add a kit, and both must keep the embedded-file coverage and level tests
green.

### A recorded kit

1. Add mono 44.1 kHz PCM WAV one-shots under
   `crates/hs-core/assets/audio/drums/<kit>/`, one per lane you need.
2. Add each file to `DRUM_FILES` with `include_bytes!`, then add that kit's row
   to `KIT_SAMPLES` using the column order documented above the table.
3. Add the display name to `KIT_NAMES`, add a starting `0.0` entry to
   `KIT_TRIM_DB`, and increase `RECORDED_KITS` so the table lengths still match.
4. Add the matching preset in `crates/hs-core/src/params.rs`; every Drum Rack
   preset becomes a family in the factory SOUNDS library and a preset chip in
   the device view.
5. Record the source, author, license and any conversion in
   `crates/hs-core/assets/audio/ATTRIBUTION.md`, and update the counts in
   `NOTICE`, `README.md` and `demo-artifacts/README.md`.
6. Measure the kit's trim with
   `cargo test -p hs-core --test drum_balance -- --nocapture`. The test prints a
   `KIT_LEVEL` line per kit with `median_rms_db`; set the trim to
   `trim + (-14.80 - median_rms_db)` and repeat until the whole-bank spread is
   at most 0.5 dB.

### A voiced kit

Voiced kits reuse recorded one-shots through the `Voicing` DSP, so they need no
new audio files or licenses. Add a `Voicing` entry to `VOICED_KITS`, add the
name to `KIT_NAMES`, add the preset in `params.rs`, then trim it with the same
balance test. Keep its lane labels tied to the `source` kit.

After either change, run the full gate set above and regenerate the delivery
artifacts with `cargo run -p hs-core --example render_demo_artifacts`. Commit
the updated `demo-artifacts/` files with the code change.

## Reporting bugs and security issues

Use the project's issue tracker for ordinary bugs and feature requests. For
security-sensitive reports, follow `SECURITY.md` rather than opening a public
issue.

Please be respectful and constructive. Harassment, discrimination, and
personal attacks are not welcome here.
