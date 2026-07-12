# EPM1 Backlog Set 4: MIDI 2.0 UMP Expressiveness (Deep Engine/DSP)

Date: 2026-07-06

Status: active backlog. `SET4-0` (ADR 0005) and `SET4-1` (spike evidence)
have landed; the rest is not started. Each item ships as its own slice with
its own evidence document and review gates. `SET4-9..12` (the touch-surface
track) were added 2026-07-06 as an addendum after the `SET4-1` decisions.

## Summary

Operator decisions (2026-07-06) — these are inputs to this backlog, not open
questions:

- **Scope**: UMP + MIDI 2.0 Channel Voice Messages go deep into the engine
  and DSP (per-note pitch/pressure/brightness, 16-bit velocity, 32-bit
  controllers, note attributes, sample-accurate timing). MIDI-CI (Discovery,
  Profiles, Property Exchange) is explicitly deferred to a future set.
- **Transport freeze**: lifted. A local ADR fully unfreezes the transport
  level of `mamut-sint-sw` for this work — "we can do whatever is needed at
  the transport level". The realtime discipline from ADR 0001 (bounded
  queues, no callback I/O, drop visibility) survives as a standing technical
  standard, not as a freeze artifact.
- **Sources**: `mamut-seq` becomes the primary programmable MIDI 2.0 (UMP)
  source, and an MPE bridge (MIDI 1.0 MPE controllers → the same per-note
  model) is in scope now. The `PC4 -> mioXM` MIDI 1.0 stage path is
  preserved with bit-identical behavior.
- **Timing**: JR-timestamp → intra-block sample-accurate scheduling is in
  scope.
- **Played source (addendum, 2026-07-06)**: the
  `pc4ms-touch-surface-android` app (sibling repo `pc4-microkit-studio`)
  becomes the *played* native UMP source — the human counterpart to
  `mamut-seq`'s deterministic one. Its `Mamut Instrument` mode moves from
  MIDI 1.0 over the USB gadget to native UMP over a UDP link (USB
  networking or Wi-Fi). App changes are in scope on the sibling-repo side;
  this backlog owns the mamut side of the contract (`SET4-9..12`).

The strategic shape: the internal event protocol becomes **UMP-first**
(MIDI 2.0-shaped). MIDI 1.0 and MPE become *translated* ingress paths into
that one internal model. Expressiveness lands as a per-voice expression
layer in the engine and DSP, routed by the patch.

Items:

1. `SET4-0` — ADR 0005: transport unfreeze plus MIDI 2.0 direction
2. `SET4-1` — ALSA UMP feasibility spike (the Rust path to UMP endpoints)
3. `SET4-2` — `mamut-midi2` leaf crate: UMP model, MIDI1→UMP translator,
   MPE mapper
4. `SET4-3` — engine event vocabulary v2 plus voice registry
5. `SET4-4` — per-voice expression state plus DSP hooks (the deep slice)
6. `SET4-5` — patch schema: per-note response and routing
7. `SET4-6` — runtime ingress v2: UMP backend, translated MIDI1/MPE,
   JR timestamps, trace widening
8. `SET4-7` — `mamut-seq` v0.5: UMP output plus per-note scenario language
9. `SET4-8` — evidence consolidation, stress, and docs truth
10. `SET4-9` — touch surface transport spike: UMP over UDP from Android
11. `SET4-10` — network UMP ingress, host side (bridge or runtime backend)
12. `SET4-11` — touch surface `Mamut Instrument` mode v2: native UMP
    (cross-repo)
13. `SET4-12` — touch surface end-to-end evidence and latency budget

## Assessment Inputs (what the code says today)

Findings from the 2026-07-06 migration assessment, anchored to source:

- **Internal values are already resolution-agnostic.** The MIDI 1.0 parser
  (`crates/mamut-runtime/src/audio_runtime/midi_parse.rs`) normalizes
  everything to `f32` at the wire; 7-bit quantization exists only at parse
  time. 32-bit MIDI 2.0 resolution is a parser change, not an internal
  value-model change.
- **Sample-accurate machinery exists but is unused.** `Engine::process_block`
  (`crates/mamut-engine/src/engine/process.rs`) applies events per frame and
  requires sorted `frame_offset`, but the runtime pins every event to
  `frame_offset: 0` (`audio_runtime/engine_thread.rs`), so effective timing
  is block-quantized (≈2.7 ms at 96 kHz / 256 frames).
- **All continuous control is channel-wide.** `ControlState`
  (`crates/mamut-engine/src/state.rs`) resolves to a single
  `DirectParameters` shared by every voice
  (`crates/mamut-engine/src/helpers.rs::resolve_direct_parameters`); pitch
  bend is added identically to every voice's frequency
  (`engine/macro_state.rs`). Per-note expression has no landing zone — this
  is the core migration surface.
- **Voices already carry rich per-voice DSP state** (oscillators, filter,
  envelopes, LFOs, deterministic seeds), so per-voice modulation targets
  exist naturally (`crates/mamut-engine/src/state.rs::VoiceState`).
- **Global resolve is per-event.** `refresh_resolved_state()` runs a full
  identity + 105-parameter resolve on every controller event; per-note
  event storms must not take that path.
- **MIDI 1.0 semantics are baked in a few places**: `NoteOn{velocity: 0}`
  acts as NoteOff inside the engine (`engine/events.rs`), bend range is
  baked into the parser, note-off velocity does not exist, polyphonic
  aftertouch is dropped, and the raw trace record stores only 4 bytes
  (`MIDI_TRACE_RAW_BYTES`) while UMP packets are up to 16.
