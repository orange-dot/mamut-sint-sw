# 2026-04-30 PC4 AG03 External Camera Solo Synth Take

## Boundary

This run tested a short live solo-synth pass on the real PC4-to-Mamut path
without desktop screen recording load.

- controller: Kurzweil `PC4`
- PC4 mode: Performance
- MIDI path: `PC4 -> mioXM DIN 1 -> Mamut EPM1`
- audio path: `Mamut EPM1 -> Yamaha AG06/AG03`
- runtime mode: standalone release windowed GUI
- recording path: external phone/camera
- MIDI trace: disabled
- initial launch patch: `razor-thaw`
- controller profile: `profiles/pc4-full.toml`
- operator goal: around 60 seconds of solo synth playing, avoiding Spectacle
  recorder stutter

Canonical command:

```bash
tools/run-pc4-ag03.sh --release --windowed razor-thaw
```

## Pre-Run Check

Before launch, no old `mamut-standalone`, `run-pc4-ag03`, `cargo run`,
`spectacle`, or `obs` process was found.

The release binary resolved `Razor Thaw` from the intended local workspace:

```text
patch: Razor Thaw (/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/patches/factory/razor-thaw.toml)
description: Baklja-ready lead with high Ruin pressure
```

The real host MIDI check found the intended input:

```text
1: mioXM:mioXM DIN 1 16:0
```

The AG06/AG03 playback route was visible as:

```text
0: hw:1,0 - AG06/AG03 - USB Audio
```

The concrete local paths and ALSA selector are preserved here as session
evidence. Portable setup should use the helper script's device detection.

## Launch Evidence

The live run was launched outside the sandbox so the host ALSA MIDI sequencer
and strict hardware audio device were available:

```text
Launching EPM1 standalone
  patch: razor-thaw
  audio: hw:1,0
  midi: mioXM DIN 1
  midi channel: 1
  controller profile: /home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/profiles/pc4-full.toml
  cargo profile: release
  midi trace: disabled
  mode: windowed
```

The process started the expected release runtime:

```text
Running `target/release/mamut-standalone play --audio-device 'hw:1,0' --midi-device 'mioXM DIN 1' --midi-channel 1 --controller-profile ... razor-thaw`
```

## Take Notes

The original plan was a single-patch `Razor Thaw` solo synth take. During the
actual run, the operator used the GUI patch selection and turned the pass into a
short live patch exploration. The runtime reported these patch loads:

```text
loaded patch: Sawyer Rezz
loaded patch: Cathedral Bloom
loaded patch: Furnace Choir
loaded patch: Glass Tide
loaded patch: Molten Horizon
loaded patch: Gravity Wake
```

No MIDI trace was enabled, by design, to avoid terminal I/O during a performance
recording. The run therefore preserves launch and patch-switch evidence rather
than raw MIDI byte evidence.

## Shutdown

The GUI process exited cleanly with code `0` after the operator finished the
take.

After shutdown, a host process check found no remaining `mamut-standalone`,
`run-pc4-ag03`, `cargo run`, `spectacle`, or `obs` process.

## Verdict

Pass:

- release windowed GUI launched on the real `PC4 -> mioXM DIN 1 -> AG06/AG03`
  path
- MIDI trace stayed off for lower recording overhead
- external phone/camera was used instead of Spectacle
- manual GUI patch switching worked across multiple factory patches
- runtime exited cleanly and left no stale process

Observed scope change:

- the session was not a strict single-patch 60-second `Razor Thaw` take; it
  became a live factory-patch exploration after launch

Follow-up:

- run a strict one-patch take later if the goal is focused musical evidence for
  one sound
- if the external recording is kept, cross-reference its filename or location in
  this note
