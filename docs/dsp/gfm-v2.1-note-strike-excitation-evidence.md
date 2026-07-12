# GFM v2.1 Note-Strike Excitation Evidence

Date: 2026-07-07

Backlog: `EPM1_BACKLOG_SET1_GFM_PLAYABLE_FIELD.md`, item `SET1-1`.

This slice makes the GFM lattice hear played notes. Until now the live GFM
layer felt only two scalars (aftertouch pressure and the `K8` amount) and its
spatial excitation footprint was fixed at the lattice center. Note events now
inject deterministic, bounded spatial strikes into the field, so the lattice
state — per-cell energy, heat, strain, rupture timing — responds to *what* the
player plays and *where* on the keyboard they play it.

## What Landed

`mamut-field`:

- `GfmStrike { x, y, pressure, heat, rupture_bias }` — one spatial strike;
  scalar fields sanitize to `[0, 1]`, coordinates wrap toroidally. The
  backlog sketch's fixed-capacity staging buffer was prototyped and then
  dropped in-slice on review: injection is always event-at-a-time through
  the engine, so a batch buffer had no consumer and would have been
  speculative public surface.
- `note_strike_position::<W, H>(note, seed)` — the deterministic note map
  (below).
- `GfmLattice::inject_strike` — event-boundary deposit
  into cell fields through a bounded footprint (quadratic falloff, radius
  `3.0`, fixed `7x7` window): `pressure` deposits energy (`x 1.10`), `heat`
  deposits heat (`x 0.45`), `rupture_bias` deposits strain (`x 0.14`). All
  deposits clamp to the same per-cell ceilings the update loop enforces
  (energy/strain `2.8`, heat `2.2`, now shared constants). No allocation, no
  RNG draw, no per-sample work — the per-sample cell loop is untouched.
- `GfmDiagnostics` gained `strike_count` (session strike counter).

`mamut-engine`:

- `GfmFieldVoice::strike_note_on / strike_note_off` — program-aware strike
  shaping (below), folded with the voice's lattice seed.
- `Engine::note_on / note_off` tap into the GFM voice only when the layer is
  armed (a `GfmFieldVoice` exists). Velocity scales strike strength; note-off
  emits a weaker release strike (`0.30 x` pressure, `0.50 x` heat, zero
  rupture bias) using the released voice's velocity.
- `Engine::set_gfm_note_strikes_enabled` — session control, enabled by
  default; disabling restores the center-only excitation baseline exactly.
  Exposed in `GfmLayerSnapshot::note_strikes_enabled`.
- The aftertouch pressure path is byte-for-byte unchanged; strikes are
  additive on top of the existing excitation contract.

Patch surface: none. Note response is session-only in v1 per the backlog's
default position; `[engine.gfm]` gained no fields.

## Note Map

`note_strike_position` is deterministic and seeded:

- pitch class (`note % 12`) selects the x band: `x_base = (pc * W) / 12`
- octave (`note / 12`) selects the y band: `y_base = (octave * H) / 11`
- the lattice seed XOR-folds a bounded per-cell offset on top:
  `folded = seed ^ (seed >> 27) ^ (note * 0x9E37_79B9_7F4A_7C15)`, then
  `dx, dy ∈ {-1, 0, +1}` from bits 8.. and 16.. of `folded`
- coordinates wrap toroidally (`wrap_index`)

Collisions are acceptable; strike energy sums (clamped). With the production
seed `0x6A46_4D40` the A/B pattern's eight notes map as (probe = `(8, 8)`):

| note | position (x, y) | note | position (x, y) |
|------|-----------------|------|-----------------|
| 36   | (15, 3)         | 65   | (6, 7)          |
| 43   | (10, 4)         | 67   | (10, 7)         |
| 52   | (5, 4)          | 72   | (0, 8)          |
| 58   | (14, 6)         | 79   | (9, 7)          |

Pitch classes around F..G land near the probe column; C lands at the far
toroidal edge. Spatial audibility therefore varies by pitch class — this is
the intended spatial character of the map, not an accident, and it is the
direct motivation for the `SET1-2` stereo probe and the `SET1-3` field view.

