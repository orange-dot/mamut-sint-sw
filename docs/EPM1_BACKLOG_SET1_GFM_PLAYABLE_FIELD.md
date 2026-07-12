# EPM1 Backlog Set 1: GFM Playable Field

Date: 2026-07-06

Status: all three items landed on 2026-07-07 —
`SET1-1` as `docs/dsp/gfm-v2.1-note-strike-excitation-evidence.md`,
`SET1-2` as `docs/dsp/gfm-v2.2-stereo-probe-evidence.md`,
`SET1-3` as `docs/dsp/gfm-v2.3-inspect-field-view-evidence.md`.
`SET1-3`'s windowed screenshot is pending a rig session (this build host is
headless); the field-view data path and render logic are unit-verified. Each
item shipped as its own slice with its own evidence document. Final `gfm-v*`
numbering was assigned at landing time and continues the chronological chain
in `docs/dsp/`.

## Summary

This set deepens the innovative core of `EPM1` — the GFM lattice in
`crates/mamut-field` — along the axis that matters most for differentiation:
making the field *playable* and *visible* instead of a hidden color layer.

Three items:

1. `SET1-1` — note-driven field excitation ("the field hears your notes")
2. `SET1-2` — stereo field probe
3. `SET1-3` — GFM field view in the `INSPECT` screen

## Shared Boundary Constraints

All items in this set respect the standing repo posture:

