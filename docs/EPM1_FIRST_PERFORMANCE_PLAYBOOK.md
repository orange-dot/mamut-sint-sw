# EPM1 First Performance Playbook

This playbook is the shortest trustworthy path from repo checkout to a first
real `PC4` live session.

It assumes the locked Sprint 6 rig:

- `PC4 -> mioXM DIN 1 -> EPM1 -> AG03/AG06`
- standalone runtime only
- `PC4 Live Profile` only
- factory live set only

## Current Readiness

What is already true:

- workspace tests pass locally
- the standalone runtime, live set, `panic`, and `PC4` profile are implemented
- real-host smoke already proved startup, MIDI attach, and live note flow
- first documented real headless session:
  `docs/live-sessions/2026-04-29-pc4-ag03-headless.md`

What is not yet closed:

- host underrun counts are still the main open blocker for stage trust

So the correct posture today is:

- functionally playable
- good for controlled live testing
- not yet automatically “stage stable” without a short underrun check on the
  target host

## 1. Preflight

From repo root:

```bash
cargo test
cargo run -p mamut-standalone -- list-audio
cargo run -p mamut-standalone -- list-midi
```

Confirm before launch:

- `PC4` is sending through `mioXM DIN 1`
- `list-audio` shows the Yamaha output as an ALSA `hw:<card>,<device>` path
- the target patch exists in `patches/factory/`

Recommended first patch:

- `molten-horizon`

## 2. First Host Launch

Start with the narrowest, lowest-ambiguity path first.

Preferred first command:

```bash
tools/run-pc4-ag03.sh molten-horizon
```

That helper currently does five important things:

- selects the standalone runtime
- starts in `--headless` mode
- resolves the AG06/AG03 ALSA `hw:` playback path before launch
- targets `mioXM DIN 1`
- filters to MIDI channel `1` by default
- can enable raw MIDI tracing with `--trace-midi` for routing diagnosis
- forwards direct ALSA period/buffer/start-threshold overrides when provided

If the helper path is not suitable on the current machine, use the explicit
runtime command:

```bash
cargo run -p mamut-standalone -- \
  play \
  --headless \
  --audio-device hw:<card>,<device> \
  --midi-device "mioXM DIN 1" \
  molten-horizon
```

Use the `egui` performance window only after the headless pass is clean enough
to trust.

## 3. First Five-Minute Smoke Pass

Immediately after startup, run:

```text
status
```

Then play this minimum pass from the `PC4`:

1. single notes across low, mid, and high range
2. repeated chord stabs
3. sustain pedal down under chord pressure
4. mod wheel movement on held notes
5. aftertouch on held notes
6. `program change 0..7`
7. `panic`

What must be true:

- sound starts immediately
- MIDI activity rises
- live slot jumps follow `program change 0..7`
- `panic` clears notes and controller residue
- no obvious clicks appear during ordinary phrase length

## 4. Watch The Right Counters

The most important live command during this phase is still:

```text
status
```

Watch these transport fields:

- `underrun_batches`
- `underrun_frames`
- `xrun_recoveries`
- `overflow_batches`
- `overflow_frames`
- `queued`
- `target`
- `write_hint`

Also watch these MIDI ingress fields:

- `accepted`
- `midi_dropped`
- `runtime_dropped`
- `trace_dropped`
- `controllers_coalesced`

Interpretation:

- small non-growing counts after startup are less worrying than counters that
  keep climbing during ordinary play
- audible clicks plus rising underruns means the session is not ready to trust
- overflow growth points to producer-side pressure or queue mismatch
- `midi_dropped`, `runtime_dropped`, and `trace_dropped` should stay at `0`
  during a normal no-recording/no-trace 96 kHz arpeggio pass
- `controllers_coalesced` may rise during dense pitch bend, mod wheel,
  aftertouch, macro, or layer amount movement; it should not imply lost note
  or sustain events

Recommended 96 kHz ingress hardening acceptance command:

```bash
tools/run-pc4-ag03.sh molten-horizon \
  --alsa-period-frames 512 \
  --alsa-buffer-frames 2048 \
  --alsa-start-threshold-frames 2048
```

Run at least five minutes of PC4 arpeggio without recording first. Add
recording only after the no-recording pass stays clean.

## 5. First Safe Patch Rotation

Use the locked live set only:

- `0` -> `molten-horizon`
- `1` -> `cathedral-bloom`
- `2` -> `ember-vault`
- `3` -> `razor-thaw`
- `4` -> `gravity-wake`
- `5` -> `furnace-choir`
- `6` -> `granite-plain`
- `7` -> `glass-tide`

Recommended first rotation:

1. `molten-horizon`
2. `ember-vault`
3. `gravity-wake`
4. `furnace-choir`

That sequence covers:

- opener mass
- bass anchor
- expressive aftertouch path
- denser poly pressure

## 6. Emergency Actions

Keep these commands in reach:

```text
panic
reset-controllers
favorite 0
next
prev
audio
midi
demo
quit
```

Use them like this:

- `panic` if notes, sustain, or controller state feels wrong
- `reset-controllers` if pitch/mod/aftertouch state feels stuck
- `favorite 0` to return to the known opener patch quickly
- `audio` and `midi` to inspect or switch devices during diagnosis
- `demo` if controller input disappears but audio path still needs checking

## 7. Decision Rule For The First Real Performance

Treat the session as acceptable only if all of the following hold on the target
host:

- startup is repeatable
- `PC4` attach is repeatable
- `program change 0..7` is predictable
- `panic` is immediate
- ordinary play does not produce sustained audible glitching
- underrun counters stay low enough that they do not keep rising under normal
  phrases

If those are not true, the repo is still in live-rig stabilization mode rather
than true stage-ready mode.

## 8. Recommended Work Order After The First Session

If the first session shows trouble, keep the follow-up order short:

1. repeat the same test in `--headless`
2. compare with the default windowed path
3. retest `molten-horizon`, then `gravity-wake`, then `furnace-choir`
4. record whether the issue appears on idle, chord pressure, sustain, macro
   motion, or aftertouch
5. only then tune queue/runtime behavior

This keeps performance work aligned with the current Sprint 6A underrun pass
instead of reopening transport architecture prematurely.
