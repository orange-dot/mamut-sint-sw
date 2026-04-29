# 2026-04-29 PC4 AG03 Headless Performance Mode Session

## Boundary

This run repeated the real headless hardware path after setting the Kurzweil
`PC4` to Performance mode:

- controller: Kurzweil `PC4`
- PC4 mode: Performance
- MIDI path: `PC4 -> mioXM DIN 1 -> Mamut EPM1`
- audio path: `Mamut EPM1 -> Yamaha AG06/AG03`
- runtime mode: standalone headless with MIDI trace
- launch patch: `molten-horizon`

Canonical run command:

```bash
tools/run-pc4-ag03.sh --audio-device hw:1,0 --trace-midi molten-horizon
```

## Startup

Startup succeeded on the strict AG03 `hw:1,0` path:

```text
play patch: Molten Horizon
audio: hw:1,0 (AG06/AG03, USB Audio) @ 44100 Hz, 2 channels
mode: connected (mioXM:mioXM DIN 1 16:0)
controller profile: pc4-full
midi trace: enabled
```

## Cold Idle / PC4 Entry Events

This was not a clean zero-event idle at startup. The PC4 emitted:

- one channel-1 note-on
- several init CC messages across channels
- channel-1 `CC7`, `CC10`, `CC0`, and `CC32`
- two channel-1 program changes to slot `1`

Representative trace:

```text
midi trace: ch=1 raw=[90 3C 4E] note on note=60 velocity=0.614
midi trace: ch=1 raw=[B0 07 7F] ignored cc=7 value=127
midi trace: ch=1 raw=[B0 0A 40] ignored cc=10 value=64
midi trace: ch=1 raw=[B0 00 00] ignored cc=0 value=0
midi trace: ch=1 raw=[B0 20 00] ignored cc=32 value=0
midi trace: ch=1 raw=[C0 01] program change slot=1
```

The program change was accepted and switched Mamut to `Cathedral Bloom`.

After `panic`, a 7-second idle check was stable:

```text
voices: active=0 sustain=false held=[] peak=0.019 clip=false
midi activity: messages=19
transport: queued=512 target=512 write_hint=256 underrun_batches=2 underrun_frames=512 xrun_recoveries=0 overflow_batches=0 overflow_frames=0
```

The session was then returned to slot `0 / Molten Horizon` with `favorite 0`.

## Deliberate Play Evidence

Deliberate play produced accepted note, channel aftertouch, and mod wheel input
on MIDI channel 1.

Representative note trace:

```text
midi trace: ch=1 raw=[90 48 32] note on note=72 velocity=0.394
midi trace: ch=1 raw=[80 48 29] note off note=72
midi trace: ch=1 raw=[90 4C 49] note on note=76 velocity=0.575
midi trace: ch=1 raw=[80 4C 36] note off note=76
```

Representative aftertouch trace:

```text
midi trace: ch=1 raw=[D0 0C] aftertouch pressure=0.094
midi trace: ch=1 raw=[D0 50] aftertouch pressure=0.630
midi trace: ch=1 raw=[D0 7F] aftertouch pressure=1.000
midi trace: ch=1 raw=[D0 00] aftertouch pressure=0.000
```

Representative mod wheel trace:

```text
midi trace: ch=1 raw=[B0 01 01] mod wheel amount=0.008
midi trace: ch=1 raw=[B0 01 40] mod wheel amount=0.504
midi trace: ch=1 raw=[B0 01 7F] mod wheel amount=1.000
midi trace: ch=1 raw=[B0 01 00] mod wheel amount=0.000
```

Post-play status showed audio and transport stayed healthy:

```text
voices: active=3 sustain=false held=[67, 67, 69] peak=0.648 clip=false
midi activity: messages=319
transport: queued=512 target=512 write_hint=256 underrun_batches=2 underrun_frames=512 xrun_recoveries=0 overflow_batches=0 overflow_frames=0
```

Three seconds later, the status still showed held notes:

```text
voices: active=2 sustain=false held=[67, 69] peak=0.610 clip=false
midi activity: messages=319
```

`panic` cleared the state immediately:

```text
voices: active=0 sustain=false held=[] peak=0.000 clip=false
midi activity: messages=319
```

## Verdict

Pass:

- AG03 strict `hw:1,0` playback remained usable
- PC4 Performance mode produced accepted channel-1 notes
- channel aftertouch and mod wheel are working end to end
- transport counters did not keep climbing during deliberate play
- `panic` cleared stuck held state immediately

Open findings:

- Performance mode still emits startup entry events, including program change
  slot `1`; this can switch Mamut away from the requested launch patch
- deliberate play produced stuck held notes `67` and `69` with sustain reported
  as `false`
- sustain, knob, slider, switch, and program-change sweeps were not completed in
  this run

## Next Work

Before the next full controller-map pass:

1. inspect PC4 Performance/Multi entry behavior and disable automatic bank/program
   send if possible
2. reproduce the stuck-held state with a minimal two-note sequence and trace
   every note-on/note-off pair
3. decide whether Mamut should defensively collapse duplicate held note entries
   for the same pitch, or whether this is strictly a PC4-side setup issue
4. after stuck-note behavior is understood, run the full one-by-one
   sustain/knob/slider/switch/program-change map pass
