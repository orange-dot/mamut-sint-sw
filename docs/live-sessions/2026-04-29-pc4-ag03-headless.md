# 2026-04-29 PC4 AG03 Headless Session

## Boundary

This run tested the real one-way live path:

- controller: Kurzweil `PC4`
- MIDI path: `PC4 -> mioXM DIN 1 -> Mamut EPM1`
- audio path: `Mamut EPM1 -> Yamaha AG06/AG03`
- runtime mode: standalone headless with MIDI trace
- patch: `molten-horizon`

Canonical run command:

```bash
tools/run-pc4-ag03.sh --audio-device hw:1,0 --trace-midi molten-horizon
```

## Preflight

Observed local devices:

- MIDI input: `mioXM:mioXM DIN 1 16:0`
- audio output: `hw:1,0 - AG06/AG03 - USB Audio`

Preflight checks completed during the same bench session:

- `cargo test --locked` passed before the live run
- `cargo run --locked -p mamut-standalone -- list-midi` showed `mioXM DIN 1`
- `cargo run --locked -p mamut-standalone -- list-audio` showed `hw:1,0`
- `cargo run --locked -p mamut-standalone -- dry-run molten-horizon` passed

## Startup Blocker

The first real launch failed before MIDI/audio runtime startup:

```text
error: failed to set ALSA sample format: ALSA function 'snd_pcm_hw_params_set_format' failed with error 'Invalid argument (22)'
```

Diagnosis:

- `/proc/asound/card1/stream0` showed AG06/AG03 playback supports `S32_LE`
- Mamut standalone had been opening direct ALSA `hw:` playback as float-only

Local fix applied in `crates/mamut-standalone/src/main.rs`:

- prefer ALSA `F32` when the device supports it
- fall back to `S32_LE` for strict USB `hw:` devices such as this AG03
- convert the engine's interleaved `f32` output to signed 32-bit PCM in the
  playback thread

Post-fix validation:

```text
cargo check --locked -p mamut-standalone
cargo test --locked -p mamut-standalone
```

Both passed. The standalone test count after the fix was `30 passed`.

## Live Evidence

After the fallback fix, the same command started successfully:

```text
play patch: Molten Horizon
audio: hw:1,0 (AG06/AG03, USB Audio) @ 44100 Hz, 2 channels
alsa: selector=hw:1,0 period=256 buffer=1024 start_threshold=1024
mode: connected (mioXM:mioXM DIN 1 16:0)
controller profile: pc4-full
midi trace: enabled
```

Incoming PC4 notes were observed on MIDI channel 1, for example:

```text
midi trace: ch=1 raw=[90 41 6A] note on note=65 velocity=0.835
midi trace: ch=1 raw=[90 3E 43] note on note=62 velocity=0.528
midi trace: ch=1 raw=[80 3E 36] note off note=62
```

First status during play:

```text
voices: active=0 sustain=false held=[] peak=0.033 clip=false
midi activity: messages=65
transport: queued=512 target=512 write_hint=256 underrun_batches=2 underrun_frames=512 xrun_recoveries=0 overflow_batches=0 overflow_frames=0
```

Emergency clear was verified:

```text
panic
voices: active=0 sustain=false held=[] peak=0.000 clip=false
```

The user confirmed audible synth output and short real play:

```text
sad sam malo i svirao, fino radi
```

## Idle / Unexpected Note Observation

During one idle check, new note events were still observed while the operator was
not intending to play at that moment:

```text
midi trace: ch=1 raw=[90 30 51] note on note=48 velocity=0.638
midi trace: ch=1 raw=[80 30 33] note off note=48
midi trace: ch=1 raw=[90 33 56] note on note=51 velocity=0.677
midi trace: ch=1 raw=[80 33 1F] note off note=51
midi trace: ch=1 raw=[90 35 60] note on note=53 velocity=0.756
```

Interpretation for this run:

- Mamut was not inventing controller state; `--trace-midi` showed real incoming
  note messages from the PC4 path
- `panic` cleared Mamut's held state
- the exact PC4-side source was not isolated in this session
- the operator later confirmed some playing also happened during the session

## Verdict

Pass for controlled hardware smoke:

- AG03 direct `hw:1,0` playback works after `S32_LE` fallback
- `PC4 -> mioXM DIN 1 -> Mamut` live note input works
- headless trace and `panic` are usable during a real session
- the `pc4-full` profile loads on the real path

Not yet stage-stable:

- underrun counters were non-zero
- knobs/sliders/switches/program change were not exhaustively stepped in this
  run
- the next run should start from a cold idle baseline and separate PC4-side
  arpeggiator/latch behavior from deliberate playing

## Next Cold-Start Protocol

Next session should start from zero:

1. power/reset PC4, mioXM path, and AG03 path
2. launch with `tools/run-pc4-ag03.sh --audio-device hw:1,0 --trace-midi molten-horizon`
3. before playing, run `status`, wait roughly 10 seconds, then run `status`
   again and confirm `midi activity` is stable
4. play a named sequence: single notes, chord stabs, sustain, mod wheel,
   aftertouch, one knob, one slider, one switch, and program change `0..2`
5. run `panic`
6. record final `status`, especially `midi activity`, `underrun_batches`,
   `underrun_frames`, `xrun_recoveries`, and `overflow_*`
