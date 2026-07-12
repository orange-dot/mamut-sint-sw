# EPM1 Backlog Set 3: Laptop MIDI Sequencer (`mamut-seq`)

Date: 2026-07-06

Status: proposed backlog. No implementation from this document has landed.
Each item ships as its own slice with its own evidence document and review
gates.

## Summary

Operator decision (2026-07-06): day-to-day development input moves from the
physical `PC4 -> mioXM` rig to MIDI generated on the laptop. This backlog
builds `mamut-seq`, a repo-local sequencer tool that becomes the default
development-time MIDI source for `EPM1`.

Two decisions are already made and are inputs to this backlog, not open
questions:

- **Home**: a new workspace member `crates/mamut-seq` in this repo (there is
  no `pc4gen` project inside the lab; the tool is EPM1-specific because it
  speaks the locked PC4 profile dialect and rides the same toolchain).
- **Shape (v1)**: hybrid — a deterministic scenario player as the core, plus
  a minimal interactive live TUI mode for ad-hoc playing.

The PC4 remains the stage instrument. `mamut-seq` runs are synthetic
development evidence; real-rig evidence in `docs/live-sessions/` stays a
separate, hardware-only class. Final classification of live-rig failures
still requires the physical rig.

Items:

1. `SET3-1` — crate scaffold plus virtual MIDI port
2. `SET3-2` — scenario schema plus deterministic player
3. `SET3-3` — scenario library v1 (the "replace PC4 for dev" payload)
4. `SET3-4` — live TUI mode
5. `SET3-5` — docs and workflow integration

## Shared Boundary Constraints

- **Zero runtime changes**: `mamut-seq` is a pure MIDI source. No changes to
  `mamut-runtime` MIDI ingress, the transport boundary, or
  `mamut-standalone` are needed or allowed under this backlog; `mamut-seq`
  stays a pure MIDI source by construction. (Written under the transport
  freeze, since rescinded by ADR 0005; the zero-runtime-changes rule stands
  on its own for this set. UMP output for `mamut-seq` is Backlog Set 4
  `SET4-7` work.)
- **No new external dependencies in v1**: everything needed is already in
  `[workspace.dependencies]` — `midir` (virtual port), `serde`/`toml`
  (scenarios), `ratatui`/`crossterm` (live mode), `anyhow`/`thiserror`.
- Workspace lint posture applies (`unsafe_code = "forbid"`, no
  `unwrap`/`expect`/`panic` in reachable paths).
- Scenarios address controls by **profile name** (for example
  `"K8 GFM Gate"`, `"S9"`), resolved through `profiles/pc4-full.toml` at load
  time. Scenario files never hardcode CC numbers; the locked profile stays
  the single source of truth for the control map.
- Adding the workspace member is the explicit task that the repo guidance
  requires; `CLAUDE.md`, `AGENTS.md`, and `README.md` crate lists are updated
  in the same change that adds the crate (docs and code must describe the
  same system truth).

## SET3-1: Crate Scaffold Plus Virtual MIDI Port

### Rationale

The foundation: a binary crate that opens a virtual ALSA MIDI output port
that `mamut-standalone` can select like any hardware input. Once this exists,
every later item is pure tool-side work.

### Design Sketch

- new workspace member `crates/mamut-seq`, binary `mamut-seq`
- virtual output port via `midir` virtual-port support on ALSA, default port
  name `mamut-seq` (suffix the PID on name collision)
- CLI skeleton:
  - `mamut-seq ports` — list what the tool can see (sanity aid)
  - `mamut-seq validate <scenario.toml>` — parse and print the expanded
    event schedule without sending
  - `mamut-seq play <scenario.toml>` — send a scenario
  - `mamut-seq live` — interactive mode (SET3-4)
  - common flags: `--channel <1..16>` (default `2`, the locked rig
    convention), `--port-name <name>`
- panic hygiene from day one: on SIGINT/quit, send all-notes-off, sustain
  clear, and controller reset before closing the port — the tool must never
  leave the instrument with stuck notes

### Acceptance

- `cargo run --locked -p mamut-standalone -- list-midi` enumerates the
  `mamut-seq` port while the tool runs
- a smoke note sequence is visible in `play --trace-midi` on the Mamut side
  and audibly triggers voices