## Strike Shaping (engine, per program)

Velocity is sanitized to `[0, 1]`; controls are the patch's sanitized
`GfmPerformanceControls`.

- `horizont`: pressure `v * (0.30 + depth*0.42 + spread*0.10)`, heat
  `v * (0.05 + heat*0.14)`, rupture bias `0` — structurally rupture-free
  (rupture additionally requires the `Baklja` posture in the cell loop).
- `pec`: pressure `v * (0.24 + body*0.28 + depth*0.12)`, heat
  `v * (0.16 + heat*0.44 + brightness*0.10)`, rupture bias `0`.
- `baklja`: pressure `v * (0.26 + depth*0.38 + body*0.12)`, heat
  `v * (0.10 + heat*0.22)`, rupture bias thresholded on velocity:
  `edge = clamp((v - 0.58) / 0.42)`, bias
  `edge * (0.30 + rupture*0.55 + brightness*0.10)` — soft notes never bias
  rupture; hard notes do, scaled by the patch's rupture control.

## Evidence Runs

Harness: `crates/mamut-engine/examples/gfm_engine_note_strike_ab_render.rs`.
Per patch it renders a `center` take (strikes disabled — the center-only
baseline), a `notes` take (strikes enabled), and a repeat of the `notes` take,
all with the same scripted eight-note pattern (notes 36..79 spanning pitch
classes, velocities 0.52..0.90, 0.30 s spacing, 0.22 s holds) and the same
held live controls (`K8 = 0.68`, aftertouch `0.62`). The example hard-fails
if signatures do not separate, if the repeat is not bit-identical, if
horizont/pec rupture, or if the Baklja rupture count exceeds its bound.

Release strikes fire on key-up regardless of the sustain pedal: the field
hears the keyboard *gesture*, not the voice envelope, so a sustained voice
keeps sounding while the field already receives its (rupture-free) release
deposit. This is the intended v1 semantic.

Smoke quality (8 kHz, 3 s), all rows from one run of one binary, measured
2026-07-07:

| patch (program) | take | RMS | peak | max ruptures | strikes | PCM signature (FNV-64) |
|---|---|---|---|---|---|---|
| cathedral-bloom (horizont) | center | 0.2796 | 0.8994 | 0 | 0  | `3cdc666938091f49` |
| cathedral-bloom (horizont) | notes  | 0.2796 | 0.8994 | 0 | 16 | `469641b3075d369c` |
| ember-vault (pec)          | center | 0.4108 | 0.7165 | 0 | 0  | `85c5e8d79485d4e9` |
| ember-vault (pec)          | notes  | 0.4108 | 0.7165 | 0 | 16 | `327d0854d99272f5` |
| razor-thaw (baklja)        | center | 0.1241 | 0.9486 | 61 | 0 | `b601825140272977` |
| razor-thaw (baklja)        | notes  | 0.1241 | 0.9486 | 76 | 16 | `7741a3e47e8fb8d3` |

- every `notes` take is PCM-distinct from its `center` baseline and
  bit-identical across repeat runs (same seed, same pattern)
- `horizont` and `pec` stay rupture-free with strikes; `baklja`'s rupture
  profile moves (61 → 76 peak in-frame count) and stays bounded (≤ 102)
- all takes finite and under the master safety ceiling
- mix-level distance (`center` vs `notes`, mono fold): diff RMS
  `1.9e-5` (horizont), `7e-6` (pec), `6.5e-4` (baklja)

Listen quality (48 kHz, 6 s; 18 scripted notes → 36 strikes per `notes`
take), measured 2026-07-07: every `notes` take is again PCM-distinct from its
baseline and bit-identical across repeats; `horizont`/`pec` stay rupture-free;
`baklja` peak in-frame rupture count moves 62 → 57 with a denser suspect
population (239 → 246 suspect cells at take end) — strikes redistribute where
and when Baklja ruptures rather than simply adding more. Mix-level mono diff
RMS: `3e-6` (horizont), `2e-6` (pec), `5.5e-4` (baklja).

