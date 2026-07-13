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
- transport-boundary hardening reached its local freeze point; the freeze was
  rescinded on 2026-07-06 by ADR 0005 for the MIDI 2.0 UMP track
- the MIDI 2.0 migration (UMP-first internal protocol, per-note
  expressiveness deep in the engine/DSP) is planned in
  `docs/EPM1_BACKLOG_SET4_MIDI2_UMP_EXPRESSIVENESS.md`
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
- `mamut-runtime` - runtime-facing transport/session logic (ALSA playback, MIDI ingress, CLI)
- `mamut-tui` - ratatui/crossterm terminal UI (lib + headless/TUI `play` binary)
- `mamut-standalone` - standalone runtime with audio, MIDI, and demo performer
- `mamut-seq` - laptop MIDI sequencer: virtual-port scenario player (default development-time input)

Quick start:

```bash
cargo test
cargo run -p mamut-standalone
cargo run -p mamut-standalone -- list-factory
cargo run -p mamut-standalone -- list-audio
cargo run -p mamut-standalone -- list-midi
tools/run-pc4-ag03.sh molten-horizon
tools/run-pc4-ag03.sh --midi-channel 2 molten-horizon
tools/run-pc4-ag03.sh --trace-midi molten-horizon
PC4GEN_ROOT=/path/to/pc4gen PC4GEN_PROFILE=/path/to/profile.json tools/run-pc4gen-consumer-smoke.sh --audio-device hw:<card>,<device>
cargo run -p mamut-standalone -- validate patches/factory/molten-horizon.toml
cargo run -p mamut-standalone -- dry-run patches/factory/furnace-choir.toml
cargo run -p mamut-standalone -- dry-run molten-horizon
cargo run -p mamut-standalone -- play --demo patches/factory/molten-horizon.toml
cargo run -p mamut-standalone -- play --audio-device hw:<card>,<device> cathedral-bloom
cargo run -p mamut-standalone -- play --audio-device 0 --midi-device 1 gravity-wake
cargo run -p mamut-standalone -- play --headless --audio-device hw:<card>,<device> molten-horizon
cargo run -p mamut-seq -- ports
cargo run -p mamut-seq -- validate scenarios/gfm-gate-arm.toml
cargo run -p mamut-seq -- play scenarios/gfm-gate-arm.toml
cargo run -p mamut-seq -- live
```

## Laptop MIDI input (`mamut-seq`)

`mamut-seq` is the default development-time MIDI source: instead of the physical
`PC4 -> mioXM` rig, generate MIDI on the laptop. It is a pure MIDI source over a
virtual ALSA port, so it changes nothing in the runtime. The stage path
(`PC4 -> mioXM -> EPM1`, see `docs/EPM1_FIRST_PERFORMANCE_PLAYBOOK.md`) is
unchanged, and `mamut-seq` runs are synthetic development evidence — real-rig
evidence in `docs/live-sessions/` stays a separate, hardware-only class.

Two-terminal flow — Mamut in one terminal, `mamut-seq` in the other:

```bash
# terminal 1 - Mamut, selecting the mamut-seq port as input
cargo run -p mamut-standalone -- play \
  --audio-device hw:<card>,<device> \
  --midi-device mamut-seq --midi-channel 2 \
  --controller-profile profiles/pc4-full.toml --trace-midi cathedral-bloom

# terminal 2 - mamut-seq, sending a scripted scenario
cargo run -p mamut-seq -- play scenarios/gfm-gate-arm.toml
```

`tools/run-seq-smoke.sh` prints this quickstart (and, with `--run --audio-device
<selector>`, drives it end-to-end). Scenarios live in `scenarios/` and address
controls by profile name (never raw CC), resolved through `profiles/pc4-full.toml`.
`mamut-seq validate <scenario>` prints the expanded schedule without sending;
`mamut-seq live` is an interactive computer-keyboard mode for ad-hoc sound checks
(the keymap is shown in-app). Full design:
`docs/EPM1_BACKLOG_SET3_LAPTOP_MIDI_SEQUENCER.md`.

If you want the shortest product-facing smoke instead of the full test suite:

```bash
cargo run --locked -p mamut-standalone -- list-factory
```

Runtime notes:

