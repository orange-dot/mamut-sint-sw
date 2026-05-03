# BCS v1.2 Playable MIDI Layer Evidence

Date: 2026-05-01

This slice turns the existing BCS standalone listening path into a playable
performance layer. Scenario selection is still explicit runtime configuration,
while actual audible entry is controlled from the PC4.

## Runtime Surface

BCS scenario selection remains available through:

```bash
mamut-standalone play --bcs-layer-scenario <scenario> [factory-name-or-path]
```

and during playback:

```text
bcs <off|stable-anchor|edge-sweep|subharmonic-pressure|recovery-return>
```

The GUI now has a `BCS LAYER` panel with:

- runtime enable/disable for the BCS scenario mode
- direct scenario buttons
- `SW9` / `S9` MIDI gain status
- note/frequency status
- boundedness and unsafe-state status

## PC4 Controls

`profiles/pc4-full.toml` now maps:

- `S9` / CC `28` -> `bcs_layer_gain`
- `SW9` / CC `90` -> `bcs_layer_enabled`

Audible BCS mix target is nonzero only when all of these are true:

- BCS scenario mode is enabled
- `SW9` reports enabled
- `S9` gain is above the BCS control deadzone
- at least one note is physically held

The layer follows the lowest held note. Releasing the final held note targets
the BCS mix back to silence, even if `SW9` and `S9` remain up. Pitch bend is
included when the BCS voice is rebuilt for the current held note.

## Engine Semantics

`mamut-engine` now exposes two realtime controller events:

- `ControllerEvent::BcsLayerEnabled { enabled }`
- `ControllerEvent::BcsLayerAmount { amount }` (`bcs_layer_gain` in the PC4 profile maps here)

`BcsLayerSnapshot` now reports:

- mode and active scenario
- PC4 playable enable
- raw amount
- smoothed effective amount
- current BCS pitch note/frequency
- sample rate and safety counters

The BCS voice can advance while silent, but it is mixed only through the
smoothed effective amount. Enabling a scenario by itself is therefore a ready
state, not an audible state.

Follow-up tuning after first traced playback:

- BCS note changes retune the active Hopf/Duffing voice without resetting
  nonlinear state.
- BCS mix attack is `45 ms`, fast enough for short played notes.
- Engine BCS layer gain is `0.24`, still bounded by the focused engine tests.

## Validation

Commands:

```bash
cargo fmt --all
cargo fmt --all --check
cargo check -p mamut-engine
cargo check -p mamut-standalone
cargo test -p mamut-engine bcs_layer -- --nocapture
cargo test -p mamut-standalone bcs -- --nocapture
cargo test -p mamut-standalone
cargo test -p mamut-engine gfm_layer -- --nocapture
cargo build --release -p mamut-standalone
target/release/mamut-standalone dry-run --bcs-layer-scenario subharmonic-pressure cathedral-bloom
rg -n "bcs_layer|BcsLayer|BCS" crates/mamut-patch patches
```

Observed focused results:

```text
Engine BCS focused tests: 9 passed; 0 failed
Field BCS focused tests: 8 passed; 0 failed
Standalone BCS focused tests: 3 passed; 0 failed
mamut-standalone full tests: 46 passed; 0 failed
mamut-standalone release build: finished
GFM layer regression tests: 15 passed; 0 failed
Patch/schema boundary search: no matches in crates/mamut-patch or patches
```

Observed release dry-run BCS line:

```text
bcs: mode=enabled scenario=subharmonic-pressure active=subharmonic-pressure playable=off knob=0.000 gain=0.000 effective_gain=0.000 pitch=- sample_rate=48000 max_state=0.026979 unsafe_events=0 unsafe=false
```

## Boundary

Changed:

- `mamut-engine` BCS controller events, smoothing, note tracking, and snapshot
- `mamut-standalone` BCS GUI panel, MIDI profile parser, status line, and PC4
  display
- `profiles/pc4-full.toml` S9/SW9 mapping

Unchanged:

- patch schema
- factory patches
- GFM K8 + aftertouch behavior
- MIDI program-change behavior
- ALSA transport architecture
- `mamut-field` BCS model

Listening verdict: pending real playback through the standalone rig.
