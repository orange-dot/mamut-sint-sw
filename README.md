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
- sibling `EPM2` hardware repo: `mamut-sint-hw`

Current status:

- Sprint 1 core workspace is implemented
- patch/schema/identity/engine contracts are live in code
- Sprint 2 standalone audio milestone is implemented
- Sprint 3 standalone hardening is implemented
- Sprint 4 playable productization is implemented
- transport-boundary hardening has reached its local freeze point
- `EPM1` transport architecture is now frozen pending shared platform extraction
- plugin/editor work is intentionally deferred

## Canonical Smoke Path

```bash
cargo test --locked
```

This is the main public proof path for the repo today.

Workspace crates:

- `mamut-params` - stable parameter and macro registry
- `mamut-patch` - canonical TOML patch model and validation
- `mamut-identity` - macro-to-identity resolution
- `mamut-dsp` - shared DSP blocks and real-time utilities
- `mamut-field` - Rust-first offline GFM lattice model and render evidence
- `mamut-engine` - voice allocation, identity resolution, and audio render path
- `mamut-standalone` - standalone runtime with audio, MIDI, and demo performer

Quick start:

```bash
cargo test
cargo run -p mamut-standalone
cargo run -p mamut-standalone -- list-factory
cargo run -p mamut-standalone -- list-audio
cargo run -p mamut-standalone -- list-midi
tools/run-pc4-ag03.sh molten-horizon
tools/run-pc4-ag03.sh --midi-channel 1 molten-horizon
tools/run-pc4-ag03.sh --trace-midi molten-horizon
PC4GEN_ROOT=/path/to/pc4gen PC4GEN_PROFILE=/path/to/profile.json tools/run-pc4gen-consumer-smoke.sh --audio-device hw:<card>,<device>
cargo run -p mamut-standalone -- validate patches/factory/molten-horizon.toml
cargo run -p mamut-standalone -- dry-run patches/factory/furnace-choir.toml
cargo run -p mamut-standalone -- dry-run molten-horizon
cargo run -p mamut-standalone -- play --demo patches/factory/molten-horizon.toml
cargo run -p mamut-standalone -- play --audio-device hw:<card>,<device> cathedral-bloom
cargo run -p mamut-standalone -- play --audio-device 0 --midi-device 1 gravity-wake
cargo run -p mamut-standalone -- play --headless --audio-device hw:<card>,<device> molten-horizon
```

If you want the shortest product-facing smoke instead of the full test suite:

```bash
cargo run --locked -p mamut-standalone -- list-factory
```

Runtime notes:

- `play` accepts either a patch path or a factory patch name like `molten-horizon`
- `play` requires `--audio-device` and accepts only an ALSA list index or an
  explicit `hw:<card>,<device>` selector
- `play --alsa-period-frames`, `--alsa-buffer-frames`, and
  `--alsa-start-threshold-frames` expose direct ALSA tuning for latency and
  underrun work
- `play --midi-channel <1..16>` filters input to one MIDI channel when a port carries extra traffic
- `play --controller-profile <path>` loads TOML controller bindings such as
  `profiles/pc4-full.toml`
- `play --trace-midi` logs incoming MIDI messages to `stderr` for routing/debug sessions
- when a graphical session is available, `play` opens a native `egui`
  performance window by default with `Live`, `PC4`, and `Debug` tabs
- use `--headless` to force the terminal runtime surface instead of the
  performance window
- `tools/run-pc4-ag03.sh` is the shortest launch path for the first
  `PC4 -> mioXM DIN 1 -> AG06/AG03` standalone test and auto-resolves the
  Yamaha ALSA `hw:` output before launch; it loads `profiles/pc4-full.toml`
  by default
- `tools/run-pc4gen-consumer-smoke.sh` is the local live ALSA smoke for
  `pc4gen -> Mamut`, using the `mamut-epm1-smoke` generator scenario and
  `--trace-midi` as the contract oracle; it is a lab integration helper and
  requires explicit `PC4GEN_ROOT` plus either `PC4GEN_PROFILE` or
  `PC4GEN_DERIVED`