- **Host is UMP-ready; the Rust layer is not.** Dev host: alsa-lib 1.2.16
  with `/usr/include/alsa/ump.h`, kernel 7.0. But `midir` 0.10.3 is MIDI 1.0
  byte-stream only, `alsa` 0.9.1 has no `ump` module, and `alsa-sys` 0.3.1
  ships pregenerated bindings with zero `snd_ump_*` symbols (verified in the
  vendored sources). This is the largest external risk and is addressed
  first (`SET4-1`).

Expressiveness capabilities this set delivers, end to end:

1. 16-bit note-on velocity and note-off (release) velocity into voice
   response curves.
2. Per-note pitch: note-on attribute Pitch 7.9, registered per-note
   controller 3 (absolute Pitch 7.25), and relative Per-Note Pitch Bend.
3. Per-note pressure (MIDI 2.0 poly pressure, 32-bit).
4. Per-note brightness (registered per-note controller 74), per-note
   expression, plus K assignable per-note lanes routed by the patch.
5. 32-bit channel controllers and JR-timestamp sample-accurate scheduling.

## Shared Boundary Constraints

- **Workspace lint posture is untouched**: `unsafe_code = "forbid"` stays.
  Any crate that must carry unsafe FFI (the ALSA UMP binding) lives
  *outside* the workspace members (vendored dependency or upstream
  contribution), never as a workspace crate. `mamut-midi2` itself is pure
  safe Rust.
- **Realtime discipline survives the unfreeze** (from ADR 0001, restated as
  standing rules): bounded queues only, no allocation/logging/formatting in
  the MIDI callback or render path, drop counters stay visible, priority
  actions (`panic`, `reset_controllers`) keep their bypass path.
- **Per-note events never trigger the global resolve.** They write per-voice
  expression lanes directly; channel events keep the existing
  `refresh_resolved_state` path. This is the CPU-discipline rule that makes
  32-bit per-note streams affordable.
- **PC4 stage path is regression-locked**: with a MIDI 1.0 controller
  profile and no MIDI 2.0 features engaged, the engine event stream for a
  recorded reference capture must be equivalent before/after (`SET4-6`
  acceptance).
- **Factory patches and the live set stay locked.** All patch-schema
  additions are optional blocks; absent block = today's behavior;
  `schema_version` stays 1.
- Docs and code must describe the same system truth in the same change
  (`CLAUDE.md`, `AGENTS.md`, `README.md`, `docs/README.md`).
- **EPM2 parity obligation**: the patch schema doc anchor lives in the
  sibling `mamut-sint-hw` docs track (`docs/patch-schema-v1.md`). The EPM2
  snapshot is currently not extracted in this lab; the parity pass
  (`hardware-software-pair-diff`) is a standing obligation scheduled for
  when it lands, and `SET4-5` records what it must cover.
- Review gates per `docs/review-gates.md` apply as listed per item.

## SET4-0: ADR 0005 — Transport Unfreeze Plus MIDI 2.0 Direction

### Rationale

`docs/EPM1_TRANSPORT_FREEZE.md` currently forbids exactly the work this set
needs (ingress vocabulary, queue payload, scheduling changes). The operator
decision is a full local unfreeze via ADR, so governance moves first and the
rest of the set works on unambiguous ground.

### Deliverables

- `docs/adrs/0005-midi2-ump-transport-unfreeze.md`:
  - rescinds the `EPM1_TRANSPORT_FREEZE.md` freeze for `mamut-sint-sw`
    (full transport unfreeze, operator-decided 2026-07-06)
  - ratifies the UMP-first internal protocol direction and the translated
    MIDI1/MPE ingress model
  - restates the ADR 0001 realtime rules as standing technical requirements
    independent of any freeze
  - records the deferred-MIDI-CI boundary
- `EPM1_TRANSPORT_FREEZE.md` gains a header: rescinded by ADR 0005, kept
  for history.
- `CLAUDE.md` (transport-freeze section) and `docs/README.md` posture
  lines updated to the new truth in the same change. (`AGENTS.md` was
  checked and carried no freeze-posture language; no change was needed
  there.)

### Acceptance

- A reader entering through any of the four docs gets one consistent story:
  transport work is open here for the MIDI 2.0 track; realtime rules still
  bind.

### Review Gates

- `sel4-integrated-systems-reviewer` (docs/architecture posture change)

### Size

S

## SET4-1: ALSA UMP Feasibility Spike (The Rust Path To UMP Endpoints)

### Rationale

Everything transport-side depends on getting UMP words in and out of ALSA
from Rust. The host stack is ready (alsa-lib 1.2.16, kernel 7.0); the Rust
crates are not (`midir` MIDI1-only; `alsa`/`alsa-sys` without `snd_ump_*`).
This spike de-risks the whole set before any production code is written.

### Design Sketch

Evaluate, in order of preference:

- **(A) Extend `alsa-rs`**: vendored fork (or upstream PR) adding the narrow
  UMP surface — `snd_seq_set_client_midi_version`
  (`SND_SEQ_CLIENT_UMP_MIDI_2_0`), UMP event I/O on the sequencer
  (`snd_seq_ump_event_t`), and virtual UMP client creation.
- **(B) Dedicated minimal sys crate** (non-workspace-member) binding only
  what we need, wrapped by a small safe API crate.
- **(C) Rawmidi UMP endpoints** (`snd_ump_open` path) as a fallback if the
  sequencer route stalls.

Also in the spike:

- Verify the ALSA sequencer auto-conversion trap: a client registered as
  legacy MIDI 1.0 receives silently *downconverted* (7-bit) data from UMP
  sources. Document why the native UMP client mode is mandatory, and how the
  evidence checks the client mode.
- Prove a **virtual UMP output client** (needed by `mamut-seq` in `SET4-7`).
- Decide the message-model layer: the `midi2` crate (no_std UMP types) as a
  dependency versus writing our own in `mamut-midi2`. Default posture is our
  own (matches the zero-dep leaf philosophy of `mamut-dsp`/`mamut-params`);
  the crate must earn its place if chosen.
