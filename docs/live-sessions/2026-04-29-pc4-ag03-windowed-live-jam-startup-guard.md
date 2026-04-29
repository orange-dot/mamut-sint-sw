# 2026-04-29 PC4 AG03 Windowed Live Jam After Startup Guard

## Boundary

This run tested the real PC4-to-Mamut windowed GUI path after adding the MIDI
startup guard that suppresses initial PC4 entry messages before they can reach
the engine.

- controller: Kurzweil `PC4`
- PC4 mode: Performance
- MIDI path: `PC4 -> mioXM DIN 1 -> Mamut EPM1`
- audio path: `Mamut EPM1 -> Yamaha AG06/AG03`
- runtime mode: standalone windowed GUI with MIDI trace
- launch patch: `molten-horizon`
- controller profile: `profiles/pc4-full.toml`
- operator goal: live jam/video capture, not a strict one-control-at-a-time lab
  pass

Canonical command:

```bash
tools/run-pc4-ag03.sh --windowed --trace-midi --audio-device hw:1,0 molten-horizon
```

## Pre-Run Code Validation

The startup-guard patch had already passed the local non-hardware validation:

```text
cargo fmt --check
cargo test --locked -p mamut-standalone
cargo test --locked
cargo run --locked -p mamut-standalone -- dry-run molten-horizon
```

Relevant behavior added before this run:

- `MIDI_STARTUP_GUARD = 700 ms`
- accepted realtime/runtime MIDI messages inside that window are traced as
  `startup suppressed`
- suppressed messages do not enter `midi_input_queue` or
  `runtime_control_queue`

## Startup Evidence

Startup used the expected hardware route:

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

The PC4 still emitted startup note and aftertouch traffic, but the new guard
suppressed those messages:

```text
midi trace: ch=1 raw=[90 3C 5B] startup suppressed note on note=60 velocity=0.717
midi trace: ch=1 raw=[D0 33] startup suppressed aftertouch pressure=0.402
midi trace: ch=1 raw=[80 25 29] startup suppressed note off note=37
midi trace: ch=1 raw=[90 25 71] startup suppressed note on note=37 velocity=0.890
midi trace: ch=1 raw=[90 31 7B] startup suppressed note on note=49 velocity=0.969
midi trace: ch=1 raw=[80 31 2B] startup suppressed note off note=49
```

No `Panic` intervention was requested during this run. The previous startup
held-note failure was not reported by the operator.

## Live Jam Evidence

After the startup guard window, normal note traffic reached the runtime:

```text
midi trace: ch=1 raw=[90 35 34] note on note=53 velocity=0.409
midi trace: ch=1 raw=[80 35 20] note off note=53
midi trace: ch=1 raw=[90 48 5C] note on note=72 velocity=0.724
midi trace: ch=1 raw=[90 4E 5B] note on note=78 velocity=0.717
midi trace: ch=1 raw=[80 48 2E] note off note=72
midi trace: ch=1 raw=[80 4E 1E] note off note=78
```

Live patch switching happened during the jam and remained usable:

```text
loaded patch: Cathedral Bloom
loaded patch: Ember Vault
loaded patch: Razor Thaw
loaded patch: Gravity Wake
loaded patch: Furnace Choir
loaded patch: Granite Plain
loaded patch: Molten Horizon
```

After some patch switches, the first note events were again marked
`startup suppressed`, which is expected because the runtime/MIDI connection is
rebuilt around patch changes. The guard protected those transition windows.

## Controller Evidence

Pitch bend was exercised across a wide range and returned to center:

```text
midi trace: ch=1 raw=[E0 00 00] pitch bend semitones=-2.000
midi trace: ch=1 raw=[E0 00 40] pitch bend semitones=0.000
midi trace: ch=1 raw=[E0 7F 7F] pitch bend semitones=2.000
```

Channel aftertouch reached the full range and returned to zero:

```text
midi trace: ch=1 raw=[D0 7F] aftertouch pressure=1.000
midi trace: ch=1 raw=[D0 00] aftertouch pressure=0.000
```

Sustain was explicitly observed:

```text
midi trace: ch=1 raw=[B0 40 7F] sustain down=true
midi trace: ch=1 raw=[B0 40 00] sustain down=false
```

Knobs were observed through the PC4 profile:

