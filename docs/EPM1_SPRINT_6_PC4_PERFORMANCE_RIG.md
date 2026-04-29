# EPM1 Sprint 6: PC4 Performance Rig

## Summary

Sprint 6 turns `EPM1` from a strong standalone synth into a live-performance
instrument optimized for:

- `PC4 -> mioXM -> EPM1 -> audio interface`
- reliable stage use
- fast patch movement
- expressive macro control
- a minimal but serious performance UI

This sprint is explicitly about **playing the instrument**, not about plugin
hosting, broad platform work, or deep editor architecture.

## Locked Profile Decisions

Sprint 6 now assumes the `PC4` policy defined in:

- `EPM1_PC4_LIVE_PROFILE.md`

That means the sprint is no longer deciding:

- slider vs knob
- macro order
- `program change` scope
- soft takeover posture
- expression pedal posture
- single-channel vs zones

Those decisions are already closed for the first live rig pass.

Current implementation baseline:

- `mamut-engine` now exposes explicit `panic` and controller-reset paths
- `mamut-standalone` now implements the locked `PC4` CC map
- `program change 0..7` now targets the live set directly
- the standalone now opens a native `egui` performance window by default when a
  graphical session is available
- `--headless` preserves the terminal runtime surface for the same rig

## Goal

At the end of Sprint 6, `EPM1` should be comfortable to play live from a
Kurzweil `PC4`, with:

- stable MIDI input through `mioXM`
- clean patch switching and emergency recovery
- strong velocity / aftertouch / sustain response
- favorite and setlist navigation
- a dedicated performance UI
- a factory bank tuned specifically for `PC4`

## Out Of Scope

- `CLAP`
- shared editor architecture
- patch schema `v2`
- broad Linux audio platform implementation
- non-performance feature expansion
- major DSP redesign unrelated to live play feel

## Core Outcome

`mamut-standalone` should become a credible `PC4` live rig:

- start ready to play
- show live state clearly
- expose the five public macros clearly
- support stage-safe patch changes
- survive ordinary controller and device churn without leaving the instrument in
  a broken state

## User Story

The primary Sprint 6 user story is:

> A player powers on `PC4`, sends MIDI through `mioXM`, launches `EPM1`, picks
> a live patch quickly, shapes it with aftertouch / mod wheel / mapped
> controls, changes favorites during play, and recovers instantly if notes or
> controllers get stuck.

## Scope

### 1. Engine performance behavior

- refine velocity response specifically against `PC4`
- refine channel aftertouch response so it is musically useful in held phrases
- refine pitch bend reset and controller reset behavior around patch changes
- add explicit `panic` behavior:
  - all notes off
  - sustain clear
  - pitch bend reset
  - live macro/controller reset where appropriate
- keep sustain and voice stealing musically acceptable under dense chord play

### 2. Engine performance control surface

- add explicit runtime-facing engine actions for:
  - `panic`
  - controller reset
  - clean patch switch
  - favorite/setlist patch load
- extend the read-only runtime snapshot only as far as needed for live UI:
  - current patch
  - active voices
  - held notes or equivalent live state
  - peak / clipping signal
  - macro values
  - standalone-side MIDI activity pulse or comparable signal

### 3. PC4 profile

- implement the locked full `PC4 Live Profile` in `profiles/pc4-full.toml`
- map `K1..K9`, `S1..S9`, and `SW1..SW9` from the real PC4 capture
- keep `mod wheel`, sustain, pitch bend, and channel aftertouch as standard
  performance lanes
- support `program change 0..7` for favorite / setlist selection
- keep reserved controls trace-visible until the engine has the matching block
- do not add expression pedal in the first Sprint 6 pass

### 4. Performance UI

- add one small dedicated performance surface for standalone use
- do not start full editor work
- implementation choice is now locked:
  - native `egui` / `eframe`
  - single-screen performance window
  - `--headless` fallback for terminal-only sessions
- the UI must expose:
  - current patch name
  - favorite / setlist context
  - five live macro controls or indicators
  - MIDI activity indicator
  - peak / clipping indicator
  - audio device status
  - MIDI device status
  - `panic`