- Kernel matrix: dev host 7.0 is fine; the `EPM1_RPI3B_MIOXM_HEADLESS.md`
  path needs a kernel ≥ 6.5 check recorded.

### Acceptance

- A throwaway spike binary (scratch, not a workspace member) receives a
  MIDI 2.0 NoteOn with 16-bit velocity from a UMP source (e.g. `aseqdump`
  against a virtual UMP client) and prints decoded fields.
- A virtual UMP output client is created and visible to ALSA tooling.
- The FFI route decision (A/B/C) and the message-model decision are
  recorded with the unsafe-surface inventory for the chosen route.

### Evidence

- proposed: `docs/EPM1_MIDI2_V0.1_ALSA_UMP_SPIKE_EVIDENCE.md`

### Review Gates

- `sel4-integrated-systems-reviewer` (dependency/architecture decision)

### Size

M

## SET4-2: `mamut-midi2` Leaf Crate — UMP Model, MIDI1→UMP Translator, MPE Mapper

### Rationale

One canonical, transport-free message layer: every ingress path (UMP wire,
MIDI 1.0 wire, MPE) converges to the same typed MIDI 2.0-shaped events
before anything downstream sees them.

### Design Sketch

New workspace member `crates/mamut-midi2` (pure safe Rust, no allocation in
steady state, no external deps unless the `SET4-1` decision says otherwise):

- **UMP packet layer**: 32-bit word framing; message types handled in v1:
  Utility (JR Clock / JR Timestamp), System, MIDI 1.0 CVM-in-UMP, MIDI 2.0
  CVM. Data 64/128, Flex Data, and UMP Stream messages are parsed-and-
  ignored with a typed `Ignored` verdict (no panics on any input).
- **Typed MIDI 2.0 CVM events**: NoteOn/NoteOff with 16-bit velocity and
  attribute (type 3 Pitch 7.9 handled; others carried opaque), Poly
  Pressure (32-bit), Control Change (32-bit), Registered/Assignable
  Controllers (RPN/NRPN), Registered/Assignable **Per-Note** Controllers,
  Per-Note Pitch Bend, Per-Note Management, Program Change, Channel
  Pressure, Pitch Bend (32-bit).
- **MIDI1→UMP translator** per the spec's default translation rules,
  including: NoteOn velocity 0 → NoteOff, 7→16/32-bit scaling rules, and
  RPN pair assembly. This is the path the existing `midir`/PC4 ingress will
  ride in `SET4-6`.
- **MPE mapper**: zone configuration plus per-member-channel state machine
  that folds member-channel pitch bend / channel pressure / CC74 into
  per-note pitch/pressure/brightness events on the zone's notes.
- Group/channel filtering helpers (the runtime keeps `--midi-channel`
  semantics).

Testing is table-driven with spec-derived vectors: round-trips, truncated
packets, reserved fields, and an equivalence suite proving a MIDI 1.0 byte
stream and its UMP upconversion yield identical typed event streams.

### Acceptance

- `cargo test --locked` covers the tables above; no reachable panic paths.
- Equivalence suite passes (MIDI1 direct vs upconverted).
- Crate compiles without `std`-only assumptions that would block a future
  embedded reuse (best effort; not a hard no_std gate in v1).
- `CLAUDE.md`, `AGENTS.md`, `README.md` crate lists updated in the same
  change.

### Evidence

- proposed: `docs/EPM1_MIDI2_V0.2_UMP_MODEL_EVIDENCE.md`

### Review Gates

- `sel4-integrated-systems-reviewer` (new workspace member)
- `sel4-rust-systems-reviewer` (crate code)

### Size

M–L

## SET4-3: Engine Event Vocabulary v2 Plus Voice Registry

### Rationale

`mamut-engine`'s API is the MIDI 1.0 bottleneck: `NoteEvent` has no note-off
velocity or attributes, `ControllerEvent` is channel-wide only. This slice
widens the vocabulary and gives per-note events a routing target, without
yet changing the sound.

### Design Sketch

- `NoteEvent::NoteOn { note, velocity, attribute: Option<NoteAttribute> }`
  (attribute v1: `Pitch7_9(f32 semitones)`);
  `NoteEvent::NoteOff { note, velocity }` (release velocity, default 0.5
  from translated MIDI 1.0).
- `ControllerEvent` gains per-note variants carrying `note: u8`:
  `PerNotePressure`, `PerNotePitchAbsolute` (RPNC 3, semitones),
  `PerNotePitchBend` (relative semitones), `PerNoteBrightness` (RPNC 74),
  `PerNoteExpression` (RPNC 11), `PerNoteAssignable { lane, value }`,
  `PerNoteManagement { detach, reset }`, plus channel `Rpn` handling for
  pitch-bend sensitivity.
- **Voice registry**: engine maps note → newest sounding instance for
  per-note routing (retrigger policy: per-note events target the newest
  instance with that note; note-off keeps the existing oldest-Held match).
  MPE/group/channel dimensions collapse to note space before the engine
  (runtime concern), keeping the engine single-timbre as today.
- **Semantics change, guarded**: a *native* MIDI 2.0 NoteOn with velocity 0
  is a valid quiet note-on (floor velocity), not a NoteOff — the engine
  guard in `handle_note_event` moves; MIDI 1.0 v0-as-off is now the
  translator's job (`SET4-2`), so PC4 behavior is unchanged.
- Per-note events route to voice expression lanes (introduced in `SET4-4`;
  in this slice they update registry-side state and are observable in
  snapshots, without DSP effect yet).

### Acceptance

- Unit tests: registry retrigger policy, per-note routing to the correct
  voice instance, velocity16 → `f32` precision, NoteOn-v0 semantics for
  native vs translated paths.
