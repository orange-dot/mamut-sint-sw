# ADR 0005: MIDI 2.0 UMP Direction And Transport Unfreeze

Status: Accepted

Date: 2026-07-06

## Context

`EPM1` has been under the local transport freeze recorded in
`docs/EPM1_TRANSPORT_FREEZE.md`: no direct redesign of the standalone
transport boundary, no queue-architecture experimentation, and a resume
point conditioned on the shared `mamut-platform` Linux audio platform
track producing a documentation-complete `v1` transport direction first.
ADR 0001 (`docs/adrs/0001-standalone-midi-ingress-hardening.md`) worked
inside that freeze and closed with the same escape clause: the next
transport decision belongs to the shared platform track.

On 2026-07-06 the operator decided to migrate `EPM1` to MIDI 2.0 with
per-note expressiveness deep in the engine and DSP. The decision set is
recorded in `docs/EPM1_BACKLOG_SET4_MIDI2_UMP_EXPRESSIVENESS.md`
(Backlog Set 4): UMP + MIDI 2.0 Channel Voice Messages into the engine
and DSP, MIDI-CI deferred; `mamut-seq` as the primary programmable UMP
source plus an MPE bridge; JR-timestamp sample-accurate scheduling in
scope; and — decisive for this ADR — a **full local transport
unfreeze**, in the operator's words: transport-level work may do
whatever the MIDI 2.0 track needs.

That work necessarily lands in the frozen zone. The 2026-07-06
assessment (summarized in the Backlog Set 4 `Assessment Inputs`
section) shows why: the MIDI ingress vocabulary
(`crates/mamut-runtime/src/types/`), the bounded queue payloads, the
`frame_offset: 0` pinning in
`crates/mamut-runtime/src/audio_runtime/engine_thread.rs`, and the
4-byte trace records are all transport-level surfaces that MIDI 2.0
must widen. Waiting on the `mamut-platform` resume conditions would
block the MIDI 2.0 track indefinitely: that docs track has not produced
the `v1` transport direction, and no implementation home exists yet.

The freeze rationale has therefore inverted. The freeze existed so that
`EPM1` would stop being the place where transport architecture evolves
ad hoc; the MIDI 2.0 track is a deliberate, backlog-governed,
review-gated program of transport evolution with `EPM1` as its proving
ground.

## Decision

`EPM1` adopts the MIDI 2.0 UMP track locally, and as the enabling
governance act this ADR **rescinds the transport freeze** in
`docs/EPM1_TRANSPORT_FREEZE.md` for `mamut-sint-sw`.

Concretely:

- **UMP-first internal protocol.** The internal event vocabulary
  becomes MIDI 2.0 Channel-Voice-shaped (16-bit velocity, note
  attributes, per-note controllers, 32-bit channel controllers).
  MIDI 1.0 and MPE become *translated* ingress paths into that one
  internal model via the planned `mamut-midi2` leaf crate. There is one
  internal protocol; wires converge to it.
- **Transport-level work is open locally**, scoped by Backlog Set 4:
  ingress vocabulary widening, queue payload changes (typed v2 events
  plus source timestamps), JR-timestamp → intra-block `frame_offset`
  scheduling, trace-record widening (4 → 16 raw bytes), queue-capacity
  retuning against measured rates, and a native ALSA UMP ingress
  backend.
- **The ADR 0001 realtime rules survive the freeze as standing
  technical requirements**, independent of any freeze document: bounded
  queues only; no allocation, file I/O, logging, or string formatting in
  the MIDI callback or render path; drop counters stay visible
  (`midi_messages_dropped`, `runtime_controls_dropped`,
  `trace_records_dropped`, coalescing counters); `panic` and
  `reset_controllers` keep their priority bypass path. ADR 0001 remains
  Accepted; only its closing escape clause (route the next transport
  decision to the shared platform track) is superseded in part by this
  ADR for the MIDI 2.0 scope.
- **MIDI-CI is out of scope** for this track (Discovery, Profiles,
  Property Exchange). The boundary is recorded in Backlog Set 4; a
  future set may reopen it.
- **Relationship to `mamut-platform`**: `EPM1` resumes ownership of its
  own transport for this track and does not wait on the platform docs
  track. Findings from the MIDI 2.0 work may inform that track later;
  no write-back commitment is part of this ADR.

Unchanged constraints, restated so the unfreeze is not misread:

- The GUI-track boundary from ADR 0004 stands: GUI work still does not
  touch the transport boundary, the audio callback, the MIDI ingress
  callback, or the voice allocator.
- The factory bank and the locked 8-slot live set stay locked; patch
  schema growth in this track is optional-block-only,
  `schema_version = 1`.
- The `PC4 -> mioXM` MIDI 1.0 stage path is regression-locked: with no
  MIDI 2.0 features engaged, the engine event stream for a recorded
  reference capture must be equivalent before and after the ingress
  changes (Backlog Set 4 `SET4-6` acceptance).
- Transport changes ride Backlog Set 4 slices with the mandated review
  gates (`docs/review-gates.md`); the unfreeze is not a license for
  unbounded experimentation outside that backlog.

## Consequences

`docs/EPM1_TRANSPORT_FREEZE.md` is marked rescinded by this ADR and
preserved for history; its resume-point conditions no longer bind.
`CLAUDE.md`, `README.md`, and `docs/README.md` posture language is
updated to the new truth in the same change as this ADR.

Backlog Set 4 items `SET4-1..8` may proceed. The engine chain
(`SET4-2..5`) was never freeze-blocked; the transport track (`SET4-1`,
`SET4-6`, `SET4-7`) becomes legitimate local work.

What becomes possible: a native ALSA UMP ingress backend, widened
`RealtimeMidiMessage` payloads with source timestamps, sample-accurate
event scheduling against the engine block timeline, 16-byte raw trace
records, and measured queue-capacity changes.

What stays forbidden: unbounded queues, allocation or I/O on the MIDI
callback and render paths, silent event loss (drop counters are part of
the contract), GUI access to transport internals, factory-bank or
live-set changes, and transport work that bypasses the Set 4 review
gates.

Reversibility: rescinding this direction before `SET4-6` lands is
cheap — reinstate a freeze via a new ADR; the engine-side event
vocabulary work stands on its own. After `SET4-6`/`SET4-7` land, the
translated-ingress model and timestamped queue payloads become
structural, and unwinding means re-narrowing the ingress to MIDI 1.0
semantics — possible, but a real regression project, not a revert.

ADR 0001 is amended in part (escape clause only), not superseded: its
hardening decisions — bounded `ArrayQueue` ingress, trace-worker
offload, priority-action bypass, coalescing with visible counters —
remain the baseline that Backlog Set 4 extends rather than replaces.
`mamut-tui`, `mamit-sint-hw`, and the `mamut-platform` docs track are
not modified by this ADR.