- the UI should optimize for fast live use, not parameter depth

### 5. Runtime ergonomics

- start directly into a playable state
- allow startup patch selection
- keep audio and MIDI device selection explicit and understandable
- support next / previous favorite quickly
- support favorite or setlist selection by `program change`
- keep reconnect behavior clear when MIDI or audio devices disappear

### 6. Factory bank tuning for PC4

- tune the shipped bank directly against `PC4`
- lock a live-ready subset of `6-8` patches
- required live roles:
  - opener
  - pad
  - bass
  - lead
  - expressive aftertouch patch
  - edge / rupture patch
- retune each live patch for:
  - velocity feel
  - aftertouch usefulness
  - sustain behavior
  - macro travel while notes are held

### 7. Stage safety

- `panic` must always be reachable
- patch switching must not leave stuck notes behind
- the UI must clearly show missing MIDI input
- the UI must clearly show clipping or transport trouble if it happens
- existing realtime discipline from the `rtrb` pass must be preserved

## Backlog

### A. `mamut-engine`

- implement or tighten a runtime-facing `panic` path
- add controller reset handling
- verify clean patch reset across:
  - notes
  - sustain
  - bend
  - aftertouch
  - live macros
- retune `PC4` velocity and aftertouch response
- extend `EngineSnapshot` for live UI needs only

### B. `mamut-standalone`

- add the locked `PC4 Live Profile`
- add `program change` handling for favorites or setlist
- add runtime `panic`
- add performance-first patch/favorite controls
- add the live UI surface
- keep CLI support, but make the performance UI the primary live surface

### C. Factory bank

- identify the final Sprint 6 live bank
- lock the 8-slot live set order for performance navigation
- tune the bank on actual `PC4`
- update listening criteria with `PC4`-specific pass/fail notes

### D. Docs

- document the `PC4` control map
- document startup and stage workflow
- document favorite / setlist / `program change` behavior
- document the locked `PC4` live profile policy
- document emergency actions:
  - `panic`
  - controller reset
  - reconnect expectations

### E. Review gates

- `sel4-rust-single-file-reviewer`
- `sel4-rust-execution-optimizer`
- `sel4-rust-systems-reviewer`
- `sel4-integrated-systems-reviewer`

## Acceptance Criteria

- `PC4` can play `EPM1` through `mioXM` in a stable live session
- startup enters a clean playable state
- `panic` reliably clears stuck notes and controllers
- patch switching is musically acceptable for live use
- `program change` selects favorites or setlist entries predictably
- sustain, pitch bend, mod wheel, and aftertouch all feel intentional
- the live UI clearly shows patch, macros, MIDI activity, and peak state
- the live-ready patch bank is tuned on the real controller path
- automated tests still pass
- host smoke with real audio + real MIDI still passes
- review gates close with no high or medium correctness findings

## Definition Of Done

Sprint 6 is done only when:

- the live rig is proven on actual `PC4 -> mioXM -> EPM1`
- the runtime UI is good enough to use without terminal-only workflow
- favorite / setlist movement is fast enough for actual play
- stuck-note recovery is explicit and reliable
- the live bank is tuned and documented
- review gates are executed and findings addressed

## Recommended Execution Order

1. `panic` and controller reset
2. patch-switch cleanup and stage-safe runtime behavior
3. `PC4` control map and `program change`
4. performance UI skeleton
5. favorite / setlist flow
6. factory bank tuning on real hardware
7. review gates and final polish

## Open Questions

Only follow-on tuning questions remain:

- Does real play testing require alternate PC4 user-template labels beyond
  `profiles/pc4-full.toml`?
- How much extra patch metadata should the performance window show live without
  turning into an editor?
- Does real-stage testing justify later soft takeover or expression-pedal work?

## Resume Note

This document is the canonical Sprint 6 planning and acceptance anchor for the
current `EPM1` live-rig phase after the local `rtrb` transport pass. It should
now be updated only when live testing or review findings force a real policy
change.