- `cargo test --locked` includes the crate's unit tests
- `CLAUDE.md`, `AGENTS.md`, `README.md` crate lists updated in the same
  change

### Evidence

- proposed: `docs/EPM1_SEQ_V0.1_VIRTUAL_PORT_EVIDENCE.md`

### Review Gates

- `sel4-integrated-systems-reviewer` (new workspace member is an
  architecture change)
- `sel4-rust-systems-reviewer` (crate code)

### Size

S–M

## SET3-2: Scenario Schema Plus Deterministic Player

### Rationale

The scenario player is what makes laptop MIDI *better* than hand-playing for
development: deterministic, repeatable gestures that double as evidence
inputs.

### Design Sketch

TOML scenario format v1:

- header: `name`, `description`, optional `channel` override, `seed`
- timeline events: `note_on`/`note_off`/`chord`/`hold`, velocity, channel
  aftertouch envelopes (attack/hold/release ramps), pitch bend, and
  `program_change` `0..7`
- CC events and CC ramps addressed by profile control name (resolved through
  `profiles/pc4-full.toml`)
- structure: explicit `wait`, sections, `loop { count }`
- timing base decided in-slice (absolute milliseconds versus beat/tempo
  grid; default position: absolute-ms with an optional tempo helper)

Player behavior:

- schedule expansion is a pure function: same file plus same seed yields an
  identical absolute event list (this is what `validate` prints)
- sending uses monotonic absolute-deadline scheduling — no cumulative drift;
  measured send jitter is reported at exit
- deterministic event *content* (bytes and order) is guaranteed; wall-clock
  jitter is measured and reported, never claimed away

### Acceptance

- unit tests over schema edge cases: note-off pairing, overlapping notes, CC
  clamp and deadzone boundaries, loop expansion, unknown control names
  (hard error naming the profile)
- determinism test: two expansions of the same scenario are identical
- a reference scenario captured via `play --trace-midi` matches the
  `validate` output event-for-event
- jitter report recorded in the evidence doc for the reference host

### Evidence

- proposed: `docs/EPM1_SEQ_V0.2_SCENARIO_PLAYER_EVIDENCE.md`

### Review Gates

- `sel4-rust-systems-reviewer`

### Size

M

## SET3-3: Scenario Library v1

### Rationale

This is the payload that actually replaces the PC4 for development: the
existing manual playbooks become runnable files. A `scenarios/` directory at
the repo top level (peer of `patches/` and `profiles/`) holds the curated
set.

### Deliverables

- `scenarios/gfm-gate-arm.toml` — `EPM1_GFM_LIVE_BUG_PLAYBOOK.md` Pass 1:
  hold a chord, phases with and without aftertouch press/release, `K8`
  open/close
- `scenarios/gfm-mapping-sweep.toml` — Pass 2: one control at a time
  (candidate GFM control, aftertouch, `K1`, `K2`, mod wheel) with settle
  gaps
- `scenarios/gfm-host-load.toml` — Pass 3: 60–120 s of dense chords,
  sustain, aftertouch, the GFM color gesture, one patch change
- `scenarios/bcs-layer-basic.toml` — `SW9` enable, `S9` gain, lowest-held-
  note follow, release-to-silence check
- `scenarios/masnoca-listening.toml` — single sustained notes and dense
  chord blocks for tonal-architecture listening work
- `scenarios/factory-bank-pass.toml` — per-slot `program change` plus a
  short phrase skeleton per `docs/factory-bank-listening-checklist.md`
- `EPM1_GFM_LIVE_BUG_PLAYBOOK.md` update: each pass may run on the synthetic
  `mamut-seq` rig for development-time classification; the evidence class
  distinction (synthetic versus hardware) is stated explicitly
- optional `tools/run-seq-smoke.sh` documenting the two-terminal flow
  (Mamut in one, `mamut-seq` in the other); v1 stays two-process by design

### Acceptance

- every shipped scenario passes `validate` and plays end-to-end
- the gate-arm scenario audibly opens GFM on a real run and the Mamut-side
  `status` counters plus `--trace-midi` confirm the expected control stream
- one full synthetic pass per playbook section is documented

### Evidence

