## Summary

<!-- What problem does this solve, and what did you change? -->

## Related issue

<!-- Link the issue, for example: Closes #123 -->

## Type of change

- [ ] Bug fix
- [ ] New feature or device
- [ ] Audio/DSP behavior
- [ ] Sample or factory content
- [ ] Documentation
- [ ] Build or CI

## Verification

Run the same gates as CI:

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace --no-fail-fast`
- [ ] `cargo build --release -p hs-app`
- [ ] `cargo build --release -p hs-cli`

For changes that affect sound or projects, include the focused regression test or render evidence:

<!-- Commands, test names, and before/after measurements if relevant. -->

For UI changes, attach a 1600x980 screenshot:

<!-- Drag the screenshot here. -->

## Checklist

- [ ] The change is focused and does not include unrelated refactors.
- [ ] New audio assets are redistributable and recorded in `crates/hs-core/assets/audio/ATTRIBUTION.md`.
- [ ] Counts or behavior changes are reflected in `README.md`, `NOTICE`, and `CHANGELOG.md` where applicable.
- [ ] The project loads existing `.harp` files, or the compatibility impact is documented.
- [ ] I have read `.github/CONTRIBUTING.md`.

## Notes for reviewers

<!-- Known limitations, follow-up work, or areas that deserve extra scrutiny. -->
