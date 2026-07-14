# ADR 0006: Tonewheel Generator Track — Component Model, Global Source, Naming Policy

Status: Accepted

Date: 2026-07-14

## Context

On 2026-07-14 the operator decided to add a credible tonewheel organ to
`EPM1`, with the bar set by ear against reference recordings of canonical
tonewheel-organ performances, not by "an organ preset" standard. The
decision set and the twelve-slice plan are recorded in
`docs/EPM1_BACKLOG_SET6_TONEWHEEL_GENERATOR.md` (Backlog Set 6); this ADR
records the architecture decisions that backlog builds on, and the
alternatives that were rejected.

The instrument family being modeled is electromechanical: a bank of
continuously rotating tonewheels driven through a fixed gear train, a
passive key-contact/busbar network, a mechanical vibrato scanner, a tube
preamp, and a two-rotor rotating speaker cabinet. "Physical modeling"
for this family therefore means modeling those components — not
waveguides, and not full circuit simulation.

The engine today has per-voice sources only (`osc1`/`osc2`/sub/noise/
spectral/additive/mozaik, summed per voice in
`crates/mamut-engine/src/engine/macro_state.rs`) plus engine-global
*layers* (GFM/BCS, applied post-mix at `macro_state.rs:345-346`). There
is no engine-global sound *source*. The nearest existing relative — the
per-voice additive bank (`additive_partials: [Oscillator; 8]`,
`crates/mamut-engine/src/state.rs:463`) — is what makes the central
question concrete: extend the per-voice additive apparatus, or build a
shared generator?

## Decision

### 1. Component model with one global shared generator

`EPM1` gets a component model of the machine: a single engine-global
generator of 91 continuously running wheels (frequencies from the
twelve-ratio gear table, not from equal temperament), a 61-key
contact/busbar model, single-trigger percussion, a dispersive
vibrato/chorus scanner, a preamp drive stage, and a two-rotor rotary
speaker. Circuit-level simulation (wave-digital tube stages, solved
resistor networks) is out of scope; a single component may earn that
upgrade later only if the calibration harness and the operator's ear
reject the component-model stage.

The generator is global because five audible behaviors are properties of
the shared machine, not of any per-voice structure:

1. **Shared-wheel phase coherence** — two held keys whose drawbar taps
   land on the same wheel reinforce exactly instead of chorusing.
2. **Loudness robbing** — passive bus summing redistributes level as
   keys are added.
3. **Key click** — emerges from switching a live, running signal
   mid-cycle through bouncing contacts.
4. **Gear-ratio tuning** — the slow near-equal-temperament beat patterns
   between drawbars are fixed properties of the gear table.
5. **Leakage** — faint bleed of wheels that are not keyed.

**Rejected: extending the per-voice additive bank.** A per-voice
nine-partial organ reproduces none of the five behaviors: unison taps
become independent detuned oscillators that beat against each other,
robbing has to be faked, the click has to be a synthetic noise burst
because per-voice phases restart, the gear-table beat signature
disappears into per-voice detune, and unkeyed wheels do not exist to
leak. The per-voice route is cheaper to integrate and was rejected on
fidelity, not on cost — the shared generator's steady-state cost
(91 phase accumulators plus three 91-wide dot products) is itself small
and constant.

### 2. Chain position

The organ bus is
`generator → contact/bus gains → percussion join → vibrato scanner →
preamp drive → rotary speaker`, mono until the rotary stage. It joins
the main mix after the per-voice loop and before the final
body/mid-side stage (`macro_state.rs:302`), so the existing final
saturation, DC blocker, and safety limiter keep governing the output.
Chorus/reverb remain available as a shared room; GFM/BCS can process
the organ like any other program material (the identity twist), with
organ demos keeping them off by default. Note events reach the organ's
contact model in parallel with the voice allocator — the same scheduled
events at the same frame offsets — so the allocator is untouched and
organ polyphony is the full 61-key compass by construction.

### 3. Control doctrine

All Set 6 controls are session-only (engine setter plus headless
command), per the Set 5 ladder: no patch-schema growth,
`schema_version` stays 1, the factory bank and locked live set do not
change. The five-controls-per-concept cap holds, with the nine-drawbar
registration counting as **one vector control** in the family's
canonical nine-digit notation (`888000000`), set atomically. Tonewheel
concept: `registration`, `percussion`, `vibrato`, `drive`, `wear`.
Rotary concept: `mode`, `balance`, `width`, `drive`.

