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

## Reporting bugs and security issues

Use the project's issue tracker for ordinary bugs and feature requests. For
security-sensitive reports, follow `SECURITY.md` rather than opening a public
issue.

Please be respectful and constructive. Harassment, discrimination, and
personal attacks are not welcome here.