- no changes to the audio callback boundary, queue architecture, or MIDI
  ingress under this set (written under the transport freeze, since rescinded
  by ADR 0005; the constraint stands on its own for this set's scope)
- `mamut-field` stays external-dependency-free (a unit test enforces the
  empty `[dependencies]` table)
- realtime paths stay allocation-free in steady state; workspace lint posture
  (`unsafe_code = "forbid"`, clippy panic/unwrap warns) is preserved
- factory bank and live-set slots stay locked
- the production GFM contract remains
  `docs/dsp/gfm-performance-control-contract.md` (three programs plus `auto`);
  any contract change lands as an explicit update to that document

## SET1-1: Note-Driven Field Excitation

### Rationale

Today the live GFM layer feels only two scalars — aftertouch pressure and the
`K8` layer amount — and the spatial excitation footprint is fixed at the
lattice center (`excitation_footprint` in `crates/mamut-field/src/topology.rs`).
Note events never reach the field. Mapping played notes into spatial strikes
turns GFM from a color layer into a playable field, which is the strongest
available differentiation step: a lattice with per-cell health, quarantine,
and quorum rupture that *responds to what the player plays*.

### Design Sketch (non-binding)

- `mamut-field`: a bounded strike API. A fixed-capacity strike buffer (for
  example `[GfmStrike; 8]` plus length) where
  `GfmStrike { x, y, pressure, heat, rupture_bias }`. Strikes deposit into
  cell fields through a small bounded footprint at injection time (event
  boundary work), or as short-lived decaying sources; either way, no
  allocation and no per-sample scan beyond the existing cell loop.
- Note-to-coordinate mapping is deterministic and seeded: for example pitch
  class selects the x band, octave selects the y band, XOR-folded with the
  lattice seed for per-patch variation. The exact map is documented in the
  evidence doc. Collisions are acceptable; energy sums.
- `mamut-engine`: note-on/note-off events tap into `GfmFieldVoice` only when
  the layer is armed. Velocity scales strike strength. Whether note-off emits
  a weaker release strike is decided inside the slice.
- The existing excitation contract stays backward compatible: the aftertouch
  pressure path is unchanged; strikes are additive on top.

### Contract Question To Settle In-Slice

Does the optional `[engine.gfm]` patch table grow note-response fields (for
example strike depth), or does note excitation stay session-only in v1?
Default position: session-only in v1; patch surface follows once the response
is proven musical.

### Deliverables

- strike API and note mapping in `mamut-field` with unit tests (sanitize,
  bounds, determinism of the mapping)
- engine plumbing from note events to armed GFM voice
- offline A/B render example (extend `gfm_engine_layer_ab_render` or add a
  sibling) driving scripted note patterns
- evidence document and, if behavior is promoted, a
  `gfm-performance-control-contract.md` update

### Acceptance

- scripted note patterns produce PCM signatures distinct from the
  center-only baseline (A/B renders, reported RMS/peak/rupture per take)
- same seed plus same pattern renders bit-identical PCM (mirror of the BCS
  determinism test)
- `horizont` and `pec` normal sweeps stay rupture-free per the v0.2 live
  response contract; `baklja` rupture profile stays scaled and bounded
- all output finite and under the master safety ceiling
- no allocation, logging, formatting, or blocking added to the per-sample
  path; strike injection is bounded event-boundary work
- `cargo test --locked` green

### Evidence

- proposed: `docs/dsp/gfm-v2.1-note-strike-excitation-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer` (`mamut-field`, `mamut-engine`)
- `sel4-rust-execution-optimizer` (lattice and injection path)
- `sel4-integrated-systems-reviewer` (contract/doc changes)

### Size

M

## SET1-2: Stereo Field Probe

### Rationale

`read_probe_output` (`crates/mamut-field/src/lattice.rs`) reads one 5-tap
center cross; the engine duplicates it to both channels, and all stereo
character comes from the mix-stage spread in
`crates/mamut-engine/src/engine/layers.rs`. A second, spatially offset tap set
gives true field decorrelation between left and right for near-zero cost —
probe reads are read-only passes and do not touch the lattice update.

### Design Sketch (non-binding)

- add a stereo readout (for example `read_probe_output_stereo`) returning a
  pair from two offset tap sets (for example ±2 columns around center);
  posture-aware offsets are optional
- keep the existing mono readout for offline renders and existing tests
- the engine GFM layer consumes the pair; the current artificial spread logic
  is reduced or removed in favor of real field stereo

### Deliverables

- stereo probe readout in `mamut-field` plus tests
- engine layer consumption
- A/B render evidence (mono baseline versus stereo probe)

### Acceptance

- measurable L/R decorrelation (report cross-correlation against the mono
  baseline in the evidence doc)
- both channels finite and bounded; mono fold-down `(L+R)/2` stays bounded
  and musically intact (listening note)
- per-sample cost delta is read-only-tap sized; report the delta once the
  SET2-1 bench exists, otherwise note it qualitatively
- determinism preserved (same seed, bit-identical pair sequence)

### Evidence

- proposed: `docs/dsp/gfm-v2.2-stereo-probe-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`, `sel4-rust-execution-optimizer`

### Size

S

## SET1-3: GFM Field View In INSPECT

### Rationale

The innovation is invisible today: `GfmDiagnostics` is scalar-only, and the
v1.9 field workbench (terrain snapshot, probe/trajectory markers) was
deliberately reverted from production. The GUI restructure to the four-screen
IA (`PERFORM`, `SOUND`, `SYSTEM`, `INSPECT`) is active per ADR 0003/0004 —
`INSPECT` is the natural home for a live 16x16 field view. A visible field
also directly supports `EPM1_GFM_LIVE_BUG_PLAYBOOK.md` Pass 1: an operator can
see the field evolving while the audible gate stays closed, which
disambiguates "armed but silent" from a dead layer.

The reverted v1.9 machinery in git history (see
`docs/dsp/gfm-v1.9-field-workbench-evidence.md`, marked experimental) is the
reference implementation; this item promotes a *bounded subset* — read-only
terrain view, no coupling-profile patch surface.

### Design Sketch (non-binding)

- `mamut-field`: a bounded terrain snapshot type (16x16 energy, fracture, and
  health, quantized fields are acceptable) produced on demand — never from
  the audio callback
- `mamut-engine`: optional terrain in `EngineSnapshot` when a GFM voice is
  armed; snapshot generation already runs off the audio fast path
- `mamut-runtime`: `SessionStateSource` read-only extension (this is the
  GUI's read surface; `SessionCommandSink` is not involved)
- `mamut-standalone`: an `INSPECT` page rendering the heat map with health
  color coding and last-rupture markers, styled per
  `docs/ui/epm1-gui-design-system.md`; legacy tabs coexist until cutover per
  ADR 0004
- if SET1-1/SET1-2 have landed, overlay strike positions and probe taps

### Deliverables

- terrain snapshot path (field → engine snapshot → state source → GUI)
- `INSPECT` field page
- evidence doc with screenshot and a playbook cross-link

### Acceptance

- `INSPECT` shows the live field with GFM armed in a windowed session
- zero changes on the audio callback path (diff-scoped assertion in review)
- snapshot copy stays fixed-size (16x16 arrays; no per-frame heap growth in
  the engine snapshot path)
- `EPM1_GFM_LIVE_BUG_PLAYBOOK.md` Pass 1 references the view as a
  classification aid

### Evidence

- proposed: `docs/dsp/gfm-v2.3-inspect-field-view-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer` (`mamut-engine`, `mamut-runtime`,
  `mamut-standalone`)
- `sel4-integrated-systems-reviewer` (IA and docs)

### Size

M

## Suggested Order And Dependencies

1. `SET1-1` (the core musical jump)
2. `SET1-2` (small; can also run before or alongside SET1-1)
3. `SET1-3` (benefits from 1 and 2 for overlays, but can start on current
   diagnostics if GUI bandwidth exists)

Cross-set: Backlog Set 3 (`mamut-seq`) makes every acceptance pass in this
set scriptable (deterministic note patterns, aftertouch gestures, `K8`
sweeps) without the physical PC4 rig. Landing Set 3 items 1–3 first is
recommended.

## Out Of Scope

- transport, queue, or callback boundary changes
- seven-program surface revival (Backlog Set 2, `SET2-6`)
- patch schema `v2`
- plugin/editor work
