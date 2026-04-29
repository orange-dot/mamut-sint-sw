# 2026-04-29 PC4 AG03 Note-Off Fix Validated

## Boundary

This run validated the repeated-pitch note-off fix on the real hardware path:

- controller: Kurzweil `PC4`
- MIDI path: `PC4 -> mioXM DIN 1 -> Mamut EPM1`
- audio path: `Mamut EPM1 -> Yamaha AG06/AG03`
- runtime mode: standalone headless with MIDI trace
- patch: `molten-horizon`

Canonical command:

```bash
tools/run-pc4-ag03.sh --audio-device hw:1,0 --trace-midi molten-horizon
```

## Cold Idle

Startup was clean:

```text
voices: active=0 sustain=false held=[] peak=0.030 clip=false
midi activity: messages=0
transport: queued=0 target=512 write_hint=256 underrun_batches=2 underrun_frames=512 xrun_recoveries=0 overflow_batches=0 overflow_frames=0
```

After a 10-second idle check:

```text
voices: active=0 sustain=false held=[] peak=0.038 clip=false
midi activity: messages=0
```

No startup program-change, note spam, or controller spam was observed.

## One-Note Routing Probe

The first routing probe produced clean note-on/note-off pairs:

```text
midi trace: ch=1 raw=[90 3C 5C] note on note=60 velocity=0.724
midi trace: ch=1 raw=[80 3C 39] note off note=60
midi trace: ch=1 raw=[90 48 68] note on note=72 velocity=0.819
midi trace: ch=1 raw=[80 48 2B] note off note=72
```

Status after release:

```text
voices: active=0 sustain=false held=[] peak=0.038 clip=false
midi activity: messages=4
```

## Repeated-Note Regression

Repeated note test produced balanced note-on/note-off pairs for notes `60` and
`72`.

Status five seconds after the repeated-note pattern:

```text
voices: active=0 sustain=false held=[] peak=0.039 clip=false
midi activity: messages=20
transport: queued=512 target=512 write_hint=256 underrun_batches=2 underrun_frames=512 xrun_recoveries=0 overflow_batches=0 overflow_frames=0
```

This validates the engine-side fix for the previously observed stuck held voice.

## Musical Smoke

The operator then played a short musical pass with chords, channel aftertouch,
and mod wheel movement.

Observed accepted controls:

- notes and chords on channel 1
- channel aftertouch from low pressure to `1.000`
- mod wheel from `0.000` to `1.000` and back

Immediate status after the last phrase still showed one release tail:

```text
voices: active=1 sustain=false held=[69] peak=0.058 clip=false
midi activity: messages=499
transport: queued=512 target=512 write_hint=256 underrun_batches=2 underrun_frames=512 xrun_recoveries=0 overflow_batches=0 overflow_frames=0
```

Six seconds later, the release tail had cleared:

```text
voices: active=0 sustain=false held=[] peak=0.038 clip=false
midi activity: messages=499
transport: queued=512 target=512 write_hint=256 underrun_batches=2 underrun_frames=512 xrun_recoveries=0 overflow_batches=0 overflow_frames=0
```

## Verdict

Pass:

- PC4 Global MIDI cleanup gave a clean cold idle
- one-note route probe passed
- repeated-pitch note-off regression passed on hardware
- musical notes, chords, aftertouch, and mod wheel worked end to end
- no stuck note reproduced after the engine fix
- transport counters did not grow beyond startup underruns

Still not covered in this run:

- sustain pedal
- PC4 K/S/SW full map sweep
- program-change live slot sweep

Next run can move from note-off validation to full controller-map validation.