- Entire existing test suite passes unchanged (no sound change in this
  slice).

### Review Gates

- `sel4-rust-systems-reviewer`

### Size

M

## SET4-4: Per-Voice Expression State Plus DSP Hooks (The Deep Slice)

### Rationale

This is the payload of the whole set: per-note pitch, pressure, brightness,
expression, and assignable lanes actually modulating per-voice DSP. It is
also the largest and most hot-path-sensitive slice.

### Design Sketch

- `VoiceState` gains a `NoteExpression` block:
  `pitch_abs: Option<f32>` (attribute 7.9 at trigger, or RPNC 3 later),
  `per_note_bend_semitones`, `pressure`, `brightness`, `expression`,
  `assignable: [f32; K]` (v1 proposal: K = 2).
- Per-voice smoothing: a small per-lane smoother block in `mamut-dsp`
  (reusing `LinearSmoother` mechanics), windows ~2–6 ms per lane, chosen
  and documented in the DSP math companions.
- **Frequency path** (`engine/macro_state.rs::render_frame`): per-voice base
  becomes `pitch_abs.unwrap_or(note as f32) + per_note_bend + channel_bend`
  feeding the existing osc1/osc2/spectral/sub/additive frequency lanes
  (spread/detune/instability terms unchanged).
- **Filter path**: per-voice cutoff offset from brightness and pressure
  depths (multiplicative in log-frequency), optional per-voice resonance
  offset; per-voice level from expression; all bounded and clamped through
  the existing `ParamSpec` limits.
- **Velocity**: 16-bit velocity flows through `shaped_velocity_level` /
  `shaped_velocity_filter`; curves audited at high resolution. Note-off
  velocity scales effective release time within explicit bounds.
- **Depths and routing come from the patch** (`SET4-5`); until that lands,
  compiled defaults behind constants keep this slice testable.
- **CPU rule enforced here**: per-note events write voice lanes directly
  and never call `refresh_resolved_state`. Per-block cost is measured, not
  guessed.
- DSP math companions updated: `docs/dsp/render-path-math.md`,
  `docs/dsp/control-identity-math.md`.

### Acceptance

- New engine example (the documented evidence path), e.g.
  `per_note_expression_sweep`: two simultaneous voices where one bends and
  brightens independently of the other; rendered offline, metrics recorded.
- `core_output_safety_sweep` extended to cover per-note extremes and
  passing (no limiter regressions, no denormal storms).
- Measured per-block CPU cost before/after with a stated budget; the
  per-note storm case (6 voices × all lanes changing) stays within it.
- Existing factory bank renders bit-identically when no per-note events are
  present.

### Evidence

- proposed: `docs/EPM1_MIDI2_V0.3_PER_NOTE_DSP_EVIDENCE.md`

### Review Gates

- `sel4-rust-systems-reviewer`
- `sel4-rust-execution-optimizer` (render hot path)

### Size

L

## SET4-5: Patch Schema — Per-Note Response And Routing

### Rationale

"Programmable expressiveness" means the patch decides how per-note lanes
land in the DSP. This slice gives the patch model that vocabulary while
keeping every existing patch valid and untouched.

### Design Sketch

- Optional block `[performance_response.per_note]` on `PatchFileV1`:
  `brightness_to_cutoff`, `pressure_to_cutoff`, `pressure_to_level`,
  `expression_to_level`, `release_velocity_to_release`,
  `pitch_attribute_enabled`, per-note bend range for the relative lane.
- Optional array `[[performance_response.per_note.assignable]]`:
  `{ lane = 1..K, controller = <assignable PNC index>, target = <bounded
  enum>, depth, curve? }` where `target` is a fixed menu of per-voice
  destinations (cutoff offset, resonance offset, level, pulse width,
  additive tilt — final menu fixed in-slice and validated hard).
- Validation in `mamut-patch`: bounds via `mamut-params` specs where
  applicable; unknown targets or lanes are hard errors naming the field.
- `schema_version` stays 1: absent block = exactly today's behavior; old
  patches parse unchanged; factory bank untouched.
- Cross-repo note recorded: the sibling `mamut-sint-hw`
  `docs/patch-schema-v1.md` anchor needs a parity update when the EPM2
  snapshot next lands in the lab (standing `hardware-software-pair-diff`
  obligation; EPM2 is not currently on disk).

### Acceptance

- Validation test tables (good/bad) for the new blocks.
- The repo-local `toml-patch-schema` check runs clean over `patches/`.
- A dev patch (not in the factory bank) exercises every routing target in a
  dry-run.
- Old-binary behavior documented honestly: binaries older than this slice
  reject patches that use the new optional block (`deny_unknown_fields`);
  factory bank is unaffected.

### Review Gates

- `sel4-rust-systems-reviewer` (`mamut-patch` is a gated crate)
- `sel4-integrated-systems-reviewer` (schema surface)

### Size

M

## SET4-6: Runtime Ingress v2 — UMP Backend, Translated MIDI1/MPE, JR Timestamps, Trace Widening

### Rationale

The transport slice, now allowed by ADR 0005: native UMP in, MIDI 1.0 and
MPE translated to the same internal events, timestamps preserved to the
engine, and evidence machinery that can still see the wire.

### Design Sketch

- **Ingress backends**:
  - UMP: ALSA sequencer UMP client per the `SET4-1` route decision (native
    MIDI 2.0 protocol client; client mode verified at open).
  - MIDI 1.0: the existing `midir` path stays, but `parse_midi_message`
    shrinks to a thin wrapper over the `mamut-midi2` translator so both
    wires converge (channel bend → semitones conversion with the patch
    bend range stays at parse time; per-note pitch is absolute and needs no
    range).
  - MPE: opt-in zone configuration on the MIDI 1.0 path
    (`--mpe-zone <lower|upper>:<member-count>`), running the `mamut-midi2`
    MPE mapper.
