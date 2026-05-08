# Repository Guidelines

## Scope

This repo is the canonical `EPM1` software/runtime line for `Mamut EPM`.
It owns the standalone runtime, engine, DSP, patch model, factory bank, and
playback ergonomics for the current playable software instrument. Keep plugin
and editor work deferred unless the task explicitly reopens that boundary.

## Project Structure

- `crates/mamut-params`: stable parameter and macro registry.
- `crates/mamut-patch`: canonical TOML patch model and validation.
- `crates/mamut-identity`: macro-to-identity resolution.
- `crates/mamut-dsp`: shared DSP blocks and real-time utilities.
- `crates/mamut-field`: offline GFM lattice model and render evidence.
- `crates/mamut-engine`: voice allocation, identity resolution, and render path.
- `crates/mamut-runtime`: runtime-facing transport/session logic.
- `crates/mamut-tui`: terminal UI surface.
- `crates/mamut-standalone`: standalone audio/MIDI/demo application.
- `docs/`: product, runtime, review-gate, and live-session documentation.
- `patches/`, `profiles/`, and `tools/`: factory patches, controller profiles,
  and repo-local helper scripts.

## Build And Test

Use locked Cargo commands when validating repo behavior:

- `cargo test --locked`: canonical smoke path.
- `cargo run --locked -p mamut-standalone -- list-factory`: shortest
  product-facing smoke.
- `cargo run --locked -p mamut-standalone -- dry-run molten-horizon`: quick
  runtime sanity path.
- `cargo run --locked -p mamut-standalone -- play --demo molten-horizon`:
  optional audible/demo runtime path when the environment supports it.

When editing Rust code, preserve the workspace lint posture:

- `unsafe_code = "forbid"`
- clippy warns on `dbg!`, `expect`, `panic`, `todo`, and `unwrap`

## Coding Rules

Keep realtime audio paths allocation-free in steady state. Avoid logging,
formatting, blocking calls, hidden `clone`/`collect` work, and queue churn in
render, audio callback, MIDI callback, and voice-allocation fast paths.

Patch, identity, runtime, and documentation changes must describe the same
system truth. Do not let docs claim a boundary, schema, or runtime behavior
that code no longer implements.

Prefer explicit ownership and timing boundaries over convenience abstractions.
If a helper hides allocation, synchronization, panic behavior, or cross-thread
ownership, it must earn its place.

## Agent Discipline

Repo-local review disciplines are vendored under:

- `.claude/agents/`

The directory name `claude` is historical. These files are not Claude-only.
Codex agents working in this repo must use the same disciplines and review bar
when the task matches them.

Required gates for core runtime work:

- `sel4-rust-systems-reviewer` for `mamut-engine`, `mamut-dsp`,
  `mamut-runtime`, `mamut-standalone`, and `mamut-patch`.
- `sel4-rust-execution-optimizer` for hot-path audio/runtime changes.
- `sel4-integrated-systems-reviewer` for architecture, docs, schema, and
  boundary shifts.

Use `docs/review-gates.md` and `tools/review/` for the detailed gate sequence.
