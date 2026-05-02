# GFM v1.4 MIDI Performance Controls Evidence

Date: 2026-05-01

Superseded note: `GFM v1.5` changes the live control semantics from
always-audible `GFM Amount` to a momentary aftertouch gate. This v1.4 document
is retained as evidence for the first MIDI-control wiring slice.

This slice makes the enabled GFM layer playable from MIDI without changing patch
schema, factory patches, ALSA, or the standalone runtime launch contract.

## Control Contract

- Channel aftertouch remains the existing expressive pressure source and now also
  feeds bounded live excitation into the active GFM field voice.
- The live pressure contribution is additive and conservative:
  - pressure: `+ aftertouch * 0.22`
  - heat: `+ aftertouch * 0.10`
  - rupture bias: `+ aftertouch * 0.16`
- `GFM Amount` is a new realtime controller event:
  `ControllerEvent::GfmLayerAmount { amount }`.
- `GFM Amount` defaults to `1.0`, preserving the v1.3 enabled-layer sound until
  a MIDI control moves it.
- `GFM Amount = 0.0` mutes the layer mix and preserves active GFM voice state
  without applying the GFM layer soft-clip branch.
- `EngineSnapshot.gfm_layer` exposes `amount` and `pressure` for GUI/status
  display.

## PC4 Mapping

- `profiles/pc4-full.toml`
  - `K8 GFM Amount`
  - CC `3`
  - kind `gfm_layer_amount`
- Channel aftertouch is not profile-specific; it continues to parse from MIDI
  status `0xD0`.

This deliberately avoids seed/program control from a continuous knob. Seed and
program changes remain discrete layer configuration because they rebuild the GFM
voice.

## GUI Contract

- The GFM panel now includes a `MIDI` metric tile:
  - primary value: GFM amount
  - secondary value: current aftertouch pressure as `AT <value>`
- The v1.3 lightweight GUI enable/apply path remains unchanged.

## Boundary Evidence

- No patch schema changes.
- No factory patch changes.
- No ALSA tuning, device selection, buffering, or recovery behavior changes.
- No MIDI runtime transport restructuring.
- `mamut-field` GFM primitive behavior is unchanged for zero live pressure:
  `GfmFieldVoice::next_sample()` still uses the accepted performance gesture.

## Validation

Commands run:

- `cargo fmt --all`
- `cargo fmt --all --check`
- `cargo test -p mamut-engine gfm_field_voice_live_pressure_changes_excitation_and_stays_bounded -- --nocapture`
- `cargo test -p mamut-engine gfm_layer_amount_zero_mutes_layer_without_disabling_voice -- --nocapture`
- `cargo test -p mamut-standalone pc4 -- --nocapture`
- `cargo test -p mamut-standalone last_control_event_tracks_profile_gfm_amount_and_program_change -- --nocapture`
- `cargo test -p mamut-standalone`
- `cargo test -p mamut-engine`
- `cargo test -p mamut-field`
- `cargo build --release -p mamut-standalone`
- `cargo run -p mamut-standalone -- dry-run --gfm-layer-seed 0x6A46_4D40 cathedral-bloom`
- `rg -n "gfm_layer_amount|GfmLayerAmount|K8 GFM Amount|aftertouch|ChannelAftertouch" crates/mamut-patch patches profiles`
- `rg -n "gfm_layer_amount|GfmLayerAmount|K8 GFM Amount" crates/mamut-engine crates/mamut-standalone profiles`
- `rg -n "gfm_layer_amount|GfmLayerAmount|K8 GFM Amount" crates/mamut-patch patches`

Results:

- `cargo test -p mamut-engine gfm_field_voice_live_pressure_changes_excitation_and_stays_bounded -- --nocapture`
  - 1 passed
- `cargo test -p mamut-engine gfm_layer_amount_zero_mutes_layer_without_disabling_voice -- --nocapture`
  - 1 passed
- `cargo test -p mamut-standalone pc4 -- --nocapture`
  - 8 passed
- `cargo test -p mamut-standalone last_control_event_tracks_profile_gfm_amount_and_program_change -- --nocapture`
  - 1 passed
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
- `rg -n "gfm_layer_amount|GfmLayerAmount|K8 GFM Amount" crates/mamut-patch patches`
  - no matches
- `rg -n "gfm_layer_amount|GfmLayerAmount|K8 GFM Amount" crates/mamut-engine crates/mamut-standalone profiles`
  - matches are limited to the new engine event/control state, standalone
    profile parsing/display/tests, and `profiles/pc4-full.toml`.
- The broader aftertouch search shows the pre-existing patch
  `performance_response.aftertouch_*` fields plus the new K8 profile mapping;
  it does not add a patch schema or factory patch field.

## Listening And Playability Verdict

Manual MIDI playability verdict is pending. The intended first check is:

- enable GFM in the GUI
- hold a note/chord
- sweep `K8`
- press channel aftertouch
- confirm the GFM layer moves continuously without runtime rebuild or patch
  reload