- **Queue payload**: `RealtimeMidiMessage` widens to the v2 typed events
  plus a source timestamp. Bounded `ArrayQueue` discipline is unchanged;
  capacities (`MIDI_INPUT_QUEUE_CAPACITY` 512) revisited against measured
  per-note storm rates, not guessed.
- **JR timestamps → frame offsets**: `engine_thread` maps event timestamps
  onto `frame_offset` within the next rendered block against the block's
  timeline. Honesty rule: "sample-accurate" means accurate *relative to the
  engine block timeline*; absolute output latency (rtrb queue → ALSA) is
  unchanged by design and stated in the evidence.
- **Coalescing v2**: channel continuous keys as today; per-note lanes
  coalesce per `(note, lane)`; notes, sustain, and per-note management stay
  FIFO. New counters: `ump_packets_accepted`, `per_note_events`,
  `per_note_coalesced`, `translation_events`.
- **Trace widening**: `RawMidiTraceRecord` raw bytes 4 → 16 (full UMP
  packet), parsed verdict extended, trace worker decodes CVM2 into the
  sidecar; `MIDI_TRACE_QUEUE_CAPACITY` sizing revisited.
- **Priority path**: `panic`/`reset_controllers` bypass unchanged;
  `reset_controllers` additionally clears per-note expression lanes.
- ADR 0001 doc updated (superseded-in-part language pointing at ADR 0005).

### Acceptance

- A UMP source drives 16-bit velocity and per-note bend audibly end to end
  (until `SET4-7` lands, via the spike tool / `aseqdump`-visible virtual
  client).
- **PC4 regression bar**: for a recorded MIDI 1.0 reference capture with the
  locked profile, the engine event stream is equivalent before/after
  (trace A/B plus counters; no behavior change with MIDI 2.0 features
  disengaged).
- 96 kHz reference storm scenario: zero ALSA xruns, zero MIDI drops, zero
  runtime-control drops in no-trace mode on the dev host, counters recorded.
- MPE smoke: a scripted 3-member-channel MPE stream produces independent
  per-note bend/pressure on chord notes.

### Evidence

- proposed: `docs/EPM1_MIDI2_V0.4_INGRESS_EVIDENCE.md`

### Review Gates

- `sel4-rust-systems-reviewer`
- `sel4-rust-execution-optimizer` (callback/queue hot paths)
- `sel4-integrated-systems-reviewer` (ADR 0001 truth update)

### Size

L

## SET4-7: `mamut-seq` v0.5 — UMP Output Plus Per-Note Scenario Language

### Rationale

The operator-decided primary MIDI 2.0 source: deterministic, programmable
per-note gestures as text files — the same role `mamut-seq` already plays
for MIDI 1.0, extended to the full expressiveness surface. This is what
makes the new engine capabilities *provable* without new hardware.

### Design Sketch

- UMP virtual output client backend (route per `SET4-1`); the existing
  `midir` MIDI 1.0 backend stays for legacy scenarios.
- Scenario schema v2 additions: `velocity16`, note attribute pitch,
  per-note lanes (pitch-bend curves, pressure envelopes, brightness ramps,
  assignable PNC ramps), 32-bit CC values, and an `emit` selector:
  `midi1 | ump | mpe` — one musical text, three wire renderings, so the
  three ingress paths can be A/B'd against each other.
- JR timestamp emission at send time; determinism story unchanged
  (expansion stays a pure function; wall-clock jitter measured and
  reported, never claimed away).

### Acceptance

- The same scenario emitted as `midi1` and as `ump` produces equivalent
  engine event streams for the overlapping feature set (trace compare).
- A per-note scenario audibly demonstrates two chord notes bending and
  brightening independently — laptop only, no hardware attached.
- `validate` prints the expanded per-note schedule; determinism test holds
  across emit modes.

### Evidence

- proposed: `docs/EPM1_SEQ_V0.5_UMP_SOURCE_EVIDENCE.md`

### Review Gates

- `sel4-rust-systems-reviewer`

### Size

M–L

## SET4-8: Evidence Consolidation, Stress, And Docs Truth

### Rationale

Kept as an explicit item so honesty work is not dropped across slices; most
content lands inside `SET4-1..7` acceptance, this item closes the loop.

### Deliverables

- MIDI 2.0 stress evidence: per-note storm at 96 kHz (6 voices × all
  lanes), counters, xruns, measured per-block CPU versus the `SET4-4`
  budget.
- A/B musical rendering: one factory patch untouched (bit-identical MIDI
  1.0 path) and one dev patch with per-note response, same scenario in
  `midi1` vs `ump` emit — the audible expressiveness delta is the artifact.
- `README.md` runtime notes (UMP device selection, `--mpe-zone`, new
  counters in `status`), `docs/README.md` index entries for this set's
  evidence docs, `mamut-epm-program-map.md` note that `EPM1` speaks
  MIDI 2.0 natively.
- `EPM1_PC4_LIVE_PROFILE.md` explicitly untouched: the stage path remains
  MIDI 1.0 and regression-locked.
- Optional stretch (may be cut without ceremony): read-only per-note
  expression lanes in the `INSPECT` screen.

### Acceptance

- A new contributor can go from clone to an audible per-note bend demo
  (laptop only) using `README.md` alone.
- `sel4-integrated-systems-reviewer` docs-truth pass over the whole set.

### Size

M

## SET4-9: Touch Surface Transport Spike — UMP Over UDP From Android

### Rationale

