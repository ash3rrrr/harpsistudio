# Audio sample credits

Harpsistudio embeds its **13 drum kits / 155 one-shot WAV files** and the
**26-file Salamander Grand Piano** in the `hs-core` binary. The drum one-shots
and piano are converted to mono 44.1 kHz, 16-bit PCM WAV for predictable CPU
use and offline playback. The musical performances and recordings remain the
work of their original creators.

## Kit inventory

| # | Kit | Recorded source | License |
| --- | --- | --- | --- |
| 1 | Acoustic Studio | Pearl Master Studio | CC BY 3.0 |
| 2 | House Club | Pearl Master Studio | CC BY 3.0 |
| 3 | Trap 808 | Boochi44 free drum samples | CC0 1.0 |
| 4 | Dubstep | Boochi44 free drum samples | CC0 1.0 |
| 5 | Boom Bap | Boochi44 free drum samples | CC0 1.0 |
| 6 | Bounce | Boochi44 free drum samples | CC0 1.0 |
| 7 | Salamander Studio | Salamander Drumkit | Public domain (2022 dedication) |
| 8 | Salamander Hard | Salamander Drumkit | Public domain (2022 dedication) |
| 9 | Salamander Soft | Salamander Drumkit | Public domain (2022 dedication) |
| 10 | World Percussion | FreePats World Percussion | CC0 1.0 |
| 11 | Orchestral Percussion | VSCO 1 Percussion (VSCO 2 CE) | CC0 1.0 |
| 12 | Mallet Lab | VSCO 2 Community Edition mallets | CC0 1.0 |
| 13 | TR-808 Studio | TidalCycles TR-808 Fischer samples | CC0 1.0 |

The library contains 155 files: 43 from the original six-kit release and 112
new, unique one-shots added in the expanded library. All 155 are reachable from
a kit lane.

## Pearl Master Studio drums

