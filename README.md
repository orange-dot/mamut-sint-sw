# mamut-sint-sw

`mamut-sint-sw` is the sibling Rust workspace for the software-clone line of
the `Mamut` synth concept.

Current status:

- Sprint 1 core workspace is implemented
- patch/schema/identity/engine contracts are live in code
- standalone is currently a headless validation and dry-run shell
- plugin/editor work is intentionally deferred

Workspace crates:

- `mamut-params` - stable parameter and macro registry
- `mamut-patch` - canonical TOML patch model and validation
- `mamut-identity` - macro-to-identity resolution
- `mamut-dsp` - shared DSP utilities and buffer helpers
- `mamut-engine` - voice allocation and process pipeline skeleton
- `mamut-standalone` - headless runtime for patch validation and engine dry-runs

Quick start:

```bash
cargo test
cargo run -p mamut-standalone
cargo run -p mamut-standalone -- validate patches/factory/molten-horizon.toml
cargo run -p mamut-standalone -- dry-run patches/factory/furnace-choir.toml
cargo run -p mamut-standalone -- play --demo patches/factory/molten-horizon.toml
```

Runtime notes:

- `play` opens the default audio output device
- if no MIDI input is available, or `--demo` is passed, a built-in demo performer
  drives the synth
- MIDI CC mapping:
  - `1` -> mod wheel
  - `64` -> sustain
  - `71` -> `Heat`
  - `73` -> `Bloom`
  - `74` -> `Gravitacija`
  - `75` -> `Ruin`
  - `76` -> `Swarm`

## Review Gates

Strict Claude `seL4` review agents are vendored in:

- `.claude/agents/`

Repo-local workflow docs and helpers:

- `docs/review-gates.md`
- `tools/review/run-rust-file-review.sh`
- `tools/review/run-rust-subsystem-review.sh`
- `tools/review/run-integrated-review.sh`
- `tools/review/run-rust-hotpath-review.sh`

Mandatory gates for core runtime work:

- `sel4-rust-systems-reviewer` for `mamut-engine`, `mamut-dsp`, `mamut-standalone`, `mamut-patch`
- `sel4-rust-execution-optimizer` for hot-path audio/runtime changes
- `sel4-integrated-systems-reviewer` for architecture, docs, schema, and boundary shifts
