# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

`mamut-sint-sw` is the canonical `EPM1` software line of the `Mamut EPM` program — a Linux-first standalone software synthesizer in Rust. The sibling repo `mamut-sint-hw` (`EPM2`) is the hardware continuation; shared identity language (`Horizont`, `Pec`, `Baklja`, `Gravitacija`) is consistent across both lines. Plugin/editor work was deferred during the early standalone maturation phase; ADR 0002 (`docs/adrs/0002-reopen-plugin-editor-track-via-vizia.md`) reopens that track on a phased `vizia` migration with an explicit off-ramp. The standalone runtime remains the primary proof path through the ADR 0002 Phase 3 cutover.

The Rust workspace at the root edits real source. Read `AGENTS.md` first — it owns coding rules and agent discipline that govern every change. This file complements `AGENTS.md` with build/test commands, architectural reading order, and gotchas not covered there.

## Build, test, run

Toolchain: stable Rust pinned via `rust-toolchain.toml`, edition 2024, MSRV 1.85. The workspace has `unsafe_code = "forbid"` and clippy warns on `dbg_macro`, `expect_used`, `panic`, `todo`, `unwrap_used` — keep that posture.

```bash
cargo test --locked                                                   # canonical smoke path; CI runs this
cargo fmt --all --check                                               # CI also runs this; format before pushing
cargo run --locked -p mamut-standalone -- list-factory                # shortest product-facing smoke
cargo run --locked -p mamut-standalone -- dry-run molten-horizon      # offline runtime sanity (no ALSA needed)
cargo run --locked -p mamut-standalone -- play --demo molten-horizon  # audible runtime smoke (requires ALSA device)

cargo test -p <crate> -- <test_name_substr>                           # run a single test
cargo test -p mamut-engine --doc                                      # doc tests for one crate
cargo build --release -p mamut-engine --examples                      # release-smoke job in CI builds these
cargo run --release -p mamut-engine --example core_output_safety_sweep # CI release-smoke also runs this
```

`cargo build` in this workspace pulls in `alsa`, `eframe`, `midir`, etc. — Linux build dependencies are non-trivial; CI installs `libasound2-dev`, `libudev-dev`, `libxkbcommon-dev`, `libwayland-dev`, `libx11-dev`, `libxrandr-dev`, `libxi-dev`, `libgl1-mesa-dev`, and `libxcb-{render0,shape0,xfixes0}-dev`. If a fresh `cargo build` fails on system libs, install those first.

`xtask/` is reserved for future helpers and is **not** a workspace member yet. Don't add it without a real task.

## Crate architecture

The workspace is layered as a strict DAG; respect the boundaries:

```
mamut-params      — parameter and macro registry; no deps; the leaf of the graph
mamut-patch       — TOML patch model + validation; depends on mamut-params, serde, toml, thiserror
mamut-identity    — macro-to-identity resolution; depends on mamut-patch, mamut-params
mamut-dsp         — shared real-time DSP blocks (oscillators, filters, envelopes, smoothers,
                    safety/limiter, denormal flush); no external deps; allocation-free in steady state
mamut-field       — offline GFM (gravitational-field model) lattice + BCS (bifurcation-coordinate
                    synthesis) primitives; render evidence lives in docs/dsp/
mamut-engine      — voice allocation, identity resolution, render path, GFM/BCS engine layers;
                    composes mamut-dsp + mamut-field + mamut-identity + mamut-patch + mamut-params
mamut-runtime     — runtime-facing transport/session logic: ALSA playback, MIDI ingress (midir),
                    crossbeam queues, rtrb ring buffers, CLI parsing, device enumeration,
                    midi trace, headless command surface
mamut-tui         — ratatui/crossterm terminal UI; depends on mamut-runtime + mamut-engine
mamut-standalone  — standalone binary: composes mamut-runtime + mamut-tui + eframe (egui)
                    performance window; the only crate with a `main`
```