- **Kits:** Acoustic Studio, House Club
- **Recorded by:** enoe
- **Source:** <https://oramics.github.io/sampled/DRUMS/pearl-master-studio/>
- **License:** Creative Commons Attribution 3.0
  (<https://creativecommons.org/licenses/by/3.0/>)
- **Files used:** `kick-01`, `snare-01`-`snare-03`, `hihat-closed`,
  `hihat-open`, `tom-01`-`tom-03`, `crash-01`, `ride-01`, `splash-01`.
- **Used in:** Acoustic Studio, House Club, and the sample-forward demos.
- **Coverage:** all 12 Pearl one-shots are reachable. `splash-01` is assigned
  to Acoustic Studio's Perc lane instead of a duplicate closed-hat reference,
  so the embedded bank has no orphaned file.

## Boochi44 free drum samples

- **Kits:** Trap 808, Dubstep, Boom Bap, Bounce
- **Source:** <https://github.com/Boochi44/free-drum-samples>
- **License:** CC0 1.0 Universal
  (<https://creativecommons.org/publicdomain/zero/1.0/>)
- **Packs used:** `01-hard-trap`, `02-bounce`, `03-soulful-vintage`.
- **Files used:** kicks, snares, claps, closed/open hats, toms, crashes, rim,
  808s, cowbell, shaker and percussion from the packs above.

## Salamander Drumkit

- **Kits:** Salamander Studio (7), Salamander Hard (8), Salamander Soft (9)
- **Recorded by:** Alexander Holm
- **Source:** <https://archive.org/details/SalamanderDrumkit>
- **License:** Public-domain dedication dated 2022-03-04. The original release
  was CC BY-SA 3.0; the author's 2022 release note dedicates the library to the
  public domain. Attribution is retained here for the recordings.
- **Character:** Three dynamics of an acoustic kit, with overhead and direct
  articulations. The three Harpsistudio kits use soft, studio and hard layers
  respectively.
- **Used in:** Salamander Studio, Salamander Hard, Salamander Soft and the
  **Live Room** demo.

### Exact one-shot mapping

### salamander-studio (kit 7)

```text
00  kick.wav         <- kick_OH_P_2.flac
01  snare.wav        <- snare_OH_MP_1.flac
02  clap.wav         <- snareStick_OH_F_1.flac
03  hat-closed.wav   <- hihatClosed_OH_P_1.flac
04  hat-open.wav     <- hihatOpen_OH_P_1.flac
05  tom-low.wav      <- loTom_OH_MP_1.flac
06  crash.wav        <- crash1_OH_P_1.flac
07  rim.wav          <- snareOFF_OH_P_1.flac
08  808.wav          <- kick2_OH_P_1.flac
09  percussion.wav   <- splash1_OH_P_1.flac
10  shaker.wav       <- hihatFoot_OH_MP_1.flac
11  ride.wav         <- ride1_OH_MP_1.flac
12  cowbell.wav      <- cowbell_P_1.flac
13  snap.wav         <- snare2_OH_MP_1.flac
14  tom-floor.wav    <- loTom_OH_PP_1.flac
15  tom-high.wav     <- hiTom_OH_P_1.flac
```

### salamander-hard (kit 8)

```text
00  kick.wav         <- kick_OH_FF_2.flac
01  snare.wav        <- snare_OH_FF_2.flac
02  clap.wav         <- snareStick_OH_F_2.flac
03  hat-closed.wav   <- hihatClosed_OH_F_2.flac
04  hat-open.wav     <- hihatOpen_OH_FF_2.flac
05  tom-low.wav      <- loTom_OH_FF_2.flac
06  crash.wav        <- crash1_OH_FF_2.flac
07  rim.wav          <- snareOFF_OH_F_2.flac
08  808.wav          <- kick2_OH_FF_2.flac
09  percussion.wav   <- china1_OH_FF_2.flac
10  shaker.wav       <- hihatFootStomp_OH_MP_1.flac
11  ride.wav         <- ride1_OH_FF_2.flac
12  cowbell.wav      <- cowbell_FF_2.flac
13  snap.wav         <- snare2_OH_FF_2.flac
14  tom-floor.wav    <- loTom_OH_FF_3.flac
15  tom-high.wav     <- hiTom_OH_FF_3.flac
```

### salamander-soft (kit 9)

```text
00  kick.wav         <- kick_OH_F_2.flac
01  snare.wav        <- snare_OH_F_2.flac
02  clap.wav         <- snare2OFF_OH_F_1.flac
03  hat-closed.wav   <- hihatClosed_OH_F_3.flac
04  hat-open.wav     <- hihatOpen_OH_F_2.flac
05  tom-low.wav      <- loTom_OH_PP_2.flac
06  crash.wav        <- crash2_OH_P_1.flac
07  rim.wav          <- snareStick_OH_F_3.flac
08  808.wav          <- kick2_OH_F_2.flac
09  percussion.wav   <- china2_OH_FF_1.flac
10  shaker.wav       <- hihatFoot_OH_MP_2.flac
11  ride.wav         <- ride2_OH_PP_1.flac
12  cowbell.wav      <- cowbell_MP_2.flac
13  snap.wav         <- snare2_OH_F_2.flac
14  tom-floor.wav    <- loTom_OH_MP_2.flac
15  tom-high.wav     <- hiTom_OH_F_2.flac
```

## FreePats World Percussion

- **Kit:** World Percussion (10)
- **Recorded by:** the FreePats project contributors; see the package README for
  the individual instrument recordists.
- **Source:** <https://github.com/freepats/world-percussion>
- **License:** CC0 1.0 Universal
  (<https://creativecommons.org/publicdomain/zero/1.0/>)
- **Character:** Acoustic cajon, bongos, congas, darbuka, claves, castanets,
  hand claps, tambourine, egg shaker and maracas.
- **Used in:** World Percussion and the **Live Room** demo.

### Exact one-shot mapping

```text
00  kick.wav         <- CajonFlamenco/101.flac
01  snare.wav        <- Bongos/1_01.flac
02  clap.wav         <- HandClap/01_02.flac
03  hat-closed.wav   <- EggShaker/fast_01.flac
04  hat-open.wav     <- Tambourine/01_01.flac
05  tom-low.wav      <- LowConga/v3_01_01.flac
06  crash.wav        <- Darbuka/pa_03_01.flac
07  rim.wav          <- Claves/01.flac
08  808.wav          <- CajonFlamenco/201.flac
09  percussion.wav   <- Conga/v2_01_01.flac
10  shaker.wav       <- Maracas/01_01.flac
11  ride.wav         <- Castanets/01.flac
12  cowbell.wav      <- Darbuka/tak_02_01.flac
13  snap.wav         <- HandClap/02_02.flac
14  tom-floor.wav    <- LowConga/v4_01_01.flac
15  tom-high.wav     <- Bongos/2_01.flac
```

## VSCO 2 Community Edition percussion

- **Kits:** Orchestral Percussion (11), Mallet Lab (12)
- **Recorded by:** Sam Gossner and Simon Dalzell; sample cutting by Elan
  Hickler.
- **Source:** <https://github.com/sgossner/VSCO-2-CE>
- **License:** CC0 1.0 Universal
  (<https://creativecommons.org/publicdomain/zero/1.0/>)
- **Libraries used:** the `VSCO 1 Percussion` folder for orchestral bass drum,
  snare, tenor drum, cymbals, claves and shaker; the `VSCO 2 CE` marimba,
  glockenspiel, xylophone and tubular-bell samples for Mallet Lab.
- **Character:** Real orchestral percussion and pitched mallet hits, not
  synthesized tones. Mallet samples were 24-bit recordings, converted to 16-bit
  for the embedded bank.
- **Used in:** Orchestral Percussion and Mallet Lab.

### Exact one-shot mapping: Orchestral Percussion

```text
00  kick.wav         <- VSCO 1 Percussion/drums/bass/bdrum3_fff_1.wav
01  snare.wav        <- VSCO 1 Percussion/drums/snare/drum1/snare1_fff_1.wav
02  clap.wav         <- VSCO 1 Percussion/drums/snare/drum1/snare1_rimshot_fff_1.wav
03  hat-closed.wav   <- VSCO 1 Percussion/varMetal/various/sleighbell1_hit_loud_1.wav
04  hat-open.wav     <- VSCO 1 Percussion/varMetal/Cymbals/susp/susp_hit_metal_2_fff.wav
05  tom-low.wav      <- VSCO 1 Percussion/drums/tenor/tenor_lower/tenor_fff_1.wav
06  crash.wav        <- VSCO 1 Percussion/varMetal/Cymbals/clash/crash_hit_fff_loose.wav
07  rim.wav          <- VSCO 1 Percussion/varWood/claves_ff.wav
08  808.wav          <- VSCO 1 Percussion/drums/bass/bdrum_muted_fff_1.wav
09  percussion.wav   <- VSCO 1 Percussion/drums/other/ethnic/congo/open/ethnicLowOpen_hit_f_1.wav
10  shaker.wav       <- VSCO 1 Percussion/varWood/Camo's Shaker/shake1.wav
11  ride.wav         <- VSCO 1 Percussion/varMetal/Cymbals/susp/susp_hit_hardmall_fff.wav
12  cowbell.wav      <- VSCO 1 Percussion/varMetal/various/cowbell_fff_1.wav
13  snap.wav         <- VSCO 1 Percussion/drums/snare/drum1/snare1_nosnare_fff_1.wav
14  tom-floor.wav    <- VSCO 1 Percussion/drums/tenor/tenor_lower/tenor_fff_2.wav
15  tom-high.wav     <- VSCO 1 Percussion/drums/tenor/tenor_higher/tenorH_fff_1.wav
```

### Exact one-shot mapping: Mallet Lab

```text
00  marimba-c2.wav   <- Percussion/Marimba/Marimba_hit_Outrigger_C2_loud_01.wav
01  marimba-f1.wav   <- Percussion/Marimba/Marimba_hit_Outrigger_F1_loud_01.wav
02  marimba-g2.wav   <- Percussion/Marimba/Marimba_hit_Outrigger_G2_loud_01.wav
03  marimba-b2.wav   <- Percussion/Marimba/Marimba_hit_Outrigger_B2_loud_01.wav
04  marimba-c4.wav   <- Percussion/Marimba/Marimba_hit_Outrigger_C4_loud_01.wav
05  marimba-f3.wav   <- Percussion/Marimba/Marimba_hit_Outrigger_F3_loud_01.wav
06  marimba-g4.wav   <- Percussion/Marimba/Marimba_hit_Outrigger_G4_loud_01.wav
07  marimba-b4.wav   <- Percussion/Marimba/Marimba_hit_Outrigger_B4_loud_01.wav
08  marimba-c6.wav   <- Percussion/Marimba/Marimba_hit_Outrigger_C6_loud_01.wav
09  glock-g4.wav     <- Percussion/Glock/glock_medium_G4.wav
10  glock-c5.wav     <- Percussion/Glock/glock_medium_C5.wav
11  glock-g6.wav     <- Percussion/Glock/glock_medium_G6.wav
12  xylo-g3.wav      <- Percussion/Xylo/Xylo_Medium_G3_ff_01_far.wav
13  xylo-c4.wav      <- Percussion/Xylo/Xylo_Medium_C4_ff_01_far.wav
14  bell-c4.wav      <- Percussion/TB_hit_C4_v4_rr1.wav
15  bell-g4.wav      <- Percussion/TB_hit_G4_v4_rr1.wav
```

## TidalCycles TR-808 Fischer samples

- **Kit:** TR-808 Studio (13)
- **Recorded by:** Michael Fischer, from a real Roland TR-808.
- **Source:** <https://github.com/tidalcycles/sounds-tr808-fischer>
- **License:** CC0 1.0 Universal
  (<https://creativecommons.org/publicdomain/zero/1.0/>)
- **Character:** Classic 808 voice circuits captured from hardware: kick,
  snare, clap, rim, toms, hi-hats, cymbals, cowbell, maracas and clave.
- **Used in:** TR-808 Studio.

### Exact one-shot mapping

```text
00  kick.wav         <- bd8/BD0000.WAV
01  snare.wav        <- sd8/SD0000.WAV
02  clap.wav         <- cp8/CP.WAV
03  hat-closed.wav   <- hc8/HC00.WAV
04  hat-open.wav     <- oh8/OH00.WAV
05  tom-low.wav      <- lt8/LT00.WAV
06  crash.wav        <- cy8/CY0000.WAV
07  rim.wav          <- rs8/RS.WAV
08  808.wav          <- bd8/BD5000.WAV
09  percussion.wav   <- cl8/CL.WAV
10  shaker.wav       <- ma8/MA.WAV
11  ride.wav         <- cy8/CY5000.WAV
12  cowbell.wav      <- cb8/CB.WAV
13  snap.wav         <- sd8/SD5000.WAV
14  tom-floor.wav    <- mt8/MT00.WAV
15  tom-high.wav     <- ht8/HT00.WAV
```

## Salamander Grand Piano

- **Library:** Salamander Grand Piano
- **Recorded by:** Alexander Holm
- **License:** Creative Commons Attribution 3.0
  (<https://creativecommons.org/licenses/by/3.0/>)
- **Source:** <https://github.com/sfzinstruments/SalamanderGrandPiano>
- **Web subset used for conversion:**
  <https://github.com/akari0624/web-piano-samples>
- **Files used:** 13 roots from MIDI 33 to 105, each in a soft and a hard
  velocity layer, trimmed to five seconds. The app pitch-shifts between roots.
- **Used in:** Salamander Grand instrument, Salamander Bass preset, Neon
  Cathedral, Velvet Sky, Cypher Room and Live Room.

## Vocal test fixture

- **File:** `crates/hs-core/tests/fixtures/voice-note.wav`
- **Recording:** a single sung note by Bart Massey
- **Source:** <https://github.com/pdx-cs-sound/wavs>
- **License:** CC0 1.0 Universal
  (<https://creativecommons.org/publicdomain/zero/1.0/>)
- **Used in:** the automated vocal-to-bass render test only. It is not shipped
  inside the application binary.

## Kit loudness balance (gain only)

The drum one-shots remain exactly as converted; balance is applied as a single
gain trim per kit at the end of the Drum Rack voice. No EQ, compression,
triggering, pitch, or source WAV was changed.

Measured 2026-11-08 through the `hs-core` Drum Rack render path at 48 kHz. Each
kit used its factory preset and one 0.9-velocity hit per lane. Short-term RMS is
the maximum 50 ms window; the table reports the median of the 16 lane values.
The master bus and limiter were bypassed for the level probe so limiting could
not mask a hot sample.

| Kit | Before median (dBFS) | Trim (dB) | After median (dBFS) | Max after pre-master peak (dBFS) |
| --- | ---: | ---: | ---: | ---: |
| Acoustic Studio | -15.70 | +0.9 | -14.80 | -6.86 |
| House Club | -15.65 | +0.9 | -14.75 | -7.72 |
| Trap 808 | -14.80 | 0.0 | -14.80 | -8.57 |
| Dubstep | -14.55 | -0.3 | -14.85 | -9.53 |
| Boom Bap | -16.90 | +2.1 | -14.80 | -7.21 |
| Bounce | -14.70 | +0.1 | -14.60 | -8.31 |
| Salamander Studio | -14.14 | -0.7 | -14.84 | -8.49 |
| Salamander Hard | -13.48 | -1.3 | -14.78 | -9.50 |
| Salamander Soft | -17.71 | +2.9 | -14.81 | -5.40 |
| World Percussion | -17.52 | +2.7 | -14.82 | -5.49 |
| Orchestral Percussion | -15.73 | +0.9 | -14.83 | -7.00 |
| Mallet Lab | -12.63 | -2.2 | -14.83 | -10.01 |
| TR-808 Studio | -13.58 | -1.2 | -14.78 | -9.77 |

The 13-kit spread fell from 5.08 dB to 0.25 dB against a -14.80 dBFS target.
Across 208 lane probes covering all 155 unique files, the lowest 50 ms RMS was
-26.38 dBFS, the highest pre-master peak was 0.537 (-5.40 dBFS), and the
largest DC offset was 0.001191. The trims are stored in `samples::KIT_TRIM_DB`
and applied by `samples::drum_kit_gain`. A second pass through the default
master compressor and limiter reached a maximum peak of 0.642
(-3.85 dBFS), below the 0.92 limiter threshold, so the limiter stays unengaged.

## Conversion and verification notes

- The 112 new one-shots were converted with FFmpeg to mono 44.1 kHz signed
  16-bit PCM WAV, no longer than four seconds. Trimmed tails use a short
  de-click/fade so they do not pop.
- Each new one-shot was peak-normalized only. Targets were 0.72 for the soft
  Salamander kit, 0.95 for the hard Salamander kit, and 0.90 for the other new
  kits. No EQ, compression, reverb, pitch-shifting, layering or synthesis was
  baked into these files.
- The 13 kit mappings each have 16 lanes: kick, snare, clap, closed hat, open
  hat, low tom, crash, rim, 808, percussion, shaker, ride, cowbell, snap, floor
  tom and high tom. Lanes that are not natural kit pieces use musically related
  percussion or mallet notes and are labeled in the device view.
- Verified at build time by automated tests: 13 kit names, 155 embedded WAVs,
  13 x 16 mappings, every lane decodes, every individual one-shot renders
  finite/audible/non-clipping through the sampler and default master path
  (208 probes, 155 files), the 13 kit medians stay within 0.25 dB, and all ten
  factory demos render.
- Per-file source/destination and gain records used for the conversion are in
  the build workspace at `target/sample-src/drum-expansion.tsv` and
  `target/sample-src/normalization-report.tsv`. They are not shipped in the
  application binary.
