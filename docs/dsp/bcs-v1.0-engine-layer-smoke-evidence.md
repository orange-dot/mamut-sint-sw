# BCS v1.0 Engine Layer Smoke Evidence

Date: 2026-05-01

This is the first `mamut-engine` integration smoke for the offline
`Bifurcation-Coordinate Synthesis` Hopf/Duffing playground. It proves that a BCS
voice can live behind an explicit engine-owned layer switch without changing the
default synth, patch schema, standalone UI, MIDI map, or factory patches.

Foundation evidence:

- `bcs-v0.1-hopf-duffing-playground-evidence.md`

The earlier BCS foundation notes are local research inputs and are intentionally
ignored by git. This file records the accepted engine-layer evidence surface.

## Engine Contract

- Public mode: `mamut_engine::BcsLayerMode`
- Public snapshot: `mamut_engine::BcsLayerSnapshot`
- Public engine methods:
  - `Engine::set_bcs_layer_mode`
  - `Engine::bcs_layer_mode`
- Default: `BcsLayerMode::Disabled`
- Enabled source: `mamut_field::bcs::BcsVoice`
- Enabled scenario selector: `mamut_field::bcs::BcsScenario`
- Sample rate: rebuilt from `EngineConfig::sample_rate_hz`
- Render placement: after the GFM layer and before stereo crossfeed/output trim
- Mix policy: conservative mono layer gain with dry-activity gate
- Safety policy: if the BCS voice reports unsafe state, or emits a non-finite
  sample, the layer returns dry audio for that frame and exposes the unsafe
  counters through the snapshot

## Tests Added

- `bcs_layer_defaults_to_disabled_snapshot`
- `bcs_layer_disabled_matches_baseline_engine_render`
- `bcs_layer_enabled_stable_anchor_changes_render_but_stays_bounded`
- `bcs_layer_standard_scenarios_stay_finite_and_bounded`
- `bcs_layer_rebuilds_when_patch_loads_and_resets_voice_state`

## Validation

Commands:

```bash
cargo fmt --all
cargo test -p mamut-engine bcs_layer -- --nocapture
cargo test -p mamut-field bcs -- --nocapture
cargo fmt --all --check
cargo test -p mamut-engine gfm_layer_ -- --nocapture
cargo test -p mamut-standalone
cargo build --release -p mamut-standalone
cargo test -p mamut-engine
rg -n "Bcs|bcs|Bifurcation|Hopf|Duffing" crates/mamut-standalone crates/mamut-patch patches profiles
```

Observed result:

```text
BCS layer focused tests: 5 passed; 0 failed
mamut-field BCS tests: 8 passed; 0 failed
GFM layer regression tests: 15 passed; 0 failed
mamut-standalone tests: 43 passed; 0 failed
mamut-standalone release build: finished
mamut-engine full tests: 46 passed; 0 failed
Boundary search: no matches in standalone, patch crate, patches, or profiles
```

## Boundary

This slice intentionally changes only `mamut-engine` and documentation.

No changes were made to:

- patch schema
- factory patches
- profiles
- standalone launch flags
- GUI controls
- MIDI mapping
- ALSA/audio device handling

The layer is not yet a performable BCS instrument. It is an engine smoke hook
for bounded, explicit, disabled-by-default experimentation.

Listening verdict: pending.
