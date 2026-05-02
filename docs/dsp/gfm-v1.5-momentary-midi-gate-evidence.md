# GFM v1.5 Momentary MIDI Gate Evidence

Date: 2026-05-01

This slice changes the GFM MIDI performance behavior from an always-audible
parallel layer into an armed momentary layer. Enabling GFM in the GUI now means
the GFM voice is built and ready, but the layer is heard only while channel
aftertouch opens the live gate.

## Control Contract

- GUI GFM enable/seed still arms or disarms the engine-owned GFM layer.
- Channel aftertouch is the momentary gate.
- `K8 GFM Amount` / CC `3` remains the maximum depth control.
- `K8 GFM Amount` defaults to `0.0`, and arming GFM clears stored GFM amount,
  so a newly armed GFM layer is silent even if the physical controller has not
  yet sent its current value.
- Effective audible amount is:
  - `GFM Amount * aftertouch_gate`
- With aftertouch at `0.0`, the GFM voice keeps advancing silently and does not
  enter the mix.
- With aftertouch above zero, the same aftertouch value both opens the audible
  gate and adds bounded live pressure/heat/rupture-bias excitation.
- `GFM Amount = 0.0` still hard-mutes the GFM layer even under aftertouch.
- Arming GFM clears the stored GFM aftertouch gate to prevent stale channel
  pressure from opening the layer immediately. It does not clear global
  aftertouch expression used by the base synth macros.

This keeps seed/program changes out of continuous MIDI controls. Seed still
belongs to layer configuration because it rebuilds the GFM voice.

## GUI Contract

- The GFM panel `MIDI` tile now shows effective audible amount as the primary
  value.
- The tile detail shows the raw controls as `K8 <amount> AT <pressure>`.
- A GUI-enabled GFM layer can show `READY` while the MIDI tile reads `0.00`;
  that is the intended armed-but-not-open state.

## Boundary Evidence

- No patch schema changes.
- No factory patch changes.
- No ALSA tuning, device selection, buffering, or recovery behavior changes.
- No MIDI runtime transport restructuring.
- `mamut-field` remains unchanged.

## Validation

Commands run:

- `cargo fmt --all`
- `cargo fmt --all --check`
- `cargo test -p mamut-engine gfm_layer_changes_enabled_patch_when_momentary_pressure_is_present -- --nocapture`
- `cargo test -p mamut-engine gfm_layer_enabled_without_momentary_controls_matches_baseline_engine_render -- --nocapture`
- `cargo test -p mamut-engine gfm_layer_enable_clears_stale_momentary_controls_without_clearing_aftertouch_macros -- --nocapture`
- `cargo test -p mamut-engine gfm_layer_amount_zero_mutes_layer_without_disabling_voice -- --nocapture`
- `cargo test -p mamut-standalone pc4 -- --nocapture`
- `cargo test -p mamut-standalone`
- `cargo test -p mamut-engine`
- `cargo test -p mamut-field`
- `cargo build --release -p mamut-standalone`
- `cargo run -p mamut-standalone -- dry-run --gfm-layer-seed 0x6A46_4D40 cathedral-bloom`
- `rg -n "gfm_layer_amount|GfmLayerAmount|effective_amount|K8 GFM Amount" crates/mamut-patch patches`
- `rg -n "gfm_layer_amount|GfmLayerAmount|effective_amount|K8 GFM Amount" crates/mamut-engine crates/mamut-standalone profiles`

Results:

- `cargo test -p mamut-engine gfm_layer_changes_enabled_patch_when_momentary_pressure_is_present -- --nocapture`
  - 1 passed
- `cargo test -p mamut-engine gfm_layer_enabled_without_momentary_controls_matches_baseline_engine_render -- --nocapture`
  - 1 passed
- `cargo test -p mamut-engine gfm_layer_enable_clears_stale_momentary_controls_without_clearing_aftertouch_macros -- --nocapture`
  - 1 passed
- `cargo test -p mamut-engine gfm_layer_amount_zero_mutes_layer_without_disabling_voice -- --nocapture`
  - 1 passed
- `cargo test -p mamut-standalone pc4 -- --nocapture`
  - 8 passed
- `cargo test -p mamut-standalone`
  - 43 passed
- `cargo test -p mamut-engine`
  - 34 passed
- `cargo test -p mamut-field`
  - 26 passed
- `cargo build --release -p mamut-standalone`
  - passed
- Enabled dry-run still reports:
  - `gfm: mode=enabled seed=0x6A464D40 selected=HorizontPerformance active=HorizontPerformance scores=(0.8994,0.1359,0.0106) ruptures=0`
- `rg -n "gfm_layer_amount|GfmLayerAmount|effective_amount|K8 GFM Amount" crates/mamut-patch patches`
  - no matches
- `rg -n "gfm_layer_amount|GfmLayerAmount|effective_amount|K8 GFM Amount" crates/mamut-engine crates/mamut-standalone profiles`
  - matches are limited to engine control/snapshot/momentary mix, standalone
    profile parsing/display/tests, and `profiles/pc4-full.toml`.

## Listening And Playability Verdict

Manual MIDI playability verdict is pending. Expected behavior:

- enable GFM in the GUI
- hold a note/chord
- with no aftertouch, GFM is silent
- sweep `K8` to set maximum depth
- press and release channel aftertouch to momentarily bring GFM in and out