- proposed: `docs/EPM1_SEQ_V0.3_SCENARIO_LIBRARY_EVIDENCE.md`

### Review Gates

- `sel4-rust-systems-reviewer` (tool changes), `sel4-integrated-systems-
  reviewer` (playbook/docs update)

### Size

M

## SET3-4: Live TUI Mode

### Rationale

Scenario files cover repeatable work; a thin interactive mode covers ad-hoc
sound checking without walking to the hardware. This replaces the *dev-time*
role of the PC4 keybed, not its stage role.

### Design Sketch

`mamut-seq live` (ratatui):

- computer-keyboard piano (two-row layout, octave shift keys)
- sustain toggle, pitch-bend nudge, `program change 0..7` keys, panic key
- channel-aftertouch lane: keyboards have no pressure, so aftertouch is
  driven by ramp-up/hold/ramp-down keys with configurable slopes — enough to
  exercise the GFM aftertouch gate contract by hand
- CC lane: select a named control from the profile (`K1..K9`, `S1..S9`,
  `SW1..SW9`), then increment/decrement/set
- fire-scenario-from-live: trigger a scenario file while noodling

Honest terminal constraint, stated up front: plain terminals deliver no
key-release events. The v1 note model is therefore configurable gate length
plus a latch/toggle mode. Where the terminal supports the kitty keyboard
protocol, `crossterm` keyboard-enhancement flags enable true press/release —
detect it, use it, and show which mode is active in the UI. No claim of
piano-grade playability is made either way.

### Acceptance

- hold a chord, open GFM with the aftertouch ramp plus `K8`, and hear the
  layer bloom — laptop only, no hardware attached
- panic always exits clean: verified via `--trace-midi` and Mamut `status`
  (no stuck notes, sustain cleared)
- input handling is event-driven with a bounded tick — no busy-loop CPU burn
- keyboard-protocol capability (release events available or not) is
  reported in the UI and in the evidence doc

### Evidence

- proposed: `docs/EPM1_SEQ_V0.4_LIVE_MODE_EVIDENCE.md`

### Review Gates

- `sel4-rust-systems-reviewer`

### Size

M

## SET3-5: Docs And Workflow Integration

### Rationale

Kept as an explicit item so it is not dropped across slices; most of it lands
inside the acceptance of SET3-1..4.

### Deliverables

- `README.md` runtime notes: `mamut-seq` as the default development input,
  with the two-terminal quickstart
- `docs/README.md` index entries for the tool and its evidence docs
- playbook cross-links (SET3-3) verified
- `EPM1_FIRST_PERFORMANCE_PLAYBOOK.md` explicitly untouched: the stage path
  remains `PC4 -> mioXM -> EPM1`

### Acceptance

- a new contributor can go from clone to audible laptop-driven Mamut using
  only `README.md`

### Size

S

## Suggested Order And Dependencies

1. `SET3-1` → 2. `SET3-2` → 3. `SET3-3` → 4. `SET3-4` → 5. `SET3-5`
   (rolling)

Cross-set: `SET3-1..3` are recommended to land before or alongside Backlog
Set 1 and Set 2 work — every GFM acceptance pass in those sets becomes a
scripted scenario instead of a hardware session.

## Out Of Scope (v1)

- audio output or any synthesis inside the tool
- `.mid` file import/export (v2 candidate)
- network or Android transports (`docs/android-touch-controller.md` owns
  that direction)
- changes to `mamut-runtime`, the transport boundary, or MIDI ingress
- GUI (`egui`) surface — TUI only
- stage use: the PC4 remains the live instrument and `docs/live-sessions/`
  remains hardware-evidence-only

## Risks And Notes

- **Timing jitter**: userspace scheduling will jitter; mitigation is
  absolute-deadline scheduling plus measured-jitter reporting. Any
  evidence-grade timing claim carries its measured jitter.
- **Terminal key-release limits**: addressed honestly in SET3-4; latch/gate
  fallback is the baseline experience on plain terminals.
- **ALSA virtual port availability**: sandboxed or headless environments may
  restrict ALSA sequencer access; the evidence docs record the environment
  that was actually used.
- **Name collisions**: multiple concurrent `mamut-seq` instances suffix the
  port name with the PID.
