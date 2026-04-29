# 2026-04-29 PC4 AG03 Windowed GUI Redesign Live Take

## Boundary

This run tested the real PC4-to-Mamut windowed GUI path immediately after the
EPM-style GUI redesign.

- controller: Kurzweil `PC4`
- PC4 mode: Performance
- MIDI path: `PC4 -> mioXM DIN 1 -> Mamut EPM1`
- audio path: `Mamut EPM1 -> Yamaha AG06/AG03`
- runtime mode: standalone windowed GUI with MIDI trace
- launch patch: `molten-horizon`
- controller profile: `profiles/pc4-full.toml`
- operator goal: live performance/video pass with the redesigned GUI visible

Canonical command:

```bash
tools/run-pc4-ag03.sh --windowed --trace-midi --audio-device hw:<card>,<device> molten-horizon
```

## Pre-Run Validation

Before the live run, the GUI redesign patch passed:

```text
cargo fmt --check
cargo check --locked -p mamut-standalone
cargo test --locked -p mamut-standalone
```

The standalone test pass was `32 passed; 0 failed`.

## First Launch

The first redesigned GUI launch used the expected hardware route:

```text
Launching EPM1 standalone
  patch: molten-horizon
  audio: hw:1,0
  midi: mioXM DIN 1
  midi channel: 1
  controller profile: profiles/pc4-full.toml
  midi trace: enabled
  mode: windowed
```

The concrete ALSA selector is preserved in the launch transcript as session
evidence. Portable setup should use `list-audio` or the helper script's device
detection instead of assuming the same card/device number.

Startup guard behavior was visible:

```text
midi trace: ch=1 raw=[B0 11 42] startup suppressed profile K6 FX 1 macro Heat value=0.520 -> macro Heat value=0.520
midi trace: ch=1 raw=[90 45 30] startup suppressed note on note=69 velocity=0.378
midi trace: ch=1 raw=[80 45 16] startup suppressed note off note=69
```

After the guard window, normal notes reached the runtime:

```text
midi trace: ch=1 raw=[90 3C 48] note on note=60 velocity=0.567
midi trace: ch=1 raw=[80 3C 33] note off note=60
```

Operator result for the first launch:

```text
ok je svirka, ali gui redesign nesto zapleo
```

Visual finding: the redesigned shell loaded, but the body layout collapsed into
a mostly blank window with a tall left logo strip and tab buttons floating near
the center-right. This was treated as a GUI layout regression, not an audio/MIDI
runtime failure.

## Layout Fix

The GUI tab bar was changed from a free `horizontal_centered` layout with a
right-to-left tab group into a fixed-height top bar:

- top bar height fixed to `54 px`
- logo fixed to `34 x 40`
- title block fixed to `190 x 42`
- tabs rendered left-to-right with fixed `76 x 38` buttons
- body scroll area remains below the top bar

After the fix, validation again passed:

```text
cargo fmt --check
cargo check --locked -p mamut-standalone
cargo test --locked -p mamut-standalone
```

The standalone test pass remained `32 passed; 0 failed`.

## Relaunch Evidence

The fixed GUI was relaunched with the same command and exited cleanly with code
`0` after the operator closed the window.

The runtime continued to receive dense real PC4 note traffic with note-off pairs:

```text
midi trace: ch=1 raw=[90 2F 43] note on note=47 velocity=0.528
midi trace: ch=1 raw=[80 2F 06] note off note=47
midi trace: ch=1 raw=[90 31 3E] note on note=49 velocity=0.488
midi trace: ch=1 raw=[80 31 0F] note off note=49
```

Mod wheel, pitch bend, and sustain were exercised:

```text
midi trace: ch=1 raw=[B0 01 7F] mod wheel amount=1.000
midi trace: ch=1 raw=[E0 7F 7F] pitch bend semitones=2.000
midi trace: ch=1 raw=[E0 00 00] pitch bend semitones=-2.000
midi trace: ch=1 raw=[B0 40 7F] sustain down=true
midi trace: ch=1 raw=[B0 40 00] sustain down=false
```

At least one mapped profile control was observed during the live take:

```text
midi trace: ch=1 raw=[B0 09 20] profile K9 Reverb direct param Reverb Mix value=0.252 -> direct param Reverb Mix value=0.252
```

After shutdown, a host process check found no remaining `mamut-standalone`,
`run-pc4-ag03`, or `cargo run` process.

## Verdict

Pass:

- real PC4 notes, note-offs, mod wheel, pitch bend, sustain, and a mapped knob
  reached the windowed runtime
- the audio/MIDI path remained usable while the GUI redesign was under test
- startup guard continued to suppress initial PC4 startup traffic
- the first GUI layout regression was reproduced, fixed, rebuilt, and retested
- the fixed run exited cleanly with no lingering process

Open polish:

- capture a screenshot or video frame of the fixed GUI in the next run to make
  the visual acceptance explicit
- reduce trace volume for performance-video takes by teeing raw MIDI to a file
  instead of relying on terminal scrollback
- continue tightening the EPM-style layout for PC4 and Debug tabs after the
  fixed top bar is visually accepted