The `pc4ms-touch-surface-android` `Mamut Instrument` mode has the right
shape already (isomorphic B-griff lattice, per-finger tracking,
pure-function MIDI encoders with JVM unit tests) but speaks MIDI 1.0 over
the Android USB gadget. There is no native USB path to MIDI 2.0 from a
stock tablet: Android's peripheral-mode gadget is the MIDI 1.0 class
function (`f_midi`; the kernel's `f_midi2` UMP gadget is not exposed by
stock ROMs), the platform UMP APIs (`TRANSPORT_UNIVERSAL_MIDI_PACKETS`,
API 33; `MidiUmpDeviceService`, API 35) cover host-mode and intra-device
routing only, and BLE MIDI has no UMP binding. UMP over UDP is the
ROM-independent native route — the app needs no Android MIDI API at all,
and the same link reaches the RPi3B rig with no USB-gadget questions. This
spike de-risks framing, link, and receiver placement the way `SET4-1`
de-risked the ALSA route.

### Design Sketch

- **Framing, evaluated in the spike** (decision recorded like `SET4-1`'s
  route decision):
  - **(A) MIDI Association Network MIDI 2.0 (UDP) transport** core:
    standard session protocol (invitation, ping, retransmit, UMP data),
    interoperable with future tooling; more protocol surface.
  - **(B) Minimal custom framing (`net-ump v0`)**: sequence number plus
    raw UMP words per datagram; smallest possible, point-to-point only,
    explicitly non-interoperable. Default posture per the zero-dep
    philosophy: start at (B), record exactly what (A) would add, decide on
    the evidence.
