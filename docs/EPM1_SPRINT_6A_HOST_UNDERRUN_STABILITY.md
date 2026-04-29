# EPM1 Sprint 6A: Host Underrun and Performance Stability Pass

## Summary

This work item is the immediate follow-up after the first successful Sprint 6
live-rig implementation.

Current truth:

- `PC4 -> mioXM -> EPM1 -> AG06/AG03` is working
- the `PC4 Live Profile` is implemented
- `panic` works on the real controller path
- the performance window and `--headless` runtime both work
- real host smoke has already confirmed startup, MIDI attach, and live note flow

Current blocker:

- the host run showed **high underrun counts** during live use
- the synth is therefore functionally correct, but not yet ready to be trusted
  as a stage-stable live rig

This sprint exists to close that gap.

## Goal

Reduce or eliminate host underruns during ordinary `PC4` live play, while
preserving:

- the `rtrb` audio handoff
- the locked `PC4` live profile
- the current patch switching behavior
- `panic`
- the `AG03` helper/startup path

## Scope

### In scope

- host-side underrun diagnosis
- queue and refill behavior
- engine-thread cadence under real host load
- UI/control-side interference with the live runtime
- `PC4`-driven live session stability
- `AG03` real output path behavior
- short iterative tuning passes with real hardware in the loop

### Out of scope

- new synthesis features
- `CLAP`
- shared editor work
- Linux platform extraction
- patch schema changes
- large new UX features unrelated to underrun/stability

## Working Assumption

The next useful progress will **not** come from sandbox-only work.

This item requires a mixed workflow:

- most code changes and tests happen locally/offline
- but important decisions are validated on the real host with:
  - `PC4`
  - `mioXM DIN 1`
  - `AG06/AG03`

In other words:

- local work is necessary
- host testing is mandatory

## Hypothesis Areas

The underrun issue may be coming from one or more of these buckets:

### 1. Refill cadence / queue policy

- queue target too small for the real host/device path
- engine thread wakes too conservatively
- refill policy too bursty
- mismatch between render block size and ALSA write demand

### 2. Host scheduler / device behavior

- direct ALSA `hw:` playback behavior on this machine
- `AG03` via strict `hw:` path only, without PipeWire/JACK/default fallback
- ALSA write cadence mismatch at `44100 Hz`
- thread wake timing under normal desktop load

### 3. Control-plane interference

- snapshot cadence still too frequent
- UI polling cadence still too expensive under load
- patch/control activity producing more engine-thread churn than expected
- runtime status/telemetry work still too noisy

### 4. Engine cost under real play

- real `PC4` note patterns more expensive than the earlier demo path
- held-note / chord density causing producer-side starvation
- control bursts under mod wheel / aftertouch / macro movement

## Implementation Plan

### 1. Reproduce and measure

Before changing behavior, capture reproducible host observations for:

- `--headless --demo`
- `--headless` with real `PC4`
- performance window with real `PC4`
- `molten-horizon`
- one heavier live patch such as `furnace-choir` or `gravity-wake`

Track:

- underrun batch count
- underrun frame count
- xrun recovery count
- overflow count
- audible clicks/glitches
- whether the issue appears mostly:
  - idle
  - sustained notes
  - repeated chords
  - macro motion
  - aftertouch/mod wheel movement
  - UI-visible mode only

### 2. Separate headless from UI impact

Run the same live pattern in:

- `--headless`
- default performance window mode

Goal:

- determine whether the UI path materially worsens underruns
- determine whether snapshot cadence is still significant on the host

### 3. Audit queue and producer behavior

Inspect and, if needed, retune:

- `AUDIO_QUEUE_CAPACITY_BLOCKS`
- `AUDIO_QUEUE_TARGET_BLOCKS`
- engine render block size
- refill wake cadence
- producer “fill until target” behavior

Possible outputs of this pass:

- larger queue target
- different block size
- different wake/sleep balance
- different refill threshold

### 4. Audit transport/status telemetry cost

Check whether current transport/status counters or snapshot behavior are still
too noisy during live use.

Focus:

- transport metrics reads/writes
- snapshot polling rate
- performance-window repaint cadence
- any remaining avoidable control-path churn during ordinary play

### 5. Preserve emergency and live semantics

While tuning performance, keep verifying:

- `panic`
- program change live-slot jumps
- next/prev live slot movement
- patch switch hard reset
- `PC4` macro mapping

No performance fix should silently regress live safety.

## Development Workflow

This sprint should use a short-cycle rhythm.

### Offline loop

Use offline/local work for:

- code changes
- unit tests
- dry-run checks
- `cargo test`
- `seL4` review passes
- reasoning about queue policy and runtime structure

### Host loop

Use real host testing for:

- audio startup on `AG03`
- real MIDI attach on `mioXM DIN 1`
- underrun reproduction
- glitch listening
- `PC4` feel
- validating whether a change helped or hurt

### Expected cycle

1. form a concrete hypothesis
2. make one bounded code change
3. run local tests
4. run a short real host session
5. observe underrun counters and audible behavior
6. either keep the change or revise the hypothesis

This sprint should avoid long “batches” of unverified changes.

## Host Test Matrix

### Baseline host scenarios

- `PC4 -> mioXM DIN 1 -> EPM1 -> AG03`
- helper script path:
  - `tools/run-pc4-ag03.sh`
- patches:
  - `molten-horizon`
  - `gravity-wake`
  - `furnace-choir`

### Performance actions to test

- single notes
- repeated chord stabs
- sustain down under chord pressure
- aftertouch on held notes
- mod wheel movement
- knob macro movement if mapped on `PC4`
- patch switch between live slots
- `panic`

### Runtime variants to compare

- `--headless`
- `egui` performance window
- default audio route
- `AG03` helper route

## Deliverables

- one documented diagnosis of the main underrun source or sources
- one or more bounded runtime fixes
- updated runtime constants/policies if needed
- updated docs if the recommended host startup path changes
- preserved live-safety behavior across `panic`, patch switch, and live slot
  selection

## Acceptance Criteria

- underrun counts are significantly reduced in ordinary `PC4` live use
  or eliminated entirely
- no obvious glitching during a short real live session
- `panic` still clears everything immediately
- `program change 0..7` still works
- live slot switching remains musically acceptable
- `cargo test` still passes
- `seL4` hot-path and subsystem review do not report new high/medium regressions

## Definition Of Done

This item is done only when:

- the real host path is re-tested with the actual controller and audio interface
- the underrun problem is either fixed or reduced enough to stop being the
  dominant live blocker
- the chosen fix is captured in code and docs
- the result is stable enough that the next sprint can return to musical/live
  refinement instead of transport firefighting

## Review Gates

Run these during implementation, not only at the end:

- `sel4-rust-single-file-reviewer`
  - `crates/mamut-standalone/src/main.rs`
  - `crates/mamut-engine/src/lib.rs` if touched
- `sel4-rust-execution-optimizer`
  - callback path
  - engine-thread producer loop
  - queue policy
  - UI/snapshot interaction if touched
- `sel4-rust-systems-reviewer`
  - `mamut-engine` + `mamut-standalone`
- `sel4-integrated-systems-reviewer`
  - code + docs + live host workflow if startup guidance changes

## Resume Note

When work resumes, start with:

1. reproduce the current underrun behavior on the host
2. identify whether `--headless` and `egui` differ materially
3. pick one bounded hypothesis and one bounded code change
4. validate on the real `PC4` + `AG03` path before taking the next step