- headless controls include `status`, `patches`, `favorites`, `favorite`,
  `patch`, `next`, `prev`, `demo-patch`, `macro`, `panic`,
  `reset-controllers`, `audio`, `midi`, `demo`, and `quit`
- `list-factory` shows the curated factory bank with display names and descriptions
- `favorites`, `favorite <slot>`, `next`, `prev`, and MIDI `program change 0..7`
  all operate on the locked 8-slot live set
- `list-audio` enumerates ALSA `hw:` playback devices; `list-midi` enumerates
  MIDI inputs by index
- if no MIDI input is available, or `--demo` is passed, a built-in demo performer
  drives the synth
- switching audio from the runtime surface restarts the live session and resets
  current held notes and live macro state
- switching patches during active play performs a clean voice/controller reset before the new patch becomes active
- `PC4` full profile MIDI mapping lives in `profiles/pc4-full.toml`; it covers
  `K1..K9`, `S1..S9`, `SW1..SW9`, notes, pitch bend, `CC1` mod wheel, `CC64`
  sustain, channel aftertouch, and `program change 0..7`
- the windowed `PC4` tab is read-only: it displays Mamut's internal synth state
  for incoming PC4 controls and does not send MIDI or synth parameter changes
  back from GUI clicks/drags
- without `--controller-profile`, the legacy fallback keeps `CC16..20` mapped
  to `Gravitacija`, `Bloom`, `Heat`, `Ruin`, and `Swarm`

Factory bank:

- `molten-horizon` - open mass with late rupture
- `furnace-choir` - dense `Pec`-centered poly body
- `razor-thaw` - `Baklja`-ready lead
- `cathedral-bloom` - stable wide `Horizont` pad
- `ember-vault` - dry playable `Pec` bass
- `gravity-wake` - performance arc around `Gravitacija`
- `granite-plain` - dry poly anchor with restrained body
- `glass-tide` - wide animated pad around `Bloom` and `Swarm`

Live set slots:

- `0` - `molten-horizon`
- `1` - `cathedral-bloom`
- `2` - `ember-vault`
- `3` - `razor-thaw`
- `4` - `gravity-wake`
- `5` - `furnace-choir`
- `6` - `granite-plain`
- `7` - `glass-tide`

## Docs

- `docs/mamut-epm-program-map.md` - umbrella map for `Mamut EPM`, `EPM1`, and `EPM2`
- `docs/README.md` - local `EPM1` doc ownership and cross-repo references
- `docs/EPM1_TRANSPORT_FREEZE.md` - explicit transport freeze, assumptions, and resume point
- `docs/EPM1_SPRINT_6_PC4_PERFORMANCE_RIG.md` - Sprint 6 plan for `PC4` live rig, performance flow, and live UI
- `docs/EPM1_SPRINT_6A_HOST_UNDERRUN_STABILITY.md` - next work item for real-host underrun diagnosis and stability tuning
- `docs/EPM1_PC4_LIVE_PROFILE.md` - locked Sprint 6 `PC4` mapping, patch-switch, and controller policy
- `docs/EPM1_FIRST_PERFORMANCE_PLAYBOOK.md` - practical first-performance startup, smoke-pass, and live fallback runbook
- `docs/factory-bank-listening-checklist.md` - locked roles and listening pass for the shipped bank
- shared `mamut-platform` docs - Linux audio platform thesis, boundary, open
  questions, and ADR track

## Review Gates

Strict Claude `seL4` review agents are vendored in:

- `.claude/agents/`

These are development-time review prompts, not runtime dependencies.

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

## Known Limits

- plugin/editor work is intentionally deferred
- the strongest current proof path is the standalone runtime, not a plugin host
- real hardware performance validation still depends on the `PC4`-specific live profile and operator playbook

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for the canonical smoke path and runtime-facing sanity checks.
