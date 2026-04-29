# 2026-04-29 PC4 AG03 Clean Idle And Note-Off Fix Session

## Boundary

This run followed the PC4 Global MIDI cleanup:

- `Program Change = Off`
- clock/transport settings kept out of the Mamut path
- intended route: `PC4 -> mioXM DIN 1 -> Mamut EPM1 -> AG06/AG03`
- runtime mode: standalone headless with MIDI trace
- launch patch: `molten-horizon`

Canonical command:

```bash
tools/run-pc4-ag03.sh --audio-device hw:1,0 --trace-midi molten-horizon
```

## Pre-Fix Reproduction In This Session

Before the code fix, the run reproduced the stuck-note behavior with clean
note-on/note-off pairs:

```text
midi trace: ch=1 raw=[90 3C 5B] note on note=60 velocity=0.717
midi trace: ch=1 raw=[80 3C 2A] note off note=60
```

Repeated same-pitch notes then left active voices:

```text
voices: active=4 sustain=false held=[72, 60, 72, 60] peak=0.657 clip=false
midi activity: messages=48
```

After six seconds, the voices were still active despite `sustain=false`:

```text
voices: active=3 sustain=false held=[72, 60, 60] peak=0.654 clip=false
```

`panic` cleared the state immediately:

```text
voices: active=0 sustain=false held=[] peak=0.000 clip=false
```

## Code Fix

Root cause found in `crates/mamut-engine/src/lib.rs`:

- `note_off` selected the oldest active matching voice
- already-released voices were eligible
- with repeated same-pitch notes during release, note-off could re-target the
  old released voice and leave the newer held voice stuck

Fix:

- `note_off` now selects only matching voices in `VoicePhase::Held`
- regression test added:
  `note_off_prefers_held_voice_for_repeated_pitch`

Validation:

```text
cargo fmt --check
cargo test --locked -p mamut-engine
cargo test --locked -p mamut-standalone
```

Results:

- `mamut-engine`: `13 passed`
- `mamut-standalone`: `30 passed`

## Post-Fix Hardware Start

After rebuilding, the same headless command started cleanly:

```text
play patch: Molten Horizon
audio: hw:1,0 (AG06/AG03, USB Audio) @ 44100 Hz, 2 channels
mode: connected (mioXM:mioXM DIN 1 16:0)
midi activity: messages=0
voices: active=0 sustain=false held=[] peak=0.030 clip=false
```

Ten-second idle check stayed clean:

```text
voices: active=0 sustain=false held=[] peak=0.038 clip=false
midi activity: messages=0
transport: queued=256 target=512 write_hint=256 underrun_batches=2 underrun_frames=512 xrun_recoveries=0 overflow_batches=0 overflow_frames=0
```

This confirms the PC4 Global MIDI cleanup stopped the previous startup
program-change and note spam.

## Open Routing Finding

After the clean idle, no new key or controller messages arrived at
`mioXM DIN 1` during the attempted play window:

```text
midi activity: messages=0
voices: active=0 sustain=false held=[]
```

Interpretation:

- Mamut and AG03 were up and stable
- the unwanted startup MIDI was gone
- the PC4 was not transmitting keys/controllers into the listened DIN route
  during this attempt

Before the next live pass, confirm the PC4 Global `Destination` is `MIDI` or
`USB+MIDI`, not USB-only, and confirm the current PC4 page/mode is actually
transmitting key events to the DIN path.

## Verdict

Pass:

- startup MIDI spam removed
- no automatic program-change slot jump
- note-off stuck-voice bug identified and fixed in code
- post-fix idle baseline is clean

Blocked:

- post-fix hardware note-off validation could not be completed because no new
  key events reached `mioXM DIN 1`

Next run should begin with a one-note routing probe before any musical test:

```text
expected: one note-on and one note-off on ch=1
expected status after release: voices=0, sustain=false, held=[]
```
