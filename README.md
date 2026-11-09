# Harpsistudio

[![CI](https://github.com/ash3rrrr/harpsistudio/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ash3rrrr/harpsistudio/actions/workflows/ci.yml)

Harpsistudio is a native beat workstation for Windows, built in Rust and
released under the MIT License. It pairs a keyboard-first Session/Arrangement
workflow with an embedded, license-audited sample library, real vocal
processing and a headless renderer. The desktop app and its audio content ship
together: there is no Electron shell, browser runtime or external sample-pack
install.

It follows Ableton Live's interaction model, not its assets: the browser,
Session and Arrangement views, clip/device detail area and mixer feel familiar,
while the DSP, song model and sample bank are original to this project. Import
a WAV or record a take, and it becomes a real track stored inside the project.

Ableton and Live are trademarks of Ableton AG. They are used here only to
describe the interaction model; no Ableton artwork or proprietary UI assets are
included.

## Key features

- **A full native workstation.** Session and Arrangement views, clip launch
  with global quantization and follow actions, a piano roll with scale snap, a
  full mixer, and up to 24 tracks. The arrangement canvas is currently 16 bars.
- **A deep instrument and effect set.** **13 instruments**, **18 effects** and
  **2 MIDI generators**, including the drum rack, Salamander Grand Piano, Vox
  Bass, Vocal Take, Tonewheel Organ, Bassify, Formant Filter, Arpeggiator and
  Chord Generator.
- **Real embedded audio.** **25 drum kits** built from **155 one-shot samples**,
  plus the Salamander Grand Piano samples and a supplied vocal take. No
  separate sample-pack download is required, and the asset licenses are
  recorded in [NOTICE](NOTICE) and
  [crates/hs-core/assets/audio/ATTRIBUTION.md](crates/hs-core/assets/audio/ATTRIBUTION.md).
- **A large factory library.** **1,440 playable sounds** (60 instrument
  families × 24 characters), **100 effect racks**, **64 tagged patterns** and
  **11 complete factory projects**.
- **One engine, two frontends.** The Windows UI and the headless
  `harpsistudio-cli` share the same `hs-core` song and offline-render path, so
  projects, demos and CI exports go through the same audio code.
- **Open interchange.** Export arrangement clips as a Standard MIDI File from
  **FILES** or the CLI, then import one back into a `.harp` project. Tempo,
  track names, clip markers, note timing and velocity survive the round trip;
  channel 10 returns as a Drum Rack track, while melodic tracks load on FM
  Keys. The readable `.harp` project remains the source of truth.
- **Open source without hidden audio.** The code is MIT; the embedded recordings
  keep their original CC0, CC BY or public-domain terms and attribution.

## Quick start

Requirements:

- Windows 10/11
- [Rust](https://rustup.rs) stable (edition 2021) with the MSVC toolchain
- A working system audio output

Build the desktop app with the same command CI uses:

```bat
cargo build --release -p hs-app
```

Launch it:

```bat
target\release\harpsistudio.exe
```

`run.bat` is a convenience wrapper around `cargo run --release`; it also checks
that Cargo is on `PATH` and pauses on failure. `cargo run --release -p hs-app`
builds and launches in one step.

The first release build takes a couple of minutes; later launches start
immediately. A debug build also works, but release is recommended while
producing. The desktop UI targets Windows 10/11. The `hs-core` library and
`hs-cli` also build and test on Ubuntu, which is covered by CI.

## Headless CLI

The `hs-cli` crate is a small, GUI-free frontend to the same song and render
path used by **Mixdown**. It is useful for CI, batch exports, scripts and
contributing without a Windows audio device. It converts arrangement clips to
and from Standard MIDI Files for interchange with another DAW.

Build the CLI with the same command CI uses:

```bat
cargo build --release -p hs-cli
```

The binary is `target\release\harpsistudio-cli.exe`. During development, run it
through Cargo:

```bat
cargo run -p hs-cli -- list-demos
cargo run -p hs-cli -- inspect "demo-artifacts/projects/Vocal Riddim.harp"
cargo run -p hs-cli -- render "demo-artifacts/projects/Vocal Riddim.harp" "target/vocal-riddim.wav"
cargo run -p hs-cli -- demo "Midnight House" "target/midnight-house.wav"
cargo run -p hs-cli -- midi "demo-artifacts/projects/Vocal Riddim.harp" "target/vocal-riddim.mid"
cargo run -p hs-cli -- import-midi "target/vocal-riddim.mid" "target/vocal-riddim-imported.harp"
```

`demo` accepts either a number from `list-demos` or a unique name. `render`
accepts any saved `.harp` project, and `midi` exports its arrangement clips as
a format 1 Standard MIDI File. `import-midi` reads format 0/1 MIDI, restores
tempo, track names, clip markers and notes, and writes a normal `.harp` project;
channel 10 becomes a Drum Rack track and melodic tracks use FM Keys. The
commands create the output directory when needed; `render` writes the same
48 kHz, 16-bit stereo WAV as the desktop app.

## Start here: the Starters tab

The browser opens on **START**, a one-click launchpad for complete projects:

| Starter | What it gives you |
| --- | --- |
| **DUBSTEP** | Half-time drums, growl bass, dark chords, talking Vox Bass and a Salamander piano hook |
| **HOUSE** | Four-on-the-floor groove, offbeat sub, warm stabs, chord bed and a supersaw lead |
| **EDM** | Wide supersaw chords, sidechained pumping and a pluck festival hook |
| **RAP** | Boom-bap drums, felt piano, sample bass, sliding 808 and a dedicated rap-flow topline |

Above the genre cards is the **VOICE → BASS** panel. Choose **DUBSTEP** or
**RAP**, then **LOAD WAV** or **RECORD 8s**. Recording uses a real microphone
when one is producing signal and otherwise captures the system output through
WASAPI loopback, so a take still lands on machines with virtual-only inputs.
Harpsistudio creates a Vocal Take track wired to Bassify, Wobble, compression
and EQ, drops a clip at bar 1 and starts auditioning it. You can also drag a
`.wav` anywhere onto the window. The imported audio is embedded in the project,
so saving and loading never depends on the original file path.

Each starter is an ordinary project. Change the kit, replace devices, edit notes
or save your own version. **PATS** holds the pattern library; **SOUNDS** opens
the 1,440-sound library, the 100 plugin racks and the raw device list; **DEMOS**
has all eleven factory projects; **FILES** handles save, mixdown, MIDI
import/export and recent projects.

## Patterns and MIDI generators

**PATS** is a searchable-by-filter bank of **64 playable patterns**, stored in
`crates/hs-core/assets/patterns.tsv` rather than hardcoded in the UI. Every row
carries a name, genre, role, key, scale, tempo range, bar length, suggested
instrument, note data and a measured gain trim. The bank is normalized by
maximum 500 ms short-term RMS so quiet chord voicings and loud organ/bass
patterns preview at comparable level.

| Genre | Patterns | Roles used |
| --- | --- | --- |
| Dubstep | 8 | bassline, lead, chords, arp, riff, pluck |
| House | 8 | bassline, lead, chords, arp, riff, pluck |
| EDM | 8 | bassline, lead, chords, arp, riff, pluck |
| Techno | 8 | bassline, lead, chords, arp, riff, pluck |
| Rap | 8 | bassline, lead, chords, arp, riff, pluck, rap flow |
| Melodic bass | 8 | bassline, lead, chords, arp, riff, pluck |
| Trap / hip-hop | 8 | bassline, lead, chords, arp, riff, pluck, rap flow |
| Ambient | 8 | bassline, lead, chords, arp, riff, pluck |

Use the genre, role and key chips to narrow the list. Click the triangle to
preview the selected pattern through the engine and its suggested instrument;
**LOAD TRACK** creates a track with that instrument, a session clip and an
arrangement clip at the current bar. Double-clicking a card also loads it.

Two insertable **MIDI FX** devices appear in the browser and the device chain:

- **Arpeggiator** — 1/4, 1/8, 1/16 or 1/32 rate; Up, Down, Up-Down and Random
  modes; 1–4 octaves; gate and swing. It stays diatonic because it reorders the
  notes it receives instead of inventing pitches.
- **Chord Generator** — key and scale (major, minor, dorian, mixolydian,
  phrygian), Diatonic/Major/Minor/7th/Sus4/Add9 quality, inversion and octave.
  It snaps the incoming root into the chosen scale and builds the voicing from
  scale degrees.

Both are real devices in the track chain, save with the project, and render
before the instrument. Chain them (chords → arpeggio) for instant melodic parts.

## Sound library and plugin racks

**SOUNDS** in the browser has three modes:

| Mode | What it does |
| --- | --- |
| **SOUNDS** | **1,440 presets** generated from 60 factory instrument presets across the 12 playable instruments, each shaped by 24 named characters (Deep, Bright, Warm, Wide, Tight, Gritty, Vintage, Plucked, Lush, Punchy and more). Filter by Drums, Bass, Lead, Pluck, Keys, Pad or FX; search by sound, family or instrument; then load the selection onto the selected track. Loading replaces that track's instrument and keeps its clips and effect chain. |
| **PLUGINS** | **100 named effect racks** across Dynamics, Drums, Vocal, Bass, Saturation, Space, Modulation, Filter, Creative and Mastering. Twenty families — Bus Glue, Drum Punch, Drum Room, Vocal Focus, Vocal Air, Bass Weight, Sub Control, Growl Stack, Riddim Wub, Trance Gate, Space Echo, Hall Bloom, Tape Warmth, Lo-Fi Dust, Filter Motion, Stereo Bloom, Pump Machine, Master Polish, Transient Sculpt and Formant Dream — each ship as Subtle, Classic, Heavy, Wide and Vintage. Inserting a rack appends its complete chain to the selected track's effects, where every device can be edited, reordered, bypassed or removed. |
| **DEVICES** | The original raw instrument/effect list and the new-drum-track button. |

Every sound is validated against its instrument's parameter ranges and keeps
enum and integer switches on real positions. The tests load all 1,440 sounds,
insert all 100 racks, and render a library sound through a library rack with
the real song engine. The quota test requires at least 1,000 sounds and at
least 100 plugins. A save/load round-trip test covers one representative from
each of the 60 sound families and all 20 plugin families, while uniqueness
tests reject duplicate parameter settings or chains anywhere in the catalogue.
A headless egui test also drives the shipped SNDS
browser with synthetic clicks and typing, asserting the SOUNDS/PLUGINS/DEVICES
mode switch, category and search filtering, empty-result handling, sound
loading, full-rack insertion and the preserved raw DEVICES actions against the
real application state.

## Workspace layout

The window follows Live 12.4's shell: a compact transport across the top, a
browser down the left edge (toggle it with **BROWSER** at the top-left), and
Session or Arrangement in the large main area with its selector at the
top-right. Clip and Device stack together in the bottom detail area, toggled
from the bottom-right rail; **Tab** swaps Session and Arrangement, **1**/**2**
select them directly, **3**/**4** toggle the details and **5** opens Mixer.
Changing the workspace never closes the clip or device you are editing, and
Mixer replaces the workspace at full width while details remain loaded.

## What the browser and views do

| Area | What it does |
| --- | --- |
| Top bar | Browser toggle, play/stop, rewind, loop, tempo, global launch quantization, position, project name, Save, Mixdown, CPU and master meter |
| Browser | Starters, 64 patterns, 1,440 sounds, 100 plugin racks, all 33 devices, factory demos and project files; **BROWSER** collapses it |
| Main workspace | `1 SESSION` and `2 ARRANGEMENT`, with the selector at the top-right; **Tab** swaps them |
| Detail area | `3 CLIP` and `4 DEVICE` toggle along the bottom and can be open together, side by side |
| Mixer | `5 MIXER` on the bottom-right rail takes the full workspace for channel strips, return and master |
| Status bar | Shortcut hints, messages, MIDI input and the active audio device |

### Session view

- Click an empty cell to create, launch and start a clip.
- Click a filled cell to queue it. The clip starts on the boundary selected by
  the top-bar **LAUNCH** control: **OFF**, **1/16**, **1/8**, **1/4**, **1/2**,
  **1 BAR**, **2 BARS** or **4 BARS**. In Live terms this is the global launch
  quantization; it is saved inside the `.harp` project.
- A queued clip shows **◷ QUEUED** and a clip that owns the track shows
  **▶ PLAYING**. Only the engine confirms a clip as active, so follow-action
  handoffs and track stops are reflected immediately.
- Each track header has a **■** stop button. **■ STOP CLIPS** stops every
  track; **SHIFT+SPACE** does the same from the keyboard.
- Select a session clip, with the detail area closed, to open the clip
  inspector strip below the grid. It exposes **FOLLOW** (NONE, NEXT, PREV,
  FIRST, STOP), **AFTER** (1, 2, 4, 8 or 16 bars), and **LOOP**. A follow action
  queues the next clip on the same musical boundary, so a scene can hand off to
  the following scene without stopping the transport.
- Double-click a clip or use **EDIT NOTES** to open the piano roll in the Clip
  dock while the Session grid stays above it.
- Per-track **M**/**S** toggles mute and solo; **DEV** opens the device rack.
- Scene launch starts a row; **STOP CLIPS** halts the session.
- **＋ SCENE** and **＋ CLIP** add rows and clips.

### Arrangement view

- A 16-bar timeline. Drag clips to move them; the ruler seeks the transport.
- Click an empty lane to stamp a new clip at the nearest bar.
- Double-click a clip to edit notes.

### Mixer

- Press **5** or click **MIXER** on the bottom-right rail to fill the window
  with the mixer.
- One strip per track: fader, pan, mute/solo, level meter and return send.
- Shared reverb return and a master strip with limiter.
- Click a strip to select its track for the device view.

### Device view

- Press **4** or use **DEV** on a track header; the rack opens in the bottom
  detail area while Session or Arrangement stays visible above. Clip and Device
  can both stay open, stacked side by side.
- Instrument/effect chain on the left; click a slot to select it.
- Preset chips load instant starting points; **INIT** resets the device.
- Parameter knobs are drag-to-adjust; double-click resets; hold SHIFT for fine
  control.
- Effects can be toggled, reordered and removed.
- Master and return pages expose their own processing.
- Drum Rack displays all 16 pad lanes and the currently loaded kit.

### Piano roll

- Press **3**, double-click a clip, or use **EDIT NOTES** to open the Clip dock.
- Click to draw a note, drag to move, drag the right edge to resize, and click
  a note to audition it through its track's instrument.
- **Shift+click** adds or removes notes from a multi-selection; drag any
  selected note to move the whole group. **DEL** or right-click erases.
- **Ctrl+C / Ctrl+X / Ctrl+V / Ctrl+D** copy, cut, paste and duplicate notes.
  Paste lands at the mint cursor line left by the last click. **Ctrl+A**
  selects every note.
- **KEY**, **MODE** and **SNAP** in the toolbar highlight the chosen scale and
  optionally snap drawn or moved notes into it. The settings save with the
  project.
- Header **LENGTH** and **VELOCITY** chips apply to the selection as well as
  new notes. **QUANTIZE 1/16** snaps the selection, or the whole clip when
  nothing is selected.
- Drum Rack tracks open as **16 named pads** using the active kit's labels
  (Kick, Snare, Clap, Hats, Toms, percussion and mallet names), so beats are
  drawn directly on the pad rows.
- Create original parts from the Session action bar with **＋ DRUMS**,
  **＋ MELODY**, **＋ BASS** and **＋ VOCAL**. The first three open a fresh,
  empty piano-roll clip; **＋ VOCAL** imports a WAV take and builds the
  Vocal Take → Bassify → Wobble chain.

## Instruments (13)

| Device | Character |
| --- | --- |
| **Vocal Take** | A sampler-backed lane for a real vocal. Load a WAV or record the mic; the take is stored inside the project. Put **Bassify** and **Wobble** after it for a talking dubstep bass. |
| **Drum Rack** | Twenty-five drum kits with 16 lanes each: thirteen recorded kits (acoustic, house, trap, dubstep, boom-bap, bounce, three Salamander layers, world and orchestral percussion, mallets, TR-808) plus twelve variants voiced from those same samples at load time. Tune, decay, punch, snap, tone and drive are applied to the samples. All 25 preset chips wrap onto multiple rows, so every kit remains selectable. |
| **Salamander Grand** | Real Salamander Grand Piano samples, 13 roots × soft/hard layers, with tone, release, detune, octave, velocity sensitivity and width. The **Salamander Bass** preset makes it a warm sample bass. |
| **Vox Bass** | A vocal-formant synth bass: vowel shapes, sub, growl, drive and tempo-free wobble. It is a playable vocal-bass source when you do not have a recording. |
| Sub Bass | Pure low end with drive, glide and envelope control. |
| Reese Bass | Detuned saw stack with moving filter — dark DnB, techno and bass design. |
| Growl Bass | Wavetable-style dubstep growl with LFO wobble and FM character. |
| Supersaw | Wide detuned stack for festival leads and chords. |
| House Pluck | Short, plucky stabs with filter snap. |
| FM Keys | Two-operator keys for deep-house and rap chords. |
| **Tonewheel Organ** | Nine-drawbar additive organ with key click, rotary tremolo and drive. Gospel, house and dub presets cover chords and lead lines; the **Organ House** demo puts it in a club context. |
| Wide Pad | Lush detuned bed with slow filter movement. |
| Riser FX | Noise/tone riser for transitions and drops. Its Length is in beats. |

## Effects (18)

Filter, Drive, Bitcrush, Reverb, Delay, Chorus, Compressor, EQ Three, Phaser,
Sidechain, Auto Pan, Utility, **Bassify**, **Formant Filter**, **Wobble**,
**Tape**, **Transient** and **Multiband**.

## MIDI generators (2)

| Device | Controls | Use |
| --- | --- | --- |
| **Arpeggiator** | Rate, Mode, Octaves, Gate, Swing | Turn held chords into a moving part; presets cover trance gate, pluck, riddim chatter and a wide cascade. |
| **Chord Generator** | Key, Scale, Quality, Inversion, Octave | Turn a bassline or single-note idea into scale-locked chords; presets include diatonic triads, seventh stack, sus lift, minor mood and major anthem. |

MIDI generators are bypassable and reorderable like effects, but they run on
notes rather than audio. The UI labels the chain **DEVICE CHAIN** so both types
read as one insert path.

The new processors fill specific production roles:

- **Bassify** — formant band-splitting, sub weighting, wavefolding and wobble
  modulation. The **Vocal Take** → **Bassify** → **Wobble** chain is the real
  vocal-to-bass path; the same effect also works on Vox Bass.
- **Wobble** — an LFO-controlled resonant filter for instant dubstep movement.
  Low rates give long wubs; high rates give riddim chatter.
- **Tape** — warm saturation, flutter and a soft top end for drums, keys or the
  master bus.
- **Transient** — adds attack snap and body without simply raising level; ideal
  on sampled drums.
- **Multiband** — three-band saturation for weight, presence and controlled
  highs.
- **Formant Filter** — three vowel band-passes that morph A·E·I·O·U under a
  slow LFO. Use it on organ, pads or vocals for talking movement, or before
  Bassify when you want the vowel sweep without the sub layer.

## Real sample library

All audio is embedded, so kits and piano work offline with no import dialog:

- **155 drum one-shots** across **25 kits**, normalized to mono 44.1 kHz
  16-bit WAV. The original six kits are joined by Salamander Studio, Salamander
  Hard, Salamander Soft, World Percussion, Orchestral Percussion, Mallet Lab
  and TR-808 Studio. Twelve more kits are voiced from those recordings at load
  time (pitch, filtering, saturation, bit reduction and tail shaping), so no new
  audio files or sample licenses are added.
- **Measured kit balance:** one gain-only trim per kit is applied after the
  Drum Rack drive stage. A 48 kHz whole-kit probe aligned each kit's median
  50 ms RMS to −14.80 dBFS; the 25-kit spread is 0.31 dB (the 13 recorded kits
  span 0.25 dB and the voiced kits were trimmed to sit inside it). The highest
  post-trim pre-master peak was −4.73 dBFS (−3.14 dBFS through the default
  master path). No WAV, EQ, compression
  or trigger timing was changed. Trims (dB): Acoustic +0.9, House +0.9,
  Trap 808 0.0, Dubstep −0.3, Boom Bap +2.1, Bounce +0.1, Salamander Studio
  −0.7, Salamander Hard −1.3, Salamander Soft +2.9, World Percussion +2.7,
  Orchestral Percussion +0.9, Mallet Lab −2.2, TR-808 Studio −1.2; voiced kits
  −3.6 to −0.3. Full before/after measurements are in `ATTRIBUTION.md`.
- **26 Salamander piano samples**: 13 roots from MIDI 33–105 in two velocity
  layers, five seconds each.
- 25 kit mappings × 16 lanes: kick, snare, clap, closed hat, open hat, low tom,
  crash, rim, 808, percussion, shaker, ride, cowbell, snap, floor tom and high
  tom. Pads retain their kit-specific labels, including mallet notes and world
  percussion names.
- Imported vocal takes are stored as mono PCM16 in the `.harp` project, so they
  are portable even if the original WAV is moved or deleted.
- Sample paths and source licenses: `crates/hs-core/assets/audio/ATTRIBUTION.md`.
  The automated vocal test uses a separate CC0 sung-note fixture that is not
  embedded in the app binary.

The Drum Rack selects the kit from its **Kit** parameter. The piano chooses the
nearest recorded root and the appropriate velocity layer automatically. The
Salamander Grand is also available as a **MIDI-playable instrument**: use the
computer keyboard or a MIDI controller and play it like any synth.

## Factory demos (11)

| Demo | Tempo | Style |
| --- | --- | --- |
| Midnight House | 124 | House stabs, pumping sub, festival lead |
| Riddim Protocol | 140 | Half-time drums, wobbling growl bass, riser |
| Vocal Riddim | 140 | Chopped vocal hook, talking vocal bass, half-time double drop |
| Big Room Anthem | 128 | Wide sidechained supersaw chords, drums-out build, bar-9 drop |
| Deep Groove | 122 | FM keys, warm pad, shuffled groove |
| Warehouse Techno | 132 | Driving Reese bass, metallic hats, dark stabs |
| Neon Cathedral | 150 | Half-time dubstep kit, talking Vox Bass, Salamander piano |
| Velvet Sky | 108 | Salamander Grand, sample bass, warm sub, organic groove |
| Cypher Room | 90 | Boom-bap drums, felt piano, sample bass, sliding 808 and vocal-forward rap flow |
| Organ House | 124 | Tonewheel organ chords, vowel-swept formant layer and gospel lead |
| Live Room | 118 | Acoustic Salamander kit, hand percussion, warm sub and Salamander piano |

## Fast recipes

### Real vocal → talking dubstep bass

Open **START**, choose **DUBSTEP** or **RAP** in the Voice → Bass panel, then
click **LOAD WAV** (or **RECORD 8s**). If the microphone is silent, RECORD
automatically captures the system output instead and names the take **System
loopback take**. The app creates a **Vocal Bass** track
with the take as its source, then runs **Vocal Take → Bassify → Wobble →
Compressor → EQ Three**. It adds both a session clip for auditioning and an
arrangement clip at bar 1, so **Mixdown** exports the vocal bass too. Sing a
sustained note or a short phrase; pitch drift becomes bass movement. Adjust
**Pitch** on Vocal Take (usually −7 to −12 semitones), the Bassify vowel, and the
Wobble rate (2–8 Hz) for the amount of talking movement.

### Dubstep

Open **START → DUBSTEP** and press SPACE. The Vox Bass track already has
Bassify and Wobble. Add a real vocal with **LOAD WAV** in the panel above. Try
the **Vox Bass** presets, then edit Wobble rate (2–8 Hz is the classic range) or
insert **Growl Bass** for a second layer. The Drum Rack is on the **Dubstep**
kit; change **Kit** if you want a cleaner trap or acoustic top.

For a complete vocal-led arrangement, load **Vocal Riddim** from **FACTORY
DEMOS**. It chops the embedded vocal take into the hook and pitches the same
take down through **Vocal Take → Bassify → Wobble** for the growl. Intro, first
drop, drums-out break and second drop are already arranged over 16 bars. The
project is portable because the take is embedded in the song.

### House

Open **START → HOUSE**. The sub is sidechained for the pump; open the Sidechain
device and adjust Depth/Release. For a deeper track, load **Deep Groove** and
replace the FM chords with Salamander Grand for a live piano texture. **Live
Room** is the organic house variation: it combines the Salamander Studio kit
with World Percussion and a live piano hook.

### EDM

Open **START → EDM** for sidechained supersaws and a festival hook. Big Room
Anthem is arranged as four bars of drums, four bars of drums-out build with the
supersaws and riser, then a bar-9 drop with full drums, sub and lead. The same
Supersaw device handles chords and lead; use **Wide Chords** and **Festival
Lead** presets, then shape the top with EQ Three and Tape.

### Rap

Open **START → RAP**. The beat uses the Boom Bap sample kit with felt piano and
a sample bass. The sliding 808 is on the Sub Bass track; use the 808 Slide
preset's Glide for slides between notes. The dedicated Rap Flow enters at bar 9
and is levelled and shaped to sit above the bed as the foreground part. For a
vocal flip, choose **RAP** in the Voice → Bass panel and load a spoken phrase;
that preset uses a −7 semitone shift and a slower Bassify/Wobble setting. For a
more modern pocket, change the Drum Rack to the Trap 808 kit.

## Playing live

Select a track, then use the computer keyboard:

| Keys | Note |
| --- | --- |
| `A` | C |
| `W` | C# |
| `S` | D |
| `E` | D# |
| `D` | E |
| `F` | F |
| `T` | F# |
| `G` | G |
| `Y` | G# |
| `H` | A |
| `U` | A# |
| `J` | B |
| `K` | C (next octave) |

`Z`/`X` shift the keyboard octave (C1–C7). Computer keys use velocity 0.8.
MIDI keyboards are opened automatically; note on/off, velocity and sustain
(CC64) go to the selected track. The status bar shows the active MIDI port. Only
the first port is opened, so restart after plugging one in.

Live notes use their own voice bank, so they do not steal or release notes from
clips on the same track.

## Keyboard shortcuts

| Key | Action |
| --- | --- |
| `SPACE` | Play / stop |
| `SHIFT+SPACE` | Stop all session clips |
| `A W S E D F T G Y H U J K` | Play the selected track |
| `Z` / `X` | Shift computer-keyboard octave |
| `1` / `2` | Switch the main workspace to Session / Arrangement |
| `Tab` | Swap Session and Arrangement |
| `3` / `4` | Toggle Clip / Device in the bottom detail area |
| `5` | Toggle the full-width Mixer |
| `←` / `→` | Jump one bar |
| `DEL` | Delete selected note, or selected clip outside the piano roll |
| `SHIFT+DEL` | Delete selected clip from the piano roll |
| `CTRL+S` | Save |
| `ESC` | Panic: stop all sound and clear launched clips |

## Files and export

Projects, mixdowns and MIDI exports live in:

```text
%USERPROFILE%\Documents\Harpsistudio
```

- **Save** writes a readable `.harp` JSON project named after the song. Imported
  vocal takes are embedded as base64 PCM16, so the project is self-contained.
- **Mixdown** renders the arrangement (or the first session clip on each track
  when there is no arrangement) to a 16-bit stereo 48 kHz WAV. An imported
  vocal adds an arrangement clip automatically, so it is included in Mixdown.
- **EXPORT MIDI** in the Files tab writes a format 1 Standard MIDI File with
  the song tempo, track names, clip markers and notes. Drum tracks use MIDI
  channel 10; other tracks use channel 1. If there is no arrangement, it
  exports the first session clip on each track, matching Mixdown.
- **IMPORT MIDI** sits next to **EXPORT MIDI**: choose a format 0/1 `.mid`
  file, and Harpsistudio rebuilds tracks and arrangement clips, loads the song,
  and saves it to `Documents\Harpsistudio` as a `.harp` project. Clip markers
  become arrangement clips, channel 10 becomes a Drum Rack track, and melodic
  tracks load with FM Keys. The MIDI file does not contain the original
  instruments or effects, so those are intentionally not reconstructed.
- The audio engine runs on the system output at its native sample rate. If the
  device is unavailable, editing/export still work; use the **AUDIO OFF**
  message or **RETRY AUDIO** in the Files tab.

## Delivery proof artifacts

The checked-in `demo-artifacts/` directory contains four complete genre
mixdowns, a supplied-vocal Bassify render, a 25-kit drum-library audition, a
Salamander + MIDI-FX render, and the measured reports. The four starters are
rendered from the shipped `presets::demo_song` factory songs through
`project::export_wav_to`, the same renderer behind the UI's **MIXDOWN** action.
The vocal proof uses the same `AudioTake::from_wav_file` →
`presets::add_vocal_bass_track` path as **VOICE → BASS**. See
`demo-artifacts/README.md` and `demo-artifacts/reports/delivery-report.md` for
the per-file evidence.

Regenerate the folder with:

```bat
cargo run -p hs-core --example render_demo_artifacts
```

## Project layout

```text
harpsistudio/
├─ .github/
│  ├─ workflows/ci.yml          # Windows + Ubuntu gates
│  ├─ ISSUE_TEMPLATE/           # bug and feature issue forms
│  ├─ PULL_REQUEST_TEMPLATE.md
│  ├─ CONTRIBUTING.md           # development and audio-contribution rules
│  └─ CODE_OF_CONDUCT.md
├─ crates/
│  ├─ hs-core/                  # DSP, song model, engine, presets, project I/O
│  │  ├─ src/dsp.rs             # oscillators, filters, envelopes, delays
│  │  ├─ src/devices.rs         # instrument voices and effect processing
│  │  ├─ src/engine.rs          # realtime engine and offline rendering
│  │  ├─ src/audio_take.rs      # imported takes, WAV loading and PCM16 project data
│  │  ├─ src/params.rs          # 33-device catalogue, parameter definitions
│  │  ├─ src/patterns.rs        # tagged 64-pattern library parser and model
│  │  ├─ src/midi_fx.rs         # arpeggiator and chord-generator note transforms
│  │  ├─ src/samples.rs         # embedded drum/piano bank and WAV decoder
│  │  ├─ src/presets.rs         # factory demos and default patterns
│  │  ├─ assets/patterns.tsv    # shipped pattern rows (name, tags, notes)
│  │  ├─ src/midi_export.rs      # format 1 Standard MIDI File writer
│  │  ├─ src/midi_import.rs      # format 0/1 Standard MIDI File reader
│  │  ├─ src/project.rs         # .harp save/load, WAV and MIDI export
│  │  ├─ src/song.rs            # Song/Track/Clip/Note model
│  │  └─ examples/render_demo_artifacts.rs # reproducible delivery proof
│  ├─ hs-app/                   # eframe/egui interface and CPAL audio device
│  │  ├─ src/app.rs             # shell, transport, browser, actions
│  │  ├─ src/view_*.rs          # session, arrangement, mixer, device, piano roll
│  │  ├─ src/widgets.rs         # knobs, faders, meters, chips, note previews
│  │  ├─ src/audio.rs           # CPAL stream and engine callback
│  │  └─ src/midi.rs            # MIDI input enumeration and note/sustain forwarding
│  └─ hs-cli/                   # headless render, inspect and demo CLI
├─ demo-artifacts/              # checked-in renders, reports and supplied vocal source
├─ screenshots/                 # README UI captures
├─ LICENSE                      # MIT for the source code
├─ NOTICE                       # asset licenses and trademark disclaimer
├─ SECURITY.md                  # responsible disclosure
├─ CHANGELOG.md
├─ run.bat
└─ README.md
```

The UI publishes immutable song snapshots to the audio callback; parameter edits
and launches travel through a command channel. Voices and effect state live
across snapshots, so turning a knob does not restart notes.

## Development

Run the same gates as CI:

```bat
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --no-fail-fast
cargo build --release -p hs-app
cargo build --release -p hs-cli
```

CI runs that set on Windows and Ubuntu (`.github/workflows/ci.yml`). The
desktop UI is developed and manually checked on Windows; the headless core and
CLI also build and test on Linux. The Ubuntu job smoke-tests `list-demos`,
`inspect`, MIDI export and MIDI import through the release CLI binary.

Tests cover the 25-kit/155-file sample bank, live-note release/sustain,
clip+live coexistence, offline render checks for the factory demos, individual
renders of every one-shot (400 kit-lane probes, 155 unique files), a measured
kit-balance assertion, and a real WAV vocal fixture rendered
through import → clip → Vocal Take → Bassify/Wobble. The vocal tests assert the
result is finite, non-silent, non-clipping and has energy in the bass band. A
capture-path test drives the same interleaved-buffer conversion the microphone
and WASAPI loopback use (i16 and f32) with a real audio buffer, then renders the
resulting take. Another test checks that takes survive project save/load inline.

The pattern tests assert library counts and tag coverage, scale membership and
bar bounds for every note, three-note chord voicings, single-line arpeggios,
audible finite previews with headroom for all 64 patterns, a measured
loudness-spread assertion, a layered preview per genre, and MIDI-FX output for
given arpeggiator/chord settings. Engine regression tests cover the global
launch-quantum boundary, the queued clip state, and a NEXT follow-action
handoff at the one-bar mark. A project test
proves the MIDI chain survives save/load. A MIDI export test verifies the
format-1 header, tempo meta event, track names, note timing and velocity, and
the percussion channel. MIDI importer fixtures cover running status, channel
10, truncated data and missing end-of-track events, and a `midi_round_trip`
integration test exports **Vocal Riddim**, imports it and checks all 424 notes,
9 tracks, 15 clip markers and the tempo. A starter test proves each of
DUBSTEP, HOUSE, EDM and RAP carries a bassline, chord voicing and melodic lead,
with dedicated rap-flow material. The `delivery` integration test exports all
four starters through `project::export_wav_to` at 48 kHz, checks their final PCM
for cleanliness and length, and verifies the dubstep wobble, house four-on-the-
floor, EDM build/drop and vocal-forward rap flow. It also covers the supplied-
vocal Bassify export, every drum-library mapping and a Salamander + MIDI-FX
export.

## Open source

Harpsistudio is released under the [MIT License](LICENSE). You can use, modify,
share and redistribute the source code, including commercially, provided the
copyright and license notice stay with it.

The MIT license covers the **code**. The embedded drum samples, Salamander
piano, vocal take, test fixture and demonstration renders keep their original
licenses and attribution. See [NOTICE](NOTICE) and
[crates/hs-core/assets/audio/ATTRIBUTION.md](crates/hs-core/assets/audio/ATTRIBUTION.md)
before redistributing the application or rendered audio.

Contributions are welcome. [CONTRIBUTING.md](.github/CONTRIBUTING.md) covers the build
gates, code layout, screenshot expectations and the rules for legally clean
audio assets. [SECURITY.md](SECURITY.md) explains private vulnerability
reporting, and [CODE_OF_CONDUCT.md](.github/CODE_OF_CONDUCT.md) sets the community
expectations.

### Roadmap and good first issues

These are verified gaps in the current tree, not a generic wish list:

1. **Good first issue — version `.harp` files.** Projects are currently
   pretty-printed `Song` JSON with no format field
   (`crates/hs-core/src/project.rs`). Add a schema version and a clear
   migration/error path for older or newer files.
2. **Good first issue — exercise Linux desktop startup.** CI builds `hs-app` on
   Ubuntu, but the UI is only manually checked on Windows. Document a tested
   distro/audio stack and add a headless startup smoke test.
3. **Larger project — MIDI mapping and multiple ports.** Only the first input
   port is opened, and only note, velocity, sustain (CC64) and all-notes-off are
   handled (`crates/hs-app/src/midi.rs`). Add port selection and CC learn.
4. **Larger project — richer MIDI import.** The importer handles format 0/1,
   4/4 files and performance data, but not automation, pitch bend, tempo maps,
   odd meters or original instrument identity. Add those deliberately with
   round-trip fixtures.
5. **Larger project — longer arrangements and richer audio import.**
   Arrangement editing is a 16-bar canvas, and audio import is WAV-only with
   eight-second mono takes. Extend both deliberately with regression coverage.

Read [`.github/CONTRIBUTING.md`](.github/CONTRIBUTING.md) for the exact gate set,
the audio-asset rules and the drum-kit workflow. Fork the repository, create a
topic branch, run the **Development** commands above, and open a pull request
using the PR template. For any audio contribution, include complete license and
attribution details.

## Limitations

- WAV import only (PCM and 32-bit float); MP3/AIFF and stem separation are not
  implemented. Recording is capped at eight seconds per take and takes are
  mixed to mono. The recorder prefers a physical input and falls back to WASAPI
  system-output loopback when that input is silent or unavailable.
- Takes are stored uncompressed in the project, so very long vocal files make
  `.harp` files larger.
- No external VST/AU hosting.
- Arrangement editing uses a 16-bar canvas; the loop range is derived from song
  length.
- Follow actions apply to session clips only. Arrangement clips still play in
  their own timeline and ignore the follow controls.
- Up to 24 tracks; stereo output only.
- Mixdown renders synchronously, so the window pauses briefly during export.
- MIDI recording/controller mapping is limited to note, velocity and sustain;
  only the first input port is opened.
- MIDI import supports format 0/1, ticks-per-quarter timing and 4/4. SMPTE
  timing and other meters fail clearly. Instruments and effects do not travel
  in MIDI, so imported melodic tracks use FM Keys and percussion uses Drum
  Rack.