- **Links measured, both on the actual tablet**: USB networking (NCM/RNDIS
  tethering — the tablet's default USB mode, point-to-point) and Wi-Fi.
  Jitter measured clock-independently host-side (arrival deltas of a
  fixed-cadence stream); latency as UDP echo RTT/2 with the clock-honesty
  caveat stated. Loss counted over a sustained per-note-storm-like cadence.
- **Receiver**: throwaway Rust bridge (scratch, not a workspace member)
  reusing the `SET4-1` FFI inventory: UDP socket → virtual ALSA seq UMP
  endpoint, subscription model per the `SET4-1` EPERM finding, negotiated
  client mode read back and printed per the standing downconversion rule.
- **Sender**: throwaway on-tablet sender (scratch app build). A laptop-side
  sender may bootstrap the bridge, but acceptance requires the actual
  tablet — the spike exists to measure the real link.
- **Loss/hung-note posture sketched** (designed properly in `SET4-10`):
  sequence gaps must be detectable at the framing level; a lost note-off
  must have a safety story.
- **Kernel note**: the bridge route requires host-kernel ALSA UMP (dev
  host: fine). If the RPi3B rig kernel stays < 6.5 (open rig check from
  `SET4-1`), the `SET4-10` decision tilts toward direct runtime ingress,
  which has no kernel dependency.

### Acceptance

- A MIDI 2.0 NoteOn with 16-bit velocity sent from the actual tablet
  arrives bit-exact in `aseqdump -u 2` through the bridge, over **both**
  links (USB networking and Wi-Fi).
- Latency/jitter/loss table for both links recorded under the sustained
  cadence, with the measurement-honesty caveats stated.
- Framing decision (A/B) and the receiver-placement recommendation for
  `SET4-10` recorded with rationale.

### Evidence

- proposed: `docs/EPM1_MIDI2_V0.5_NET_UMP_SPIKE_EVIDENCE.md` (the
  `EPM1_MIDI2_V0.x` series is independent of the `EPM1_SEQ_V0.x` series —
  the `SET4-7` doc's `V0.5` names `mamut-seq`'s own version line)

### Review Gates

- `sel4-integrated-systems-reviewer` (transport/architecture decision)

### Size

S–M

## SET4-10: Network UMP Ingress — Host Side

### Rationale

The host-side landing for the touch surface (and any future network UMP
source), per the `SET4-9` decisions. Two placements are on the table; the
default posture mirrors `mamut-seq`: a leaf tool that makes zero runtime
changes by construction.

### Design Sketch

- **(A) Leaf bridge binary `mamut-net-ump`** (default): UDP in → virtual
  ALSA seq UMP client out. Mamut is untouched — the runtime sees one more
  UMP source through the `SET4-6` backend, and the bridge composes with
  `aseqdump`, `mamut-seq`, and any other UMP consumer. Follows the
  `mamut-seq` crate posture (leaf binary, no mamut-crate dependencies; the
  safe ALSA-UMP wrapper from the `SET4-1` route B surface arrives as the
  same non-workspace dependency that `SET4-6`/`SET4-7` use). Depends on
  host-kernel ALSA UMP.
- **(B) Runtime ingress backend** (`--midi-net-ump <bind>` in
  `mamut-runtime`): no kernel dependency; network code enters the product
  runtime. Chosen only if the rig kernel forces it (`SET4-9` kernel note).
  If chosen: socket reader on a worker thread, feeding the same
  bounded-queue/typed-event path as the other `SET4-6` backends, no
  allocation on the forwarding path in steady state.
- Either way:
  - **Loss posture**: sequence-gap detection with a visible counter; a
    detected gap while notes are sounding triggers the note-off safety
    (per-note management / all-notes-off equivalent), so a dropped
    note-off cannot hang a voice indefinitely. Link death (silence
    timeout) does the same.
  - **Counters**: `net_ump_packets_accepted`, `net_ump_packets_dropped`,
    `net_ump_seq_gaps`.
  - **Exposure**: binds localhost/link-local by default; any wider bind is
    an explicit flag. This is a trusted point-to-point instrument link,
    not a network service — stated in `--help` and docs.

### Acceptance

- The tablet (or the `SET4-9` scratch sender) audibly drives the engine
  end to end through the chosen path and the `SET4-6` ingress; negotiated
  client mode printed per the standing rule.
- The `SET4-9` storm cadence runs with zero drops and zero sequence gaps
  on the USB networking link; counters visible in `status` (route B) or
  the bridge's own output (route A).
- Mid-note link kill (cable unplug) releases all sounding voices via the
  loss posture within a stated bound.
- Route A adds a workspace binary: `CLAUDE.md`, `AGENTS.md`, `README.md`
  crate/binary lists updated in the same change (the "three binaries with
  a `main`" line becomes four).

### Evidence

- proposed: `docs/EPM1_MIDI2_V0.6_NET_UMP_INGRESS_EVIDENCE.md`

### Review Gates

- `sel4-rust-systems-reviewer`
- `sel4-rust-execution-optimizer` (only if route B touches `mamut-runtime`
  hot paths)
- `sel4-integrated-systems-reviewer` (new binary/boundary, docs truth)

### Size

M

## SET4-11: Touch Surface `Mamut Instrument` Mode v2 — Native UMP (Cross-Repo)

### Rationale

What MIDI 1.0 takes away from the lattice is exactly what this set builds:
today the app sends *channel* aftertouch (one lane shared by all fingers)
and 7-bit velocity. Native UMP gives it per-finger pressure, 16-bit
velocity, and per-note bend — the playable counterpart of the `SET4-4`
engine work.

### Design Sketch

Cross-repo split: this backlog owns the mamut side of the contract; the
Kotlin implementation lives in `pc4-microkit-studio` under that repo's
conventions, tracked in the app's own README/plan with a reference back to
this slice (creating that sibling-repo tracking entry is part of this
slice, so the app work cannot be orphaned by a backlog that cannot gate
it).

- **Mamut-side deliverable — the contract**: golden UMP test vectors
  exported from the `mamut-midi2` (`SET4-2`) test tables as a checked-in
  fixture (format decided in-slice; e.g. JSON with description, decoded
  fields, and expected 32-bit words). The app's Kotlin encoder unit tests
  consume the same fixture — a cross-implementation contract that keeps
  two repos honest without sharing code.
- **App-side sketch (recorded here as the contract, implemented there)**:
  - Pure-Kotlin UMP encoder module beside `MidiEncoding.kt`, same
    pure-function + JVM-unit-test discipline, validated against the golden
    vectors.
  - `Mamut Instrument` mode gains a wire selector: `MIDI 1.0 (USB)`
    (unchanged; the regression baseline) | `UMP (network)` per the
    `SET4-9` framing.
  - Mapping v1: 16-bit velocity from the existing Y-pos/touch-size/fixed
    modes; per-finger vertical slide → **per-note** pressure (32-bit poly
    pressure), replacing the global channel-AT compromise; macros CC16–20
    and expression CC11 as 32-bit CC; program change 0–7 unchanged (the
    factory live set stays locked); panic maps to the per-note equivalent
    plus the existing backstops.
  - Decided in-slice with the operator (musical semantics): per-note bend
    source (horizontal in-button slide?), brightness (PNC 74) source, and
    whether the lattice engages the Pitch 7.9 note attribute (12-EDO stays
    the default; the attribute would make the lattice microtonal-capable
    with no engine changes).

### Acceptance

- The golden-vector fixture lands in this repo with a contract note naming
  the consuming app tests; `mamut-midi2` tests prove the fixture is
  generated from (not maintained parallel to) its own tables.
- App-side: Kotlin encoder tests pass against the fixture; UMP mode sends
  the mapping v1 surface; the MIDI 1.0 mode is regression-clean (existing
  encoders and tests untouched and passing). Verified from this repo's
  side in `SET4-12`.

### Evidence

- mamut side: the fixture plus a contract note (location decided in-slice;
  e.g. `docs/EPM1_MIDI2_TOUCH_SURFACE_CONTRACT.md`). The joint end-to-end
  evidence is `SET4-12`'s deliverable.

### Review Gates

- `sel4-rust-systems-reviewer` (if `mamut-midi2` grows an export path)
- `sel4-integrated-systems-reviewer` (cross-repo contract doc)
- Kotlin code is reviewed under `pc4-microkit-studio` conventions, not
  this repo's gates.

### Size

M (app side) + S (mamut side)

## SET4-12: Touch Surface End-To-End Evidence And Latency Budget

### Rationale

Closes the touch-surface track the way `SET4-8` closes the core set: the
played source is only real when a two-finger gesture on the tablet audibly
moves two voices independently, with the link characterized and the
standing trap checks green.

### Deliverables

- Full-chain runs: tablet (`SET4-11` app) → both links (USB networking,
  Wi-Fi) → `SET4-10` path → `SET4-6` ingress → engine trace.
- A/B against `mamut-seq` `ump` emit for an equivalent scripted gesture:
  engine event streams equivalent for the overlapping feature set (the
  same bar `SET4-7` sets for midi1-vs-ump).
- The expressiveness artifact: two held lattice notes with independent
  per-finger pressure/bend, audibly independent — the human counterpart of
  `SET4-4`'s offline example.
- Latency/jitter table per link at performance cadence; `SET4-6` honesty
  rules apply (block-relative accuracy; absolute latency stated, never
  claimed away).
- Standing trap checks in every run: negotiated client mode printed;
  downconversion canary (a legacy client sees 7-bit while the native path
  sees 16-bit).
- Hung-note safety demonstrated live: cable unplug mid-chord releases
  voices within the `SET4-10` bound.
- Docs truth: `README.md` gains the touch-surface quickstart note;
  `docs/README.md` indexes the new evidence docs; the app README's mamut
  section is updated in the sibling repo in the same change.

### Acceptance

- A new contributor with the tablet, one cable, and `README.md` alone gets
  from clone to the two-finger independence demo.
- All checks above recorded in the evidence doc with pasted outputs.

### Evidence

- proposed: `docs/EPM1_MIDI2_V0.7_TOUCH_SURFACE_E2E_EVIDENCE.md`

### Review Gates

- `sel4-integrated-systems-reviewer` (evidence/docs truth)

### Size

S–M

## Suggested Order And Dependencies

1. `SET4-0` (governance unlock — everything else assumes it)
2. `SET4-1` (spike; its decisions feed 2, 6, 7)
3. `SET4-2` → 4. `SET4-3` → 5. `SET4-4` → 6. `SET4-5`
   (engine chain; `SET4-5` may overlap `SET4-4` once lane semantics are
   fixed; none of it needs working UMP transport — `dry-run`, engine
   examples, and translated MIDI 1.0 carry the proofs)
7. `SET4-6` (needs `SET4-1..3`; unlocks native UMP end to end)
8. `SET4-7` (needs `SET4-1..2`; full end-to-end proof needs `SET4-6`)
9. `SET4-8` (rolling; closes after 6 and 7)
10. `SET4-9` (any time after `SET4-1`; parallel to the engine chain)
11. `SET4-10` (needs `SET4-9`; route A can land before `SET4-6`, but the
    audible acceptance needs `SET4-6`)
12. `SET4-11` (needs the `SET4-2` vectors and the `SET4-9` framing; the
    app work proceeds in the sibling repo)
13. `SET4-12` (needs `SET4-4`, `SET4-6`, `SET4-10`, `SET4-11`)

The engine chain (`SET4-2..5`) and the transport track (`SET4-1`, then
`SET4-6`) can proceed in parallel after the spike decision. The
touch-surface track (`SET4-9..12`) is additive: it rides the same ingress
(`SET4-6`) and engine (`SET4-3..5`) work and adds no new engine scope —
`SET4-9` is the only piece worth starting early, because its link
measurements and kernel note steer `SET4-10`.

## Out Of Scope (This Set)

- MIDI-CI: Discovery, Profiles, Property Exchange (future set; the
  `mamut-params` registry is already PE-friendly when that day comes)
- Flex Data, UMP Stream configuration, SysEx7/8 payloads beyond
  parse-and-ignore
- GUI *editing* surfaces for per-note routing (INSPECT read-only lanes are
  the only optional GUI touch, in `SET4-8`)
- `.mid` / SMF2 clip file support in `mamut-seq`
- BLE MIDI transport (no UMP binding exists). Network UMP moved *into*
  scope on 2026-07-06 via the `SET4-9..12` addendum, but only as the
  point-to-point touch-surface link — no general RTP-MIDI service, no
  discovery/mDNS unless the `SET4-9` framing decision lands on the
  standard transport
- Android app scope beyond the `Mamut Instrument` mode v2 contract
  (the app's PC4/Workbench modes are untouched by this set)
- plugin/editor track (stays deferred per ADR 0004 posture)
- EPM2 implementation parity (standing obligation recorded in `SET4-5`;
  executed when the EPM2 snapshot is back in the lab)

## Risks And Notes

- **alsa-rs UMP gap (largest risk)**: no `snd_ump_*` in the Rust binding
  today. Mitigation: `SET4-1` first, three routes evaluated, unsafe surface
  quarantined outside the workspace; upstreaming preferred long-term.
- **Sequencer auto-conversion trap**: a legacy-registered client silently
  receives 7-bit downconversions from UMP sources — evidence must verify
  the client's negotiated mode, or the whole set can "pass" at MIDI 1.0
  resolution without anyone noticing.
- **CPU under per-note storms**: the per-note path bypasses the global
  resolve and coalesces per `(note, lane)`; the `SET4-4` budget measurement
  is the enforcement point, and `controllers_coalesced`-family counters
  keep losses visible.
- **Forward compatibility**: `deny_unknown_fields` means binaries older
  than `SET4-5` reject patches using the per-note block. Accepted: factory
  bank stays clean; dev patches carry the new blocks.
- **NoteOn velocity-0 semantics** differ between MIDI 1.0 (= NoteOff) and
  native MIDI 2.0 (= quiet note-on): owned entirely by the `SET4-2`
  translator; engine tests pin both behaviors.
- **Timestamp honesty**: JR-based scheduling is sample-accurate relative to
  the engine block timeline; absolute latency is unchanged. Evidence docs
  state this explicitly.
- **`midi2` crate versus own model**: decided in `SET4-1`; default is own
  code per the zero-dep leaf philosophy — an external message-model crate
  must earn its place.
- **RPi3B headless path**: needs kernel ≥ 6.5 for ALSA UMP; checked and
  recorded on the rig, not assumed (open check carried from `SET4-1`). The
  result steers the `SET4-10` placement: an old rig kernel forces route B
  (direct runtime ingress), which needs no kernel UMP support.
- **Wi-Fi jitter on the played path**: the untethered link may not meet
  performance feel. USB networking is the primary stage posture and Wi-Fi
  the convenience mode; `SET4-9` measures instead of guessing, and JR
  timestamps (`SET4-6`) absorb what the budget allows.
- **Custom framing lock-in**: `net-ump v0`, if chosen in `SET4-9`, is
  deliberately non-interoperable. The decision record must state what
  upgrading to the MIDI Association UDP transport would touch, so the
  standard stays an exit, not a rewrite.
- **Cross-repo drift**: the app and `mamut-midi2` can silently diverge.
  The golden-vector fixture (`SET4-11`) is the contract and `SET4-12`
  re-runs it end to end; app-side changes do not gate this repo's CI —
  accepted, with the fixture as the tripwire.
- **UDP ingress exposure**: a UMP-over-UDP listener is attack surface. The
  default bind stays localhost/link-local and any wider bind is a
  deliberate flag (`SET4-10`); the link is a trusted point-to-point
  instrument cable in posture, never a service.
- **Voice count**: 6 voices keep per-voice expression state cost trivial;
  the design does not preclude raising `voice_count`, but no such change is
  part of this set.