Key invariant: a crate must not depend "upward". `mamut-dsp` and `mamut-field` are the real-time leaves; `mamut-engine` orchestrates them; `mamut-runtime` owns transport; `mamut-standalone` owns the binary surface. `mamut-runtime` was extracted from what used to live in `mamut-standalone/src/{audio_runtime,cli,commands,devices,midi_trace,runtime,session,types}.rs`.

### Engine internal structure

`crates/mamut-engine/src/` re-exports through `lib.rs`:

- `api.rs` — public types: `Engine`, `EngineConfig`, `NoteEvent`, `ControllerEvent`, `Scheduled<*>`, `ProcessBlock`, `BcsLayerMode/Snapshot`, `BcsScenario`, `GfmLayerMode/Snapshot`, `GfmVoiceProgramSelection`, smoothing/deadzone constants, `MASTER_DC_BLOCKER_CUTOFF_HZ`.
- `engine/mod.rs` — `Engine` itself (voice allocator, render path, control resolution).
- `gfm_layer.rs` — engine-owned GFM layer; armed momentary gate; program selection helpers (`select_gfm_program`, `select_gfm_program_for_patch`).
- `state.rs` — `EngineSnapshot`, `DirectParameters`, `OutputSafetySnapshot`, `PerformanceResponseSnapshot`, `VoicePhase/Snapshot`.
- `helpers.rs`, `tests.rs` — internal.

Engine examples under `crates/mamut-engine/examples/` are the documented evidence path for GFM/BCS work; CI's release-smoke job builds them all and runs `core_output_safety_sweep`.

## Real-time discipline (load-bearing)

The audio callback, render loop, MIDI callback, and voice allocator are **allocation-free in steady state**. The workspace lint posture enforces panic discipline; never reach for `unwrap`, `expect`, `panic!`, or `dbg!` on these paths. No logging, formatting, blocking calls, hidden `clone`/`collect`, or queue churn in render-time code.

Cross-thread structure (read this before touching transport):

- ALSA playback runs in a dedicated thread driven by `mamut-runtime::audio_runtime`. Frames flow through `rtrb` ring buffers; transport metrics are atomic counters.
- MIDI ingress (midir) drops messages on a session-owned worker thread, **not** in the MIDI callback. `status` and the GUI expose accepted/dropped/trace-dropped/coalesced-controller counters.
- MIDI `panic` and `reset_controllers` bypass the normal runtime-control queue and go through a priority action path directly.
- Engine snapshot generation and patch load are deliberately off the audio callback fast path (this was the work `EPM1_TRANSPORT_FREEZE.md` ratified).

## Transport freeze (read this before designing)

`docs/EPM1_TRANSPORT_FREEZE.md` freezes the standalone transport architecture. **Frozen**: direct redesign of the standalone transport boundary, queue architecture experimentation in `mamut-standalone`/`mamut-runtime`, callback/runtime-boundary redesign beyond bugfixes. **Allowed**: ordinary bugfixes, narrow correctness fixes that don't reopen transport architecture, docs alignment. Future transport evolution moves to a shared `mamut-platform` track, not here. If a task seems to want a "queue rewrite" or "lock-free transport experiment", stop and confirm with the user — that work is out of scope for this repo.

## GUI track (read this before designing UI)

The current `egui` GUI is being redesigned. The information architecture moves from five legacy tabs (`LIVE`, `SOUND LAB`, `ENGINE`, `PC4`, `DEBUG`) to four screens (`PERFORM`, `SOUND`, `SYSTEM`, `INSPECT`) per ADR 0003. The toolkit moves from `egui` to `vizia` per ADR 0002, behind a phased plan with an explicit off-ramp. The plugin-editor track is reopened by ADR 0002 to enable `VST3` / `CLAP` distribution.

Read these in order before authoring GUI changes:

- `docs/adrs/0002-reopen-plugin-editor-track-via-vizia.md` — toolkit decision, phase plan, off-ramp, Phase 1 kill criteria.
- `docs/adrs/0003-gui-information-architecture.md` — four-screen IA, cut rule, current → new mapping.
- `docs/ui/vizia-design-system.md` — palette, typography, knob geometry, panel chrome, screen wireframes, high-risk widgets.

During ADR 0002 Phases 1 through 3, the workspace carries both `egui` (via `mamut-standalone`) and `vizia` (via a new `mamut-vizia` crate added in Phase 1) in parallel. The toolkits do not share a feature flag. The Phase 3 cutover removes `crates/mamut-standalone/src/gui/`. Until then, `egui` work is restricted to narrow bugfixes; new GUI investment goes into the `mamut-vizia` crate.

The transport freeze remains in effect. GUI work does not touch the transport boundary, the audio callback, the MIDI ingress callback, or the voice allocator. The new `SessionStateSource` trait introduced in ADR 0002 is a read-only consumer interface over already-published metrics and snapshots; it does not reshape the transport.

## Review gates (mandatory for core changes)

Vendored under `.claude/agents/` (the name `claude` is historical; agents apply regardless of operator). `docs/review-gates.md` is the canonical spec; `tools/review/` ships shell helpers that print the recommended prompt and diff context.

| Change touches                                                                  | Required agent                          |
|---------------------------------------------------------------------------------|-----------------------------------------|
| `mamut-engine`, `mamut-dsp`, `mamut-runtime`, `mamut-standalone`, `mamut-patch` | `sel4-rust-systems-reviewer`            |
| Audio render loop, callback, voice allocator, filter/osc/final-stage hot path   | `sel4-rust-execution-optimizer`         |
| `README.md`, `docs/`, patch schema, crate boundaries, runtime ownership         | `sel4-integrated-systems-reviewer`      |
| A single risky file in a Rust subsystem                                         | `sel4-rust-single-file-reviewer`        |

Helper scripts (do not mutate sources; print agent + scope + prompt):

```bash
tools/review/run-rust-file-review.sh <repo-relative-rust-file>
tools/review/run-rust-subsystem-review.sh <crate-or-subsystem-paths...>
tools/review/run-rust-hotpath-review.sh <hot-path-files...>
tools/review/run-integrated-review.sh <paths-and-docs...>
```

## Standalone runtime surface

`cargo run -p mamut-standalone -- <command>` accepts: `list-factory`, `list-audio`, `list-midi`, `validate <path>`, `dry-run [<patch>]`, `play [...]`, `help`. `play` is the live entry point.

`play` essentials (full list in `README.md`):

- `--audio-device` is required and accepts only an ALSA list-index or `hw:<card>,<device>`.
- Sample rate defaults to **96 kHz**. Supported: `44100`, `48000`, `88200`, `96000`, `176400`, `192000`. Pass `--sample-rate 44100` for the legacy path.
- ALSA tuning: `--alsa-period-frames`, `--alsa-buffer-frames`, `--alsa-start-threshold-frames`. First 96 kHz AG03 live tests start at `512 / 2048 / 2048` (≈5.33 ms period, 21.33 ms buffer at 96 kHz).
- `--midi-channel <1..16>`, `--controller-profile <path>` (default fallback maps `CC16..20` to the five public macros), `--trace-midi`, `--demo`, `--headless`.
- A graphical session opens an `egui` performance window. The legacy tab structure (`LIVE`, `SOUND LAB`, `ENGINE`, `PC4`, `DEBUG`) is being redesigned to the four-screen information architecture in ADR 0003 (`PERFORM`, `SOUND`, `SYSTEM`, `INSPECT`) and re-implemented in `vizia` per ADR 0002; until that cutover, the legacy tabs remain in place. The `PC4` mirror is **read-only** in both the legacy GUI and the redesign — it displays Mamut's internal state for incoming controls; GUI clicks/drags do not send MIDI back.

