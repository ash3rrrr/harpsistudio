# Security policy

Harpsistudio is a local desktop application: it reads project and WAV files,
writes projects and mixdowns, and opens MIDI/audio devices. Security-sensitive
reports include memory-safety issues, malformed-file crashes, path traversal,
unsafe audio-device handling, and accidental disclosure of project contents.

Please do not open a public issue for a security-sensitive report. Use the
repository's private security advisory feature, or contact the maintainers
through the contact method listed on the repository profile. Include:

- the affected version or commit,
- the operating system and Rust toolchain,
- a minimal reproduction or the smallest project/WAV file that triggers it,
- the expected and actual behavior, and
- any relevant logs with personal paths and audio content removed.

Do not attach a private `.harp` project or recording unless a maintainer asks
for a minimal copy. This project does not currently operate a bug bounty
program. Reports will be acknowledged as soon as practical, and coordinated
disclosure will be preferred once a fix is available.