**Rejected: patch-schema-first controls.** Growing
`[engine.tonewheel]`/`[engine.fx.rotary]` schema tables before the
sound exists would lock a contract around numbers that the calibration
work is expected to move, and it contradicts the Set 5 doctrine that a
contract surface is earned by a separate contract set after the
evidence ladder completes (the note-strike precedent). Patch schema,
controller-profile drawbar bindings (a nine-fader surface maps
naturally), GUI exposure, and `EPM2` parity are deferred follow-ups
named in the backlog.

### 4. Naming and trademark policy (binding beyond Set 6)

This repo names no manufacturer and no model of the emulated instrument
family — not in docs, code, identifiers, params, patch names, UI
strings, or commit messages. The vocabulary is generic: *tonewheel
generator*, *drawbars*, *registration*, *foldback*, *key click*,
*percussion*, *vibrato scanner*, *rotary speaker*. External works are
cited in `docs/EXTERNAL_DSP_REFERENCES.md` by author/venue/year plus a
link; where a cited work's verbatim title contains a protected name,
the citation uses the author-venue-year form instead of quoting the
title. The repo has zero such occurrences today and this policy keeps
it that way, in this set and after it.

### 5. Calibration strategy

Credibility is measured, not asserted: the operator provides reference
recordings before implementation begins (intake is `SET6-1`; the
recordings are being assembled now). They never enter the repo
(copyright); they live in an operator-local directory referenced via
`MAMUT_TW_REFERENCE_DIR`; evidence documents carry only derived metrics
and the segment catalog. The offline calibration harness (`SET6-3`) is
the measuring instrument, validated on our own known-ground-truth
renders before it is trusted on references, and every audible slice
from then on lands with a harness delta table where the reference set
covers its axis.

### 6. Rotary speaker as a source-agnostic engine-global layer

The rotary speaker is built as an engine-global layer whose first
client is the organ bus, not as an organ-private effect. Its evidence
must demonstrate it on an existing factory-patch render (session-only,
no patch change) as well as on the organ.

**Rejected: organ-private rotary.** Binding the rotary into the organ
chain as private code would duplicate work the existing patches can use
today, hide a reusable layer behind a single client, and contradict how
the engine already treats global processing (GFM/BCS are layers over
whatever the mix carries).

Unchanged constraints, restated so the new source is not misread:

- The ADR 0001 realtime rules bind every slice: allocation-free steady
  state on render paths, bounded everything, no callback I/O, drop
  visibility. Set 6 adds a constant-cost rule: all 91 wheels render
  every frame while enabled; differences are gain-gated, never
  per-wheel branch-gated; with the concept disabled the organ block is
  skipped and the render is bit-identical to the pre-slice baseline.
- The ADR 0004 GUI boundary stands; GUI exposure of Set 6 state is
  deferred (at most an `INSPECT` stretch in `SET6-11`).
- The ADR 0005 transport posture is untouched: Set 6 consumes the same
  scheduled engine note events the voice allocator consumes and must
  never block on, or be blocked by, Set 4.
- The mandated review gates (`docs/review-gates.md`) apply to every
  slice, including `sel4-rust-execution-optimizer` for every slice that
  adds DSP to the per-frame organ render path (`SET6-4..10`).

## Consequences

Backlog Set 6 items `SET6-1..11` may proceed in the order the backlog
recommends; `SET6-1` intake is blocked only on the operator's reference
recordings. `docs/README.md` and the `CLAUDE.md` reading order gain
this ADR in the same change.

What becomes possible: the first engine-global sound source, a
reusable rotary-speaker layer for existing patches, a calibration
harness that later tracks can reuse for any reference-driven work, and
a pinned-constants discipline (`docs/dsp/tonewheel-constants.md`) that
keeps numeric behavior sourced.

What stays forbidden: patch-schema growth in this set, factory-bank or
live-set changes, protected names anywhere in the repo, committed
reference audio in any form, per-wheel branch-gating on the render
path, and transport or GUI-boundary changes smuggled in through the
organ work.

Reversibility: before `SET6-4` lands, the track is additive leaf code
plus docs and rescinding it is a deletion. After `SET6-4`, the engine
carries a global source join in `render_frame` and a session command
surface; unwinding is still a bounded removal (the disabled path is
bit-identical by construction, so the join is cleanly excisable). The
naming policy (§4) is deliberately *not* reversible with the track: it
binds the repo regardless of the organ's fate.