```text
midi trace: ch=1 raw=[B0 47 7F] profile K1 Filter 1 macro Gravitacija value=1.000 -> macro Gravitacija value=1.000
midi trace: ch=1 raw=[B0 49 55] profile K2 Filter 2 macro Bloom value=0.669 -> macro Bloom value=0.669
midi trace: ch=1 raw=[B0 48 7F] profile K3 Attack direct param Amp Attack value=1.000 -> direct param Amp Attack value=10000.001
midi trace: ch=1 raw=[B0 4F 65] profile K4 Release direct param Amp Release value=0.795 -> direct param Amp Release value=1517.408
midi trace: ch=1 raw=[B0 0E 7F] profile K5 Motion macro Swarm value=1.000 -> macro Swarm value=1.000
midi trace: ch=1 raw=[B0 11 7F] profile K6 FX 1 macro Heat value=1.000 -> macro Heat value=1.000
midi trace: ch=1 raw=[B0 12 28] profile K7 FX 2 macro Ruin value=0.315 -> macro Ruin value=0.315
```

Sliders were observed through the PC4 profile:

```text
midi trace: ch=1 raw=[B0 0C 71] profile S1 direct param Osc1 Saw Level value=0.890 -> direct param Osc1 Saw Level value=0.890
midi trace: ch=1 raw=[B0 0D 51] profile S2 direct param Osc1 Pulse Level value=0.638 -> direct param Osc1 Pulse Level value=0.638
midi trace: ch=1 raw=[B0 16 34] profile S3 direct param Osc2 Saw Level value=0.409 -> direct param Osc2 Saw Level value=0.409
midi trace: ch=1 raw=[B0 17 2A] profile S4 direct param Sub Level value=0.331 -> direct param Sub Level value=0.331
midi trace: ch=1 raw=[B0 18 35] profile S5 direct param Pre-Filter Drive value=0.417 -> direct param Pre-Filter Drive value=0.417
midi trace: ch=1 raw=[B0 19 05] profile S6 direct param Body Mix value=0.039 -> direct param Body Mix value=0.039
midi trace: ch=1 raw=[B0 1A 0B] profile S7 direct param Filter Cutoff value=0.087 -> direct param Filter Cutoff value=36.381
midi trace: ch=1 raw=[B0 1B 43] profile S8 direct param Chorus Mix value=0.528 -> direct param Chorus Mix value=0.528
midi trace: ch=1 raw=[B0 1C 03] profile S9 direct param Final Stage Body Drive value=0.024 -> direct param Final Stage Body Drive value=0.024
```

At least one switch release was observed:

```text
midi trace: ch=1 raw=[B0 59 00] profile SW8 favorite slot=4 release ignored value=0.000
```

This is consistent with momentary runtime controls where release values do not
trigger the action. The broader jam did include patch/favorite movement, but
this was not a controlled switch-by-switch pass.

## Operator Result

The operator closed the GUI window after the jam. The process exited cleanly
with code `0`, and a host process check showed no remaining
`mamut-standalone`, `run-pc4-ag03`, or `cargo run` process.

Operator note:

```text
odlicno, zatvorio sam, zapisi ovaj test u detalje sve.
zadovoljan sam muzickim utiskom. ima dosta za peglanje, ali prvi utisak je dobar
```

Interpretation:

- first musical impression is positive
- the startup held-note fix behaved correctly in the observed run
- PC4 expressive controls and much of the full controller surface propagated
  through Mamut
- there is still polish work before this is a finished performance instrument

## Verdict

Pass:

- windowed GUI starts on the strict AG03 `hw:1,0` route
- PC4 startup note/aftertouch traffic is visible and suppressed by the new guard
- no manual `Panic` was needed for the startup-held-note issue
- live notes, pitch bend, sustain, and channel aftertouch worked during the jam
- live patch changes stayed usable
- knobs and sliders propagated through the PC4 profile into Mamut parameters
- GUI shutdown was clean
- operator musical impression was good

Open polish:

- next run should save the raw trace to a file because terminal output is too
  large for complete manual capture
- run a controlled switch-by-switch pass for SW1..SW9, including press and
  release behavior
- decide whether patch switches should preserve more live state or intentionally
  reset around the guarded transition
- tune GUI readability for video capture: PC4 tab, debug state, and last-control
  highlights need to be easy to read on screen
- review whether `700 ms` is the right guard duration after a few more real PC4
  starts and patch switches