Field-level response (GfmFieldVoice at 8 kHz, live controls 0.62/0.68, two
strikes at notes 67 and 79 — the cells nearest the probe cross), measured
with a throwaway harness over the public API before this document was
written:

| program | max |Δsample| | Δ RMS |
|---|---|---|
| horizont | 0.101 | 0.0076 |
| pec | 0.055 | 0.0028 |
| baklja | 1.203 | 0.2970 |

## Honest Audibility Note

At the engine mix the strike response is subtle for `horizont`/`pec` (the
GFM layer mixes at gain 0.14/0.20 and the probe reads only the 5-tap center
cross, so most of the field's spatial state is invisible to the output) and
clearly audible for `baklja` (strikes shift rupture timing). The field-level
response is strong for all three programs. This is the structural limit of
the mono center probe, not of the strike path — widening what the output can
see is exactly `SET1-2` (stereo field probe), and making the response visible
is `SET1-3` (INSPECT field view). Reviewers should treat the strike path as
landed and the audible reach as intentionally conservative in v1.

## Validation

Commands run (2026-07-07):

- `cargo fmt --all --check` — clean
- `cargo test --locked -p mamut-field` — 33 passed (6 new strike tests:
  map determinism/bounds, seed folding, hostile-field sanitize,
  byte-identical strike renders, baseline distinctness,
  finite/bounded/rupture-safe renders)
- `cargo test --locked -p mamut-engine` — 67 passed (4 new: armed A/B with
  bounded rupture + snapshot flag truth, byte-identical strike render across
  runs, release-strike counting incl. disabled-layer no-op, field-voice
  strike determinism)
- `cargo test --locked` (workspace) — green: dsp 48, engine 67, field 33,
  identity 4, params 5, patch 9, runtime 12, seq 27, standalone 79, tui 2;
  0 failed
- `cargo run --locked -p mamut-engine --example gfm_engine_note_strike_ab_render`
  (smoke and `--quality listen`) — all in-example assertions green

Listen-quality artifacts (48 kHz, 6 s) under `target/gfm-render/`:
`strike_listen_center_<patch>.wav` / `strike_listen_notes_<patch>.wav`.

## Boundary Compliance

- no changes to the audio callback boundary, queue architecture, or MIDI
  ingress; strike injection is bounded event-boundary work inside
  `Engine::process_block`'s existing event dispatch
- `mamut-field` remains external-dependency-free (the `[dependencies]` table
  is still empty; the enforcing test still passes)
- no allocation, logging, formatting, or blocking added to the per-sample
  path; the cell update loop is unchanged apart from naming its existing
  clamp ceilings
- factory bank and live-set slots untouched
- contract update landed in `gfm-performance-control-contract.md`
  (note-strike addendum)

## Review Gates

Run 2026-07-07:

- `sel4-rust-systems-reviewer` (`mamut-field`, `mamut-engine`) —
  **approve-with-nits**. One minor acted on: the fixed-capacity strike
  staging buffer from the design sketch had no production consumer and was
  removed in-slice (injection is event-at-a-time through the engine).
- `sel4-rust-execution-optimizer` (lattice injection path) — **approve**.
  `inject_strike` is a fixed 49-iteration window, allocation/RNG/panic-free,
  ~2 µs per note event on the measurement host's class of CPU; a 6-note
  chord burst is under 0.5 % of a 96 kHz / 256-frame block budget. Per-sample
  path confirmed unchanged; ceiling consts confirmed bit-identical to the
  prior literals. Optional footprint-LUT nit declined (risks golden-signature
  ULP drift for no needed budget win).
- `sel4-integrated-systems-reviewer` (contract/doc changes) —
  **approve-with-nits**. Both doc-integrity majors fixed in this document
  (single-run signature table with full FNV-64 values; dead buffer removed
  from code and doc); note-range framing corrected to 36..79; the sustain
  release-strike semantic documented above as intended.
