# Mamut EPM / EPM1 Software Repo

`mamut-sint-sw` is the canonical `EPM1` software repo inside the broader
`Mamut EPM` program.

Canonical line split:

- `EPM1` = current software/runtime line in this repo `mamut-sint-sw`
- `EPM2` = hardware line in sibling repo `mamut-sint-hw`

Current product posture:

- `EPM1` is the current primary playable expression of `Mamut EPM`
- `EPM2` is the hardware continuation of the same identity
- `Mamut EPM` is the umbrella concept across both lines

This repo owns the live software/runtime implementation for `EPM1`:

- standalone runtime
- engine and DSP implementation
- patch runtime code and validation
- factory patch bank and playback ergonomics

Shared identity language stays aligned across both lines:

- `Horizont`
- `Pec`
- `Baklja`
- `Gravitacija`

Program map and repo split:

- `docs/mamut-epm-program-map.md`
- sibling `EPM2` hardware repo: `/home/dev/sel4/mamut-sint-hw`

Current status:

- Sprint 1 core workspace is implemented
- patch/schema/identity/engine contracts are live in code
- Sprint 2 standalone audio milestone is implemented
- Sprint 3 standalone hardening is implemented
- Sprint 4 playable productization is in progress
- plugin/editor work is intentionally deferred

Workspace crates:

- `mamut-params` - stable parameter and macro registry
- `mamut-patch` - canonical TOML patch model and validation
- `mamut-identity` - macro-to-identity resolution
- `mamut-dsp` - shared DSP blocks and real-time utilities
- `mamut-engine` - voice allocation, identity resolution, and audio render path
- `mamut-standalone` - standalone runtime with audio, MIDI, and demo performer

Quick start:

```bash
cargo test
cargo run -p mamut-standalone
cargo run -p mamut-standalone -- list-factory
cargo run -p mamut-standalone -- list-audio
cargo run -p mamut-standalone -- list-midi
cargo run -p mamut-standalone -- validate patches/factory/molten-horizon.toml
cargo run -p mamut-standalone -- dry-run patches/factory/furnace-choir.toml
cargo run -p mamut-standalone -- dry-run molten-horizon
cargo run -p mamut-standalone -- play --demo patches/factory/molten-horizon.toml
cargo run -p mamut-standalone -- play cathedral-bloom
cargo run -p mamut-standalone -- play --audio-device 0 --midi-device 1 gravity-wake
```

Runtime notes:

- `play` accepts either a patch path or a factory patch name like `molten-horizon`
- when `play` is started from a terminal, it opens a tiny runtime control surface
  with `status`, `patches`, `patch`, `macro`, `audio`, `midi`, `demo`, and `quit`
- `list-factory` shows the curated factory bank with display names and descriptions
- `list-audio` and `list-midi` enumerate selectable devices by index
- `play` opens the default audio output unless `--audio-device` is provided
- if no MIDI input is available, or `--demo` is passed, a built-in demo performer
  drives the synth
- switching audio from the runtime surface restarts the live session and resets
  current held notes and live macro state
- MIDI CC mapping:
  - `1` -> mod wheel
  - `64` -> sustain
  - `71` -> `Heat`
  - `73` -> `Bloom`
  - `74` -> `Gravitacija`
  - `75` -> `Ruin`
  - `76` -> `Swarm`

Factory bank:

- `molten-horizon` - open mass with late rupture
- `furnace-choir` - dense `Pec`-centered poly body
- `razor-thaw` - `Baklja`-ready lead
- `cathedral-bloom` - stable wide `Horizont` pad
- `ember-vault` - dry playable `Pec` bass
- `gravity-wake` - performance arc around `Gravitacija`
- `granite-plain` - dry poly anchor with restrained body
- `glass-tide` - wide animated pad around `Bloom` and `Swarm`

## Docs

- `docs/mamut-epm-program-map.md` - umbrella map for `Mamut EPM`, `EPM1`, and `EPM2`
- `docs/README.md` - local `EPM1` doc ownership and cross-repo references
- `docs/factory-bank-listening-checklist.md` - locked roles and listening pass for the shipped bank

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
