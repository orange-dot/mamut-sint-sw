# GFM v2.0 Seven Program Evidence

Status note, 2026-05-22: this document describes an experimental seven-program
surface. The current production GFM contract remains the three-program
`auto/horizont/pec/baklja` surface described in
`docs/dsp/gfm-performance-control-contract.md`.

Date: 2026-05-09

## Scope

This slice turns GFM from a three-program selector into a seven-program musical
surface with explicit patch and live-session program control:

- `horizont`
- `pec`
- `baklja`
- `gravity`
- `swarm`
- `rupture`
- `recovery`

The new programs are still `mamut-field` / `mamut-engine` primitives, not a
plugin or editor boundary. Factory patches may pin `engine.gfm.program`; the
runtime `GFM Program` direct param is session-only and does not mutate exported
patch data.

## Contract

- `GfmProgramId::ALL_PERFORMANCE` is the canonical performance list.
- `GfmProgramId::control_index()` maps `1..=7`; `0` means live AUTO.
- Patch TOML accepts `engine.gfm.program = "auto" | "horizont" | "pec" |
  "baklja" | "gravity" | "swarm" | "rupture" | "recovery"`.
- Live override has higher priority than patch selection. Clearing live
  override returns to the patch program if one is pinned, otherwise to automatic
  identity scoring.
- Live, Sound Lab, and Engine tabs expose the same GFM program strip.

## Character Intent

- `horizont`: open, broad, stable field.
- `pec`: hot mass and furnace body.
- `baklja`: flame-ready fracture with controlled recovery.
- `gravity`: inward pull, slow mass trajectory.
- `swarm`: moving micro-agent field with wide dispersion.
- `rupture`: explicit fracture/gate behavior.
- `recovery`: healing motion and low-rupture return.

## Verification

Commands run:

```sh
cargo fmt --all
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked -p mamut-dsp
cargo test --locked -p mamut-field
cargo test --locked -p mamut-engine gfm_field_voice -- --nocapture
cargo test --locked -p mamut-engine gfm_layer -- --nocapture
cargo test --locked -p mamut-engine --lib -- --skip gfm_field_voice --skip gfm_layer
cargo test --locked -p mamut-patch
cargo test --locked -p mamut-params
cargo test --locked -p mamut-runtime -p mamut-standalone
cargo run --locked -p mamut-standalone -- list-factory
cargo run --locked -p mamut-standalone -- dry-run molten-horizon
```

Observed results:

- `mamut-dsp`: 50 passed.
- `mamut-field`: 29 passed.
- `mamut-engine gfm_field_voice`: 12 passed.
- `mamut-engine gfm_layer`: 15 passed.
- remaining `mamut-engine --lib`: 37 passed.
- `mamut-patch`: 7 passed.
- `mamut-params`: 5 passed.
- `mamut-standalone`: 78 passed.
- `cargo check` and `cargo clippy -D warnings`: clean.

Review-gate scripts were run for subsystem, hot-path, integrated, and
single-file `gfm_layer.rs` scopes. The actual review pass found no blocking
findings after the live-session override test was added.
