# 2026-04-29 PC4 AG03 Windowed GUI Session

## Boundary

This run tested the real PC4 controller path in the windowed standalone GUI:

- controller: Kurzweil `PC4`
- PC4 mode: Performance
- MIDI path: `PC4 -> mioXM DIN 1 -> Mamut EPM1`
- audio path: `Mamut EPM1 -> Yamaha AG06/AG03`
- runtime mode: standalone windowed GUI with MIDI trace
- launch patch: `molten-horizon`
- controller profile: `profiles/pc4-full.toml`

Canonical command:

```bash
tools/run-pc4-ag03.sh --windowed --trace-midi --audio-device hw:1,0 molten-horizon
```

## Preflight

Audio and MIDI device discovery found the expected hardware endpoints:

```text
0: hw:1,0 - AG06/AG03 - USB Audio
1: mioXM:mioXM DIN 1 16:0
17: AG06/AG03:AG06/AG03 MIDI 1 20:0
```

Patch dry-run passed before launching the GUI:

```bash
cargo run --locked -p mamut-standalone -- dry-run molten-horizon
```

## Startup

The GUI launched with the expected real hardware route:

```text
audio: hw:1,0
midi: mioXM DIN 1
midi channel: 1
controller profile: profiles/pc4-full.toml
midi trace: enabled
mode: windowed
```

At startup, before deliberate playing, the operator heard a single low held
tone. MIDI trace showed startup note traffic from the PC4 path; the first
representative event was:

```text
midi trace: ch=1 raw=[90 30 5C] note on note=48 velocity=0.724
```

The held tone disappeared immediately when `Panic` was pressed. That confirms
the tone was a held Mamut voice caused by incoming MIDI state, not a permanent
audio-engine drone.

## Deliberate Play Evidence

After `Panic`, deliberate playing worked through the GUI runtime. Representative
note pairs were visible in trace:

```text
midi trace: ch=1 raw=[90 41 34] note on note=65 velocity=0.409
midi trace: ch=1 raw=[80 41 2B] note off note=65
midi trace: ch=1 raw=[90 42 50] note on note=66 velocity=0.630
midi trace: ch=1 raw=[80 42 0A] note off note=66
```

Pitch bend, mod wheel, and channel aftertouch were also observed:

```text
midi trace: ch=1 raw=[E0 00 00] pitch bend semitones=-2.000
midi trace: ch=1 raw=[E0 00 40] pitch bend semitones=0.000
midi trace: ch=1 raw=[D0 71] aftertouch pressure=0.890
midi trace: ch=1 raw=[D0 00] aftertouch pressure=0.000
```

The operator also performed a control-only GUI pass with no notes. All tested
PC4 controls visually updated in the GUI. This pass did not verify the audible
effect of each control; it only verifies GUI propagation/status for the
controller surface.

## Shutdown

After the operator finished, the windowed process was stopped and verified gone:

```text
target/debug/mamut-standalone play --audio-device hw:1,0 --trace-midi ...
```

No `mamut-standalone` process remained in the host process list after shutdown.

## Verdict

Pass:

- windowed GUI starts on the strict AG03 `hw:1,0` path
- real PC4 note input reaches the GUI runtime
- deliberate notes sound and trace as expected after `Panic`
- pitch bend and channel aftertouch reach the runtime
- PC4 controls visually propagate into the GUI during a control-only pass
- `Panic` immediately clears the reproduced held-note state

Open findings:

- startup can still produce a held note in windowed mode even when the operator
  is not playing
- the all-controls GUI pass verified visible GUI response, not the audible
  effect of each mapped parameter
- a clean follow-up should start from zero, press `Panic`, then run a
  one-control-at-a-time audible sweep with no notes and with held notes

