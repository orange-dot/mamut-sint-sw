# GFM v1.3 GUI Runtime Toggle Evidence

Date: 2026-05-01

This slice keeps the existing engine-owned GFM layer and standalone launch flag,
but changes the windowed GUI apply path so enabling, disabling, or changing the
GFM seed sends a lightweight command to the running engine thread. It no longer
rebuilds the full audio runtime for this GUI action.

Default standalone behavior remains `GfmLayerMode::Disabled`.

## GUI Contract

- The windowed GUI GFM panel keeps the existing enable checkbox and seed field.
- `APPLY SEED` resolves the UI text into `Option<u64>` using the same decimal
  and `0x`/`0X` hexadecimal parser as the standalone CLI.
- Enabled mode sends `GfmLayerMode::Enabled { seed }` to the engine thread.
- Disabled mode sends `GfmLayerMode::Disabled` to the engine thread.
- The command returns `GfmVoiceProgramSelection` through a reply channel.
- Re-applying the already active seed reads the current snapshot selection
  instead of rebuilding the GFM voice.
- The header status is intentionally short and stable: `READY`, `OFF`, or
  `WAITING`. Detailed selected/active program and score data remain in the
  existing metric tiles.

## Runtime Contract

- New standalone-internal command:
  `EngineCommand::SetGfmLayerMode(GfmLayerMode, reply)`.
- The command is handled on the engine thread by calling
  `Engine::set_gfm_layer_mode(mode)`.
- Patch switching, audio-device switching, ALSA rollback, and other full runtime
  rebuild paths still call `build_audio_runtime` and preserve the current
  `gfm_layer_seed`.
- The CLI/headless `--gfm-layer-seed` path from v1.2 is unchanged.

## Boundary Evidence

- No patch schema changes.
- No factory patch changes.
- No profile or MIDI map changes.
- No ALSA tuning, device selection, buffering, or recovery behavior changes.
- No `mamut-field` or `mamut-engine` DSP algorithm changes.
- This is a standalone GUI/control-path slice only.

## Validation

Commands run:

- `cargo fmt --all`
- `cargo fmt --all --check`
- `cargo test -p mamut-standalone gfm_layer -- --nocapture`
- `cargo run -p mamut-standalone -- dry-run cathedral-bloom`
- `cargo run -p mamut-standalone -- dry-run --gfm-layer-seed 0x6A46_4D40 cathedral-bloom`
- `cargo test -p mamut-standalone`
- `cargo test -p mamut-engine`
- `cargo test -p mamut-field`
- `rg -n "rebuild_with_gfm_seed" crates/mamut-standalone/src/main.rs`
- `rg -n "GfmLayerMode|SetGfmLayerMode|set_gfm_layer_seed|gfm_layer_seed|gfm-layer" crates/mamut-patch patches profiles`
- `rg -n "Gfm|gfm|GFM" crates/mamut-patch patches profiles`

Results:

- `cargo test -p mamut-standalone gfm_layer -- --nocapture`
  - 7 passed
- Disabled dry-run reported:
  - `gfm: mode=disabled`
- Enabled dry-run reported:
  - `gfm: mode=enabled seed=0x6A464D40 selected=HorizontPerformance active=HorizontPerformance scores=(0.8994,0.1359,0.0106) ruptures=0`
- `cargo test -p mamut-standalone`
  - 43 passed
- `cargo test -p mamut-engine`
  - 32 passed
- `cargo test -p mamut-field`
  - 26 passed
- `rg -n "rebuild_with_gfm_seed" crates/mamut-standalone/src/main.rs`
  - no matches
- `rg -n "GfmLayerMode|SetGfmLayerMode|set_gfm_layer_seed|gfm_layer_seed|gfm-layer" crates/mamut-patch patches profiles`
  - no matches
- `rg -n "Gfm|gfm|GFM" crates/mamut-patch patches profiles`
  - no matches

## Listening And Visual Verdict

Manual windowed GUI confirmation is still pending. The code path is now covered
by unit tests and dry-run evidence, but visual flicker/no-flicker should be
confirmed in the real GUI session before calling the v1.3 interaction fully
accepted.