Headless interactive controls (when stdin is a TTY and not `--headless`): `status`, `patches`, `favorites`, `favorite <slot>`, `patch`, `next`, `prev`, `demo-patch`, `macro`, `panic`, `reset-controllers`, `audio`, `midi`, `demo`, `quit`. Switching audio or patches mid-play resets held notes and live macro state cleanly.

Factory bank lives in `patches/factory/*.toml`; the locked 8-slot live set is `0:molten-horizon, 1:cathedral-bloom, 2:ember-vault, 3:razor-thaw, 4:gravity-wake, 5:furnace-choir, 6:granite-plain, 7:glass-tide`. MIDI program-change `0..7` selects these slots.

## Live rig helpers

- `tools/run-pc4-ag03.sh` — the shortest launch path for the `PC4 → mioXM DIN 1 → AG06/AG03` live rig. Auto-resolves the Yamaha ALSA `hw:` output via `/proc/asound/cards`+`pcm`, loads `profiles/pc4-full.toml`. Env overrides: `MAMUT_AUDIO_DEVICE`, `MAMUT_ALSA_*_FRAMES`, `MAMUT_MIDI_DEVICE`, `MAMUT_MIDI_CHANNEL`, `MAMUT_CONTROLLER_PROFILE`, `MAMUT_CAPTURE_DIR`, `MAMUT_CARGO_PROFILE`, `MAMUT_AG_CARD_PATTERN`.
- `tools/run-pc4gen-consumer-smoke.sh` — local lab integration for `pc4gen → Mamut`; requires `PC4GEN_ROOT` plus `PC4GEN_PROFILE` or `PC4GEN_DERIVED`.

## Reading order for new contributors

1. `README.md` — feature surface, factory bank, runtime flags
2. `AGENTS.md` — coding rules, agent discipline (the rules govern every change)
3. `docs/README.md` — local doc map and external anchors
4. `docs/EPM1_TRANSPORT_FREEZE.md` — what you are not allowed to redesign
5. `docs/EPM1_PC4_LIVE_PROFILE.md` — locked PC4 control map, slot policy, performance-window truth model
6. `docs/EPM1_FIRST_PERFORMANCE_PLAYBOOK.md` — startup runbook for the first real performance
7. `docs/review-gates.md` — exactly which agent gates are mandatory for which change type
8. `docs/dsp/{primitives,render-path,control-identity}-math.md` — DSP math companions
9. `docs/adrs/0001-standalone-midi-ingress-hardening.md` — accepted MIDI hygiene decision
10. `docs/adrs/0002-reopen-plugin-editor-track-via-vizia.md` — accepted decision to reopen the plugin/editor track on a phased `vizia` migration with off-ramp
11. `docs/adrs/0003-gui-information-architecture.md` — accepted four-screen GUI information architecture (`PERFORM`, `SOUND`, `SYSTEM`, `INSPECT`)
12. `docs/ui/vizia-design-system.md` — palette, typography, knob geometry, panel chrome, and screen wireframes that govern the GUI redesign
13. `docs/live-sessions/` — real hardware run evidence (preserve as evidence; not portable defaults)

## Things to avoid

- Adding allocation, logging, formatting, panic-prone calls, or `clone`/`collect` to the audio callback, render loop, MIDI callback, or voice allocator.
- Reopening transport architecture inside `mamut-runtime` or `mamut-standalone` while the freeze in `docs/EPM1_TRANSPORT_FREEZE.md` is active.
- Letting docs claim a boundary, schema, or runtime behavior that code no longer implements (the integrated reviewer enforces this).
- Adding a workspace member, raising MSRV, weakening the `unsafe_code = "forbid"` posture, or relaxing the clippy panic/unwrap warnings without an explicit task.
- Renaming or reflowing factory patches or live-set slots — they are locked. The `glass-tide` etc. names are spec, not placeholders.
- Initializing or rewriting git history at the workspace root — this child project is one of many extracted snapshots in the surrounding lab workspace.