- `play` accepts either a patch path or a factory patch name like `molten-horizon`
- `play` requires `--audio-device` and accepts only an ALSA list index or an
  explicit `hw:<card>,<device>` selector
- `play` defaults to `96 kHz`; pass `--sample-rate 44100` for the older
  44.1 kHz behavior or `--sample-rate 192000` for the later high-rate
  experiment path. Supported rates are `44100`, `48000`, `88200`, `96000`,
  `176400`, and `192000`.
- `play --alsa-period-frames`, `--alsa-buffer-frames`, and
  `--alsa-start-threshold-frames` expose direct ALSA tuning for latency and
  underrun work
- first 96 kHz AG03 live tests should start with `--alsa-period-frames 512`,
  `--alsa-buffer-frames 2048`, and `--alsa-start-threshold-frames 2048`; that
  is about a 5.33 ms period and 21.33 ms buffer at 96 kHz. The same frame
  counts represent half as much wall-clock slack again at 192 kHz.
- `play --midi-channel <1..16>` filters input to one MIDI channel when a port carries extra traffic
- `play --controller-profile <path>` loads TOML controller bindings such as
  `profiles/pc4-full.toml`
- `play --trace-midi` logs incoming MIDI messages to `stderr` for routing/debug sessions
- MIDI trace and latest-control UI work run on a session-owned worker, not in
  the MIDI callback; `status` and the GUI expose accepted, dropped, trace
  dropped, and coalesced-controller counters
- MIDI `panic` and `reset_controllers` bypass the normal runtime-control queue
  and use the priority action path directly
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
- controller-profile binding `kind`s: `macro`, `direct_param`, `gfm_layer_amount`,
  `bcs_layer_amount`/`bcs_layer_gain`, `bcs_layer_enabled`, `mozaik_control`,
  `runtime_action`, `toggle_param`, `reserved`
- `mozaik_control` (SET5-8) binds a CC to a Mozaik session control via
  `target = "mix" | "slope" | "contrast" | "phason" | "drift"`; the CC value maps
  linearly `0..127 -> 0.0..1.0` and routes through the same
  `EngineCommand::SetMozaikParam` path as the headless `mozaik set` (one truth for
  clamping, smoothing, and the slope detent-snap). No `mozaik_control` binding
  enables, disables, or re-seeds the Mozaik layer — enable stays a deliberate
  `--mozaik` / `mozaik on` act, so a stray CC cannot restart a running texture
  (unlike `bcs_layer_enabled`, which does gate the BCS layer from a CC)
- `profiles/android-touch.toml` is the example channel-1 surface for the PC4MS
  Android touch instrument (sibling repo `pc4-microkit-studio`,
  `docs/PC4MS-TOUCH-SURFACE-BACKLOG.md` items `TS-1..3`): macros `CC16..20`,
  expression `CC11`, and Mozaik on the app's `Profile CC 21-31` slide-lane band

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
- `docs/EPM1_TRANSPORT_FREEZE.md` - rescinded transport freeze (see ADR 0005); preserved for history
- `docs/EPM1_BACKLOG_SET4_MIDI2_UMP_EXPRESSIVENESS.md` - MIDI 2.0 UMP migration backlog (`SET4-0..12`; `SET4-9..12` is the Android touch-surface track — native UMP over a UDP link)
- `docs/EPM1_SPRINT_6_PC4_PERFORMANCE_RIG.md` - Sprint 6 plan for `PC4` live rig, performance flow, and live UI
- `docs/EPM1_SPRINT_6A_HOST_UNDERRUN_STABILITY.md` - next work item for real-host underrun diagnosis and stability tuning
- `docs/EPM1_PC4_LIVE_PROFILE.md` - locked Sprint 6 `PC4` mapping, patch-switch, and controller policy
- `docs/EPM1_FIRST_PERFORMANCE_PLAYBOOK.md` - practical first-performance startup, smoke-pass, and live fallback runbook
- `docs/factory-bank-listening-checklist.md` - locked roles and listening pass for the shipped bank
- shared `mamut-platform` docs - Linux audio platform thesis, boundary, open
  questions, and ADR track

## Review Gates

Strict `seL4` review disciplines are vendored in:

- `.claude/agents/`

The `claude` directory name is historical. These are development-time review
prompts, not runtime dependencies, and Codex agents in this repo should apply
the same disciplines when the task matches them.

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
