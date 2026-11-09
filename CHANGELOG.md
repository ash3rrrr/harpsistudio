# Changelog

All notable changes to Harpsistudio are documented here. The project follows
[Semantic Versioning](https://semver.org/) for the application version.

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
