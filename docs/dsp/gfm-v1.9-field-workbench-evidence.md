# GFM v1.9 Field Workbench Evidence

Status note, 2026-05-22: this document describes an experimental surface that
is not the current production GFM contract. The current implemented contract is
tracked in `docs/dsp/gfm-performance-control-contract.md`.

This slice turns the GFM layer into an inspectable and patch-controllable field
workbench instead of a fixed hidden color layer.

## Implemented Surface

- `mamut-field` now exposes bounded field diagnostics for energy accounting,
  coupling profile state, probe position, trajectory position, and a fixed
  16x16 terrain snapshot.
- GFM coupling remains sparse and bounded around the existing K=7 neighbor
  topology. The new controls shape the weights; they do not allocate or rebuild
  topology in steady state.
- `mamut-patch` now has an optional `[engine.gfm]` table with defaults for
  coupling profile, local/long coupling, asymmetry, energy budget,
  dissipation, trajectory shape/rate/depth, and probe X/Y.
- `mamut-params` exposes the same GFM fields as direct parameters
  `GfmCouplingProfile..GfmProbeY`.
- `mamut-engine` snapshots include GFM controls and terrain when an active GFM
  voice exists. Direct parameter edits update both exported patch state and the
  live GFM voice.
- The standalone Engine tab gets a `GFM Field` workbench page with terrain,
  selected coupling paths, probe marker, trajectory marker, energy tiles, and
  patch-backed controls.

## Runtime Discipline

- No heap allocation, logging, formatting, or blocking work was added to the
  audio sample step.
- Energy budget correction is a bounded lattice pass only when the field is
  over budget.
- Coupling parameter changes do not dirty topology. Topology rebuilds remain
  tied to actual graph geometry inputs.
- GUI terrain rendering is snapshot-side only and outside the audio callback.

## Validation

- `cargo fmt --all`
- `cargo check --locked -p mamut-field -p mamut-patch -p mamut-params -p mamut-engine -p mamut-runtime -p mamut-standalone`
- `cargo clippy --locked --all-targets -p mamut-field -p mamut-patch -p mamut-params -p mamut-engine -p mamut-runtime -p mamut-standalone -- -D warnings`
- `cargo test --locked -p mamut-params -p mamut-patch -p mamut-field`
- `cargo test --locked -p mamut-runtime -p mamut-standalone`
- `cargo test --locked -p mamut-engine gfm_field_voice_matches_direct_field_pcm -- --nocapture`
- `cargo test --locked -p mamut-engine gfm_field_voice_mono_block_matches_sample_step_and_direct_field -- --nocapture`
- `cargo test --locked -p mamut-engine gfm_direct_params_update_snapshot_exported_patch_and_live_voice`
- `cargo test --locked -p mamut-field terrain_snapshot_tracks_probe_trajectory_and_sparse_coupling`

## Review Gates

The repo-local review helper scripts were run for this slice:

- `tools/review/run-rust-file-review.sh crates/mamut-field/src/lattice.rs`
- `tools/review/run-rust-subsystem-review.sh crates/mamut-field crates/mamut-engine crates/mamut-patch crates/mamut-runtime crates/mamut-standalone`
- `tools/review/run-rust-hotpath-review.sh crates/mamut-field/src/lattice.rs crates/mamut-engine/src/gfm_layer.rs crates/mamut-engine/src/engine/direct_params.rs`
- `tools/review/run-integrated-review.sh crates/mamut-field crates/mamut-engine crates/mamut-patch crates/mamut-runtime crates/mamut-standalone docs/dsp docs/review-gates.md`

Self-review finding closed during the gate: GFM coupling controls were removed
from the topology dirty key because they shape sparse weights, not graph
geometry.
