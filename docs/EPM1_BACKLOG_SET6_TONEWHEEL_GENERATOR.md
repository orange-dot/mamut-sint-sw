# EPM1 Backlog Set 6: Tonewheel Generator — A Component-Modeled Electromechanical Organ

Date: 2026-07-14

Status: accepted (operator decisions recorded 2026-07-14); no slices landed
yet. Each item ships as its own slice with its own evidence document and
review gates. Evidence file names are proposals; final numbering is assigned
at landing time, continuing each concept's own `docs/dsp/` chain the way
`gfm-v*` and `mozaik-v*` grew.

## Summary

Operator decisions (2026-07-14) — these are inputs to this backlog, not open
questions:

- **The goal is a credible tonewheel organ, judged by ear.** Not "an organ
  preset". The acceptance bar for the set is that renders stand next to
  operator-provided reference recordings of canonical tonewheel-organ
  performances (the Jon Lord / Keith Emerson school) and survive the
  comparison on the measurable axes — registration spectra, key click,
  scanner motion, rotary motion — plus the listening checklist.
- **Component model of the machine, not an additive approximation.** The
  instrument family being modeled is electromechanical: a bank of
  continuously rotating tonewheels, a passive key-contact/busbar network, a
  mechanical vibrato scanner, a tube preamp, and a two-rotor rotating
  speaker cabinet. Set 6 models those components. Full circuit-level
  simulation (wave-digital tube stages, solved resistor networks) is
  explicitly *not* in this set; a single component may earn that upgrade
  later only if the calibration harness and the operator's ear demand it.
- **One global shared generator, not per-voice partials.** Five behaviors
  that per-voice additive synthesis cannot reproduce are the point of the
  track: (1) *shared-wheel phase coherence* — two held keys whose drawbar
  taps land on the same wheel reinforce exactly instead of chorusing;
  (2) *loudness robbing* — passive bus summing redistributes level as keys
  are added; (3) *key click* that emerges from switching a live, running
  signal mid-cycle through bouncing contacts; (4) *gear-ratio tuning* —
  wheel frequencies come from a twelve-ratio gear table, so the slow
  near-equal-temperament beat patterns between drawbars are fixed
  properties of the machine; (5) *leakage* — faint bleed of wheels that are
  not keyed. The generator is the first engine-global sound *source*
  (GFM/BCS are engine-global *layers*; Mozaik is per-voice with a global
  frame).
- **Naming and trademark policy (binding).** This repo names no
  manufacturer and no model of the emulated instrument family — not in
  docs, code, identifiers, params, patch names, UI strings, or commit
  messages. The vocabulary is generic: *tonewheel generator*, *drawbars*,
  *registration*, *foldback*, *key click*, *percussion*, *vibrato
  scanner*, *rotary speaker*. The repo has zero such occurrences today and
  every Set 6 slice keeps it that way. External works are cited in
  `docs/EXTERNAL_DSP_REFERENCES.md` by author/venue/year plus a link;
  where a cited work's verbatim title contains a protected name, the
  citation uses the author-venue-year form instead of quoting the title,
  and any unavoidable edge is resolved by the operator at `SET6-1` review.
- **Calibration is reference-driven.** The operator provides reference
  recordings *before implementation begins* (`SET6-1` intake). Recordings
  never enter the repo (copyright); they live in an operator-local
  directory referenced via the `MAMUT_TW_REFERENCE_DIR` environment
  variable; evidence documents carry only derived metrics (spectra, rates,
  times, level tables) and the segment catalog.
- **The rotary speaker stays inside Set 6** (`SET6-8/9`) and is built as a
  source-agnostic engine-global layer: its evidence must demonstrate it on
  the organ bus *and* on an existing factory-patch render (session-only;
  no patch file changes).
- **Evidence-first, session-only, no schema growth.** The Set 5 ladder
  applies unchanged: `mamut-dsp` primitive plus offline render evidence
  first, engine integration second, session-only controls third.
  `schema_version` stays 1; factory patches and the locked live set do not
  change; GUI exposure is deferred (at most an `INSPECT` stretch in
  `SET6-11`, cuttable without ceremony). Patch schema, controller-profile
  drawbar bindings, and `EPM2` parity form a future contract set (see
  Deferred Follow-Ups).
- **Session controls are capped at five per concept** (plus the on/off
  mode), and the nine-drawbar registration counts as **one vector
  control**, written in the instrument family's canonical nine-digit
  notation (`888000000`, leftmost = the sub-octave drawbar) and set
  atomically. Tonewheel concept: `registration`, `percussion` (compound
  enum), `vibrato` (enum), `drive`, `wear`. Rotary concept: `mode`,
  `balance`, `width`, `drive`. If either concept wants a sixth knob, the
  model is wrong; redesign instead of adding it.

Items:

1. `SET6-0` — ADR 0006: tonewheel generator track decision record
2. `SET6-1` — constants, external references, and reference-recording
   intake
3. `SET6-2` — Tonewheel v0.1: shared generator primitive (`mamut-dsp`)
   plus offline render evidence
4. `SET6-3` — Tonewheel v0.2: offline calibration/analysis harness
5. `SET6-4` — Tonewheel v0.3: engine integration — contact/busbar model,
   key click, robbing, session controls
6. `SET6-5` — Tonewheel v0.4: percussion
7. `SET6-6` — Tonewheel v0.5: vibrato/chorus scanner
8. `SET6-7` — Tonewheel v0.6: preamp drive
9. `SET6-8` — Rotary v0.1: two-rotor core — crossover, Doppler, AM,
   inertia
10. `SET6-9` — Rotary v0.2: cabinet, stereo field, drive interaction
11. `SET6-10` — Tonewheel v0.7: leakage, hum, wheel character (the `wear`
    knob)
12. `SET6-11` — consolidation: cost table, docs truth, demo script,
    listening checklist

Milestones: the organ is *recognizable* after `SET6-4` plus `SET6-8`
(drawbars, click, robbing, a moving speaker); the *credible* bar is the
full set with harness deltas inside the `SET6-1` tolerances.

## Assessment Inputs (What The Code Says Today)

- The render frame is `crates/mamut-engine/src/engine/macro_state.rs:16`
  (`render_frame`): a per-voice source loop (`:53-300`, summing at
  `:231-252`), then the final body/mid-side stage (`:302-323`), then
  chorus/reverb (`:325-343`), then the engine-global GFM/BCS layers
  (`:345-346`), then crossfeed and output trim (`:348-353`). The organ bus
  joins after the voice loop and before the final body/mid-side stage, so
  the existing final saturation and safety rail keep governing the output
  (exact position recorded in ADR 0006).
- Engine-global precedents: `apply_gfm_layer`/`apply_bcs_layer`
  (`macro_state.rs:345-346`, layer struct in
  `crates/mamut-engine/src/gfm_layer.rs`) for engine-owned global DSP, and
  the Mozaik global frame (`advance_mozaik_frame`, `macro_state.rs:48`,
  consumed per voice at `:245-252`) for a per-frame global state feed.
  There is no engine-global *source* yet; Set 6 adds the first one.
- The nearest existing relative is the per-voice additive bank
  (`additive_partials: [Oscillator; 8]`,
  `crates/mamut-engine/src/state.rs:463`, rendered at
  `macro_state.rs:193-221`). Set 6 deliberately does **not** extend it —
  see the Summary for the five behaviors that force a shared generator.
- `mamut-dsp` already carries the needed primitives, zero-dependency and
  allocation-free: `Oscillator` phase accumulator
  (`crates/mamut-dsp/src/oscillator.rs:4`) with `sine_phase_sample`
  (`waveform.rs:132`); `DelayLine::read_linear` fractional tap
  (`delay.rs:35`); `AllpassFilter` (`delay.rs:103`), `CombFilter`,
  `OnePoleDamping`; `TptStateVariableFilter` (`filter.rs:16`) for
  crossover work; `AdsrEnvelope` (`envelope.rs:30`); `NoiseRng`
  (`noise.rs:2`); `LinearSmoother` (`smoother.rs:2`); waveshapers
  `tanh_drive_compensated` (`waveshaper.rs:8`) and `asymmetric_diode`;
  the safety rail (`safety.rs`: `DcBlocker`, `master_safety_limit`,
  `sanitize_block`).
- Session-control precedent (Set 5): `engine.set_mozaik_mode(...)` /
  `set_mozaik_param(...)` setters used by the A/B example
  (`crates/mamut-engine/examples/mozaik_engine_source_ab_render.rs`); the
  headless `mozaik set` chain — parse
  (`crates/mamut-runtime/src/cli/runtime_commands.rs:67`), dispatch
  (`crates/mamut-runtime/src/session/commands.rs:74`), shared session
  method `set_mozaik_param`
  (`crates/mamut-runtime/src/session/switching.rs:84`); and the
  status-line helpers (`crates/mamut-runtime/src/commands.rs:119-123`).
  The controller-profile binding kind (`mozaik_control`, SET5-8, routed
  at `session/runtime.rs:316-317` onto the same shared session method) is
  the pattern a future contract set reuses for drawbars.
- Evidence template: `mozaik_engine_source_ab_render.rs` — 48 kHz render,
  256-frame blocks, scripted `ScheduledNoteEvent` streams, pre-slice
  baseline FNV-64 signatures hardcoded as asserts, WAV outputs, per-block
  cost measurement. Set 6 examples follow it.
- Runtime sample rates: default 96 kHz, supported 44.1–192 kHz
  (`CLAUDE.md`); all fixed buffers are sized at construction from
  `EngineConfig`. At 44.1 kHz the top wheel (≈5.9 kHz) is still far below
  Nyquist; no per-rate special cases are expected.
- Cost envelope (to be measured, not asserted): 91 phase accumulators at
  96 kHz ≈ 8.7 M sine evaluations/s plus three 91-wide dot products and
  two 91-wide one-pole smoother banks ≈ tens of M mult-adds/s — well
  under a few percent of one core on the reference host. Every slice's
  evidence carries its own measured per-block cost table.

## Shared Boundary Constraints

- **Workspace lint posture untouched**: `unsafe_code = "forbid"`; clippy
  warnings on `dbg!`/`expect`/`panic`/`todo`/`unwrap` stay warnings-clean.
  Pure safe Rust; no new external dependencies anywhere in the set;
  `mamut-dsp` keeps its zero-dependency posture (the calibration harness
  reads WAV via a minimal repo-local RIFF reader, not a crate).
- **Realtime discipline** (ADR 0001 standing rules): allocation-free in
  steady state on render paths; no logging/formatting/blocking there;
  bounded everything. All audio-rate state is fixed-size and always
  allocated at construction.
- **Constant-cost rule (new for Set 6)**: while the tonewheel concept is
  enabled, all 91 wheels advance and render every frame regardless of held
  keys; behavior differences are gain-gated, never per-wheel
  branch-gated. Per-key folding work happens once per block at the top of
  `process_block` (before the per-frame loop — the engine has no separate
  control-rate tick today) plus on contact/registration events at their
  frame offsets, bounded by the 61-key compass. With the concept
  `Disabled` the whole
  organ block is skipped (the Mozaik skip precedent,
  `macro_state.rs:241-252`) and the render is bit-identical to the
  pre-slice baseline.
- **Determinism** (the `gfm-wild-concepts.md` reading contract, binding):
  same event stream produces bit-identical output; contact-bounce
  randomness comes from a fixed-seed generator advanced per event ordinal,
  never from time or global entropy; fixed iteration budgets; clamped
  everything; NaN/inf control inputs sanitize. Two-run FNV-64 signature
  equality is mandatory in every evidence doc.
- **Baseline mechanics** (the SET5-4 standard): integration slices render
  the pre-slice baseline at the parent commit, record signatures in the
  evidence doc, **and hardcode them as asserts inside the A/B example**,
  so the bit-identity bar is executable on every run.
- **No patch-schema growth.** `schema_version` stays 1; factory patches
  and the locked live set stay locked; all controls are session-only
  (engine setter, headless command). Session values do not survive a
  runtime engine rebuild (patch or audio-config switch reapplies
  on-enable defaults) — accepted for the whole set, stated here once.
- **Keyboard compass**: the organ tracks a 61-key manual, MIDI notes
  36..=96. Note events outside the compass are ignored by the organ
  (the voice layer keeps seeing them normally); the snapshot counts them
  so the behavior is visible, not silent.
- **Velocity is ignored by the organ** (contact closure is binary; click
  level is not velocity-scaled) — that is how the machine works. A
  velocity→click mapping is a possible future UMP-era slice, not Set 6.
- **SET4 independence.** The organ consumes the same scheduled engine
  note events the voice allocator consumes, at the same frame offsets.
  Nothing in this set touches transport, MIDI parsing, queue payloads, or
  the callback. Set 6 must never block on Set 4, and vice versa.
- **Copyright discipline**: reference recordings are never committed, in
  any form, including short excerpts; evidence docs carry derived metrics
  and segment references (artist, release, timestamp) only.
- **Honesty discipline** (`masnoca.md` precedent): every evidence doc
  carries a nearest-relatives section. For this set that means saying
  plainly that component-modeled tonewheel emulation is established
  practice (commercial emulations and published papers exist); the claim
  here is calibration and engineering discipline, not novel DSP.
- **Docs and code describe the same truth in the same change**
  (`CLAUDE.md`, `AGENTS.md`, `README.md`, `docs/README.md` where touched).
- The mandated gates in `docs/review-gates.md` apply to every slice in
  full; the per-item gate lists below name slice-specific *additions*
  only. Stated once so no slice reads as exempt: **every slice that adds
  DSP to the per-frame organ render path (`SET6-4` through `SET6-10`)
  carries `sel4-rust-execution-optimizer`** on top of its listed gates.
  The helper scripts under `tools/review/` print the prompts.

## Execution Protocol For The Implementing Agent

Read these before the first line of code, in this order: `AGENTS.md`,
`CLAUDE.md`, this backlog, `docs/review-gates.md`, ADR 0006 (once landed),
`docs/dsp/tonewheel-constants.md` (once landed — no numeric behavior may be
implemented before its constant is pinned or explicitly marked open),
`docs/dsp/gfm-wild-concepts.md` (reading contract and honesty style), and
`crates/mamut-engine/examples/mozaik_engine_source_ab_render.rs` plus one
landed evidence doc (`docs/dsp/gfm-v2.2-stereo-probe-evidence.md`) as the
format models.

Per slice, the loop is:

1. Render the **pre-slice baseline** first where the slice's acceptance
   needs one (integration slices): run the named baseline command at the
   parent commit, record output signatures in the evidence draft.
2. Implement the slice. One slice per change; do not fold two items into
   one branch.
3. `cargo fmt --all --check`, `cargo test --locked`, plus the slice's own
   example runs. All green before review.
4. Write the evidence doc in the same change (environment header, exact
   commands, one-run/one-binary tables, full FNV-64 signatures, honest
   caveats, nearest-relatives section). From `SET6-3` on, every audible
   slice also includes its calibration-harness delta table where the
   reference set covers the slice's axis; where it does not, the doc says
   so instead of skipping silently.
5. Run the mandated review gates (`tools/review/*.sh` print the prompt
   and scope) and address findings.
6. Update `docs/README.md` (and `README.md`/`CLAUDE.md` only where the
   slice says so) in the same change.

Numbers marked **tuned in-slice** below are starting points chosen for
plausibility, not measurements; the slice must keep or move them based on
rendered evidence (and, where covered, harness deltas) and say which
happened. Struct sketches are shape guidance, not frozen names — but
parameter *semantics*, clamps, and determinism requirements are binding.

Recommended serial order for a single implementing agent:
`SET6-0, 1, 2, 3, 4, 8, 5, 6, 7, 9, 10, 11` — the rotary core lands right
after engine integration so the recognizability milestone arrives as early
as possible. `SET6-8/9` depend only on `SET6-0/1/3` and may proceed in
parallel with `SET6-4..7` when more than one agent works the set.

---

## SET6-0: ADR 0006 — Tonewheel Generator Track Decision Record

### Rationale

This track adds the first engine-global sound source, a new post-mix layer
family, a binding naming policy, and a reference-calibrated evidence bar.
That is an architecture decision, and the house rule is that architecture
decisions land as ADRs before implementation.

### Design Sketch

`docs/adrs/0006-tonewheel-generator-track.md` records, as accepted
decisions with their rejected alternatives:

- **Component model with one global shared generator** (the five behaviors
  from the Summary), rejecting the per-voice additive extension.
- **Chain position**: the organ bus is
  `generator → contact/bus gains → percussion join → vibrato scanner →
  preamp drive → rotary speaker`, mono until the rotary stage, joining the
  main mix after the voice loop and before the final body/mid-side stage
  (`macro_state.rs:302`) so the existing final saturation, DC blocker, and
  safety limiter still govern the output. Chorus/reverb remain available
  as a shared room; GFM/BCS can process the organ like any other program
  material (the identity twist), with organ demos keeping them off by
  default.
- **Control doctrine**: session-only controls; the nine-digit registration
  vector counts as one control; the five-per-concept cap and the exact
  control lists from the Summary.
- **Naming and trademark policy** as stated in the Summary, binding for
  all future sets too.
- **Calibration strategy**: operator-provided reference recordings,
  `MAMUT_TW_REFERENCE_DIR`, derived-metrics-only evidence, and the harness
  (`SET6-3`) as the measuring instrument.
- **Rotary speaker as a source-agnostic engine-global layer**, not an
  organ-private effect.

### Acceptance

- ADR 0006 lands in `docs/adrs/` following the house ADR format, linked
  from `docs/README.md`; `CLAUDE.md` reading order gains the ADR entry.
- The ADR states the rejected alternatives (per-voice additive extension;
  organ-private rotary; patch-schema-first controls) with one-paragraph
  reasons each.

### Evidence

- the ADR itself plus the `docs/README.md` map update (no render evidence)

### Review Gates

- `sel4-integrated-systems-reviewer` (architecture/docs)

### Size

S

---

## SET6-1: Constants, External References, And Reference-Recording Intake

### Rationale

Everything numeric in this set — wheel frequencies, taper and robbing
curves, click windows, percussion decays, scanner rate and span, rotor
speeds and inertias — must come from pinned, sourced constants, not from
folklore. The operator's reference recordings arrive before implementation
and must be cataloged so later slices can cite segments precisely.

### Design Sketch

`docs/dsp/tonewheel-constants.md` pins, each with source and a confidence
note (or an explicit **open** marker naming the slice that must resolve
it):

- The twelve driver/driven gear-tooth pairs and the resulting wheel 1..91
  frequency table at the nominal motor speed, plus the deviation-from-ET
  table (expected sub-cent to ~1 cent).
- The foldback rule: manual buses have no wheels below 13; taps clamp
  into wheel range [13, 91] by octave steps.
- Drawbar footage → semitone offsets relative to the 8′ fundamental:
  −12, +7, 0, +12, +19, +24, +28, +31, +36 (leftmost digit = −12).
- The drawbar step curve (≈3 dB per step, **pin**), the per-(key, bus)
  taper curve family, and the per-bus loudness-robbing curve target.
- Key-contact bounce statistics: bounce count and time window (**pin**,
  expected ≤3 toggles inside ~0–2 ms) and the per-wheel gain smoothing
  time constant range.
- Percussion: fast/slow decay times, soft/normal levels, the
  sustained-tone attenuation while percussion is on normal, the
  ninth-drawbar mute quirk (the highest drawbar's bus doubles as the
  percussion trigger bus, so that drawbar is silent while percussion is
  enabled), and the percussion join point relative to the scanner
  (**open** — pinned from schematic-level sources; the implementation
  isolates it behind one documented constant).
- Scanner: scan rate (**pin**, expected ≈6.9 Hz), total line delay
  (**pin**, expected ≈1 ms), section/tap count, V1/V2/V3 scan spans,
  C1/C2/C3 dry-mix ratios.
- Rotary: crossover frequency (**pin**, expected ≈800 Hz), chorale and
  tremolo target speeds per rotor (the two rotors differ), the four
  inertia time constants (horn/drum × spin-up/spin-down, horn ≈1 s vs
  drum several seconds), brake behavior, Doppler depth derived from horn
  geometry (**pin**, expected ≈0.3–0.9 ms swing), AM directivity depths
  per rotor.
- Leakage and hum levels (idle-generator noise floor targets), mains
  fundamental choice for hum.

`docs/EXTERNAL_DSP_REFERENCES.md` gains a "tonewheel-organ modeling"
section citing (author-venue-year plus link, per the naming policy): the
DAFx-02 paper on rotary-speaker Doppler simulation, the DAFx-11 paper on
computationally efficient tonewheel-organ synthesis, and the
service-documentation-derived public constant tables used above.

`docs/dsp/tonewheel-reference-set.md` catalogs the operator-provided
recordings: artist, release/track, segment timestamps, and what each
segment demonstrates (steady registration, key click, percussion
staccato, scanner mode, rotary transition, drive). Acceptance requires at
least one usable segment per calibration axis; gaps are listed as open
intake requests to the operator, not silently absorbed.

### Acceptance

- Every constant above is pinned with a source or explicitly marked
  **open** with an owning slice; no constant is "known" without a source.
- The reference catalog exists, `MAMUT_TW_REFERENCE_DIR` is documented,
  and the copyright discipline (no audio in the repo) is stated in the
  catalog header.
- The citation section passes the naming policy (no protected names in
  our prose; author-venue-year citation form).

### Evidence

- `docs/dsp/tonewheel-constants.md`,
  `docs/dsp/tonewheel-reference-set.md`, and the
  `EXTERNAL_DSP_REFERENCES.md` section (documents are the evidence; no
  render)

### Review Gates

- `sel4-integrated-systems-reviewer` (docs/reference truth)

### Size

M

---

## SET6-2: Tonewheel v0.1 — Shared Generator Primitive Plus Offline Render Evidence

### Rationale

The generator is pure, testable math with no engine entanglement: 91
phase accumulators with gear-table frequencies and three gain banks. It
lands first as a `mamut-dsp` leaf module plus an offline render that makes
the shared-generator argument executable — the phase-coherence A/B against
a per-voice-style sine pair is the founding exhibit of the whole set.

### Design Sketch

New module `crates/mamut-dsp/src/tonewheel.rs` (shape guidance):

```rust
pub const WHEEL_COUNT: usize = 91;      // wheel N lives at index N-1
pub const DRAWBAR_COUNT: usize = 9;
pub const MANUAL_KEYS: usize = 61;

pub struct TonewheelGenerator {
    wheels: [Oscillator; WHEEL_COUNT],   // frequencies from the gear table
    freq_hz: [f32; WHEEL_COUNT],         // computed at construction
    keyed_gain: [f32; WHEEL_COUNT],      // smoothed toward keyed_target
    keyed_target: [f32; WHEEL_COUNT],    // block-rate contact×taper×drawbar folds
    perc_gain: [f32; WHEEL_COUNT],       // smoothed toward perc_target
    perc_target: [f32; WHEEL_COUNT],     // block-rate contact×taper folds (pre-drawbar)
    leak_gain: [f32; WHEEL_COUNT],       // static leakage profile (default level 0 until SET6-10)
    level_profile: [f32; WHEEL_COUNT],   // per-wheel output level (flat until SET6-10)
    gain_smooth_coeff: f32,              // from the pinned click smoothing constant
}

pub struct TonewheelFrame {
    pub keyed: f32,        // Σ wheel × keyed_gain × level_profile
    pub percussion: f32,   // Σ wheel × perc_gain × level_profile
    pub leak: f32,         // Σ wheel × leak_gain
}

impl TonewheelGenerator {
    pub fn new(sample_rate_hz: f32) -> Self { /* gear table → freq_hz */ }
    pub fn set_keyed_targets(&mut self, targets: &[f32; WHEEL_COUNT]) { .. }
    pub fn set_perc_targets(&mut self, targets: &[f32; WHEEL_COUNT]) { .. }
    pub fn render(&mut self) -> TonewheelFrame { /* advance all 91, three dot products */ }
}

/// Drawbar tap → wheel mapping with foldback, total over the compass.
pub fn wheel_index(key: usize /* 0..61 */, drawbar: usize /* 0..9 */) -> usize {
    // 13 + key + offset[drawbar], then octave-fold into [13, 91]; -1 for the index
}
```

- Frequencies come from the `SET6-1` gear table times the configured
  sample rate's phase step; clamped below Nyquist margin as a sanitize
  guard only (no wheel needs it at supported rates).
- All 91 wheels advance every `render` call; the three outputs are plain
  dot products; the two gain banks move by one-pole smoothing toward
  their targets every sample (the smoothing constant is what shapes
  click brightness — **tuned in-slice** within the pinned range).
- v0.1 contains **no contact model**: callers fold targets themselves.
  The offline example computes targets from a registration plus a held-key
  list using `wheel_index` and the pinned taper/drawbar curves, so the
  mapping function is exercised before the engine exists.

Offline example `crates/mamut-engine/examples/tonewheel_generator_render.rs`
(hosted in `mamut-engine/examples` — a deliberate divergence from the
SET5-3 `mamut-dsp/examples/` location so that CI's release-smoke job,
which builds `mamut-engine` examples, covers it; the module under test is
`mamut-dsp`):

- Registration scripts: single-drawbar `008000000`, full `888888888`, a
  sparse jazz-style and a dense gospel-style registration (labeled by
  digits only), each over a scripted chord sequence.
- **Phase-coherence A/B (the founding exhibit)**: one two-key interval
  whose drawbar taps share a wheel, rendered (a) through the shared
  generator and (b) through two independent sine pairs detuned per the
  additive idiom (`additive_detune_cents`,
  `crates/mamut-engine/src/state.rs`); the evidence table shows the beat
  products present in (b) and absent in (a), with WAVs for both.
- Foldback audit: a bottom-octave and top-octave run per drawbar,
  rendered and tabled against the expected wheel indices.

### Acceptance

- Unit tests (table-driven, in `mamut-dsp`): the wheel frequency table
  matches the `SET6-1` gear table within f32 tolerance and the
  deviation-from-ET table matches its pinned values; `wheel_index` is
  total over all (key, drawbar) pairs and always lands in [13, 91];
  foldback edge keys map exactly as pinned; hostile inputs (NaN/inf
  targets) sanitize; two identical runs produce identical output
  (FNV-64).
- The example renders all scripts; the phase-coherence A/B shows the
  stated contrast audibly in the WAVs and numerically in the table.
- No allocation in `render` (code-shape review, not a benchmark claim);
  per-block cost table measured and recorded.

### Evidence

- proposed: `docs/dsp/tonewheel-v0.1-generator-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer` (`mamut-dsp` is a gated crate)
- `sel4-rust-single-file-reviewer` on `crates/mamut-dsp/src/tonewheel.rs`

### Size

M

---

## SET6-3: Tonewheel v0.2 — Offline Calibration/Analysis Harness

### Rationale

"Credible, judged by ear" needs a measuring instrument or it decays into
vibes. The harness computes the same metric set on our renders and on the
operator's reference segments, so every later slice can print a delta
table instead of an adjective. Building it immediately after the generator
means every audible slice from `SET6-4` on lands with numbers.

### Design Sketch

Offline example `crates/mamut-engine/examples/tonewheel_calibration.rs`
(offline-only; never on a render path):

- Reads reference segments from `MAMUT_TW_REFERENCE_DIR` per the
  `SET6-1` catalog (a minimal repo-local RIFF PCM 16/24-bit reader — no
  new dependencies). If the directory is unset or a cataloged segment is
  missing, the run **fails loudly with the list of missing segments**;
  render-side metrics can still be produced with an explicit
  `--renders-only` degradation that the evidence doc must mention.
- Metric set v1 (extended in later slices as their axes land):
  - steady-segment harmonic amplitude vectors → estimated registration
    digits;
  - beat-rate detection between drawbar pairs (the gear-table
    signature);
  - key-click spectral centroid and decay time;
  - vibrato/chorus rate and depth, plus a dispersion signature (depth
    versus frequency band);
  - rotary AM and FM rates per band (below/above the crossover), and
    spin-up/spin-down times from transition segments;
  - a drive proxy (odd/even harmonic growth on sustained chords).
- Outputs one-run tables (stdout → evidence doc), per segment and per
  render, in the same units so deltas are direct.

### Acceptance

- **Self-test on ground truth**: registration estimation applied to our
  own `SET6-2` renders recovers the scripted digits within ±1 digit per
  drawbar; beat-rate detection recovers the gear-table prediction within
  a stated tolerance. (The harness is validated against inputs we
  control before it is trusted on references.)
- Runs over the cataloged reference segments and produces the metric
  tables; missing segments fail loudly by name.
- Deterministic: two runs over the same inputs produce identical tables.

### Evidence

- proposed: `docs/dsp/tonewheel-v0.2-calibration-harness-evidence.md`
  (harness self-test tables plus the first reference-segment metric
  tables)

### Review Gates

- `sel4-rust-systems-reviewer` (example in a gated crate)

### Size

M

---

## SET6-4: Tonewheel v0.3 — Engine Integration: Contacts, Busbars, Session Controls

### Rationale

This is the slice where the machine becomes playable: note events close
and open contacts, the busbar network folds gains, robbing and tapering
shape the sum, and the click emerges from switching the live generator.
It is the largest core touch of the set and carries the full baseline
bit-identity mechanics.

### Design Sketch

Engine-owned layer (pattern: `gfm_layer.rs`), e.g.
`crates/mamut-engine/src/tonewheel_layer.rs`:

```rust
pub enum TonewheelMode { Disabled, Enabled }

pub struct TonewheelLayer {
    generator: TonewheelGenerator,
    held: [bool; MANUAL_KEYS],
    registration: [u8; DRAWBAR_COUNT],     // digits 0..=8
    drawbar_gain: [f32; DRAWBAR_COUNT],    // from the pinned step curve, smoothed
    taper: [[f32; DRAWBAR_COUNT]; MANUAL_KEYS], // built at construction from pinned curves
    robbing: [f32; 62],                    // lookup by active-contact count, from pinned curve
    bounce_rng: TonewheelBounceRng,        // fixed seed, advanced per event ordinal
    bounce_queue: BounceQueue,             // bounded (cap tuned in-slice); overflow drops and counts
    out_of_compass_notes: u32,             // visible in the snapshot
}
```

- Key events: the layer consumes the same scheduled note events the voice
  allocator consumes, at the same frame offsets. A key-down closes the
  key's nine contacts; each contact close/open schedules up to the pinned
  bounce count of toggle steps inside the pinned window, at deterministic
  offsets drawn from `bounce_rng` keyed by (note, event ordinal). Gain
  targets refold at the block top plus at each contact event's and bounce
  step's frame offset; per-wheel smoothing in the generator turns those
  steps into the click.
- Target folding (once per block at the top of `process_block`, refolded
  at event frame offsets; bounded by the compass): for each bus, for
  each held key: `w = wheel_index(key, bus)`;
  `keyed_target[w] += taper[key][bus] × drawbar_gain[bus] × robbing[active_contacts(bus)]`;
  `perc_target[w] += taper[key][bus]` for the percussion-designated bus
  only (percussion taps the busbar before the drawbar gain — a drawbar at
  0 still feeds percussion, per the machine).
- Chain in `render_frame`: after the voice loop and before the final
  body/mid-side stage (`macro_state.rs:302`), when enabled:
  `let f = self.tonewheel.render(); left += f.keyed; right += f.keyed;`
  (the percussion and leak sums join in their own slices) — mono center
  until the rotary layer lands (`SET6-8` moves the join into the rotary
  input). Scanner/preamp slots are identity pass-throughs until their
  slices land.
- Session controls (this slice): mode on/off; registration as one atomic
  vector — engine setter `set_tonewheel_registration([u8; 9])` (values
  clamp to 0..=8) plus headless `tonewheel drawbars <9 digits>` and
  `tonewheel status` mirroring the `mozaik set` chain
  (`cli/runtime_commands.rs:67` → `session/commands.rs:74` →
  `session/switching.rs:84`). On-enable default
  registration `888000000` (**tuned in-slice**). Registration changes are
  click-free (smoothed drawbar gains); key contacts click — like the
  machine.
- `EngineSnapshot` gains `tonewheel: TonewheelSnapshot` (mode,
  registration digits, held-key count, per-bus robbing factor,
  out-of-compass counter) for the status line and future `INSPECT` use.

### Acceptance

- Pre-slice baseline rendered at the parent commit; Disabled renders are
  bit-identical, with signatures hardcoded as asserts in the A/B example.
- A/B example `tonewheel_engine_source_ab_render.rs`: registration
  contrasts audible; **key click audible in the WAV and quantified**
  (the GFM probe-audibility lesson: state the click's peak dBFS relative
  to the sustained tone, no inaudible-but-tabled evidence); robbing table
  (steady level versus 1..10 held keys, monotone per the pinned curve);
  foldback boundary script matches `SET6-2`'s mapping audit end-to-end;
  harness delta table for the registration axis.
- Unit tests: contact fold determinism (two-run FNV); registration
  parse/clamp (reject length ≠ 9 or non-digit; clamp digits); robbing
  lookup monotone non-increasing with `r[1] = 1.0`; bounce queue overflow
  drops-and-counts; out-of-compass notes ignored and counted.
- Per-block cost table with a held 10-key chord (steady state), compared
  against the `SET6-2` generator-only cost.

### Evidence

- proposed: `docs/dsp/tonewheel-v0.3-contact-engine-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer` (`mamut-engine` is a gated crate)
- `sel4-integrated-systems-reviewer` (runtime command surface + docs)

### Size

L

---

## SET6-5: Tonewheel v0.4 — Percussion

### Rationale

Percussion is what makes the attack articulate; its trigger logic is the
fiddly part and is exactly the kind of stateful behavior that must land
with a truth table, not a description.

### Design Sketch

- One mono percussion envelope shared by the whole manual (an
  `AdsrEnvelope` in decay-only configuration, or a dedicated one-pole
  decay — in-slice choice): triggers on a key-down **only when no other
  manual key is held**; never retriggers while any key is held (legato
  runs stay silent); re-arms only when all keys are released. Decay time
  fast/slow, level soft/normal from `SET6-1`.
- Source: the percussion bus from `SET6-4` (pre-drawbar tap), harmonic
  selector second (+12 bus) or third (+19 bus).
- The quirks, from the pinned constants: while percussion is enabled on
  normal, the sustained keyed sum attenuates by the pinned amount and the
  ninth drawbar is silent (trigger-bus theft); both restore when
  percussion is disabled. Join point relative to the scanner sits behind
  the single documented constant from `SET6-1`.
- Session control: one compound `percussion` control —
  `off | {second, third} × {fast, slow} × {soft, normal}` — engine setter
  plus `tonewheel percussion <spec>` headless command.

### Acceptance

- Unit-test truth table over scripted sequences: single key from silence
  fires; overlapping second key does not; staccato re-fires only after
  all-keys-up; decay times match pinned constants within tolerance;
  soft/normal levels and the sustained-tone attenuation match; ninth
  drawbar mutes exactly while enabled; determinism (two-run FNV).
- A/B render: the same staccato riff with percussion off / second-fast /
  third-slow; envelope capture table; harness click/attack metrics delta
  against a reference staccato segment where the catalog provides one.
- Disabled percussion leaves the `SET6-4` render bit-identical (hardcoded
  signature assert).

### Evidence

- proposed: `docs/dsp/tonewheel-v0.4-percussion-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`
- `sel4-rust-single-file-reviewer` on the trigger-logic module

### Size

M

---

## SET6-6: Tonewheel v0.5 — Vibrato/Chorus Scanner

### Rationale

The scanner is not a chorus pedal: the real line box is a dispersive LC
ladder, so different frequency bands modulate with different delays. That
dispersion is audible and is the difference between "vibrato effect" and
the instrument's throb. The line-plus-scanner model buys it directly.

### Design Sketch

- A cascade of N line sections (N pinned in `SET6-1`), each a small
  allpass/low-pass lattice built from the `AllpassFilter`
  (`delay.rs:103`) / `OnePoleDamping` idioms — the in-slice choice must
  demonstrate frequency-dependent group delay across the line, matching
  the pinned total line delay.
- Taps after each section; a scan position sweeps triangularly at the
  pinned rate; the output crossfades between adjacent taps (fractional
  read across the tap index, the `read_linear` idiom applied to taps).
- Modes: `off | V1 | V2 | V3 | C1 | C2 | C3` — V modes take the swept
  tap only, with span growing V1→V3; C modes mix the swept tap with dry
  at the pinned ratios. Applied to the mono organ bus at its chain slot
  (before the preamp).
- Session control: one `vibrato` enum — engine setter plus
  `tonewheel vibrato <mode>` headless command.

### Acceptance

- **Dispersion A/B is the core exhibit**: the same nominal depth rendered
  through (a) a plain modulated-delay chorus and (b) the scanner line;
  the evidence shows per-band modulation depth/phase differing in (b) and
  uniform in (a), plus WAVs.
- Harness vibrato rate/depth and dispersion-signature deltas against
  reference scanner segments where cataloged.
- Mode `off` leaves the prior render bit-identical (hardcoded assert);
  V/C modes each produce distinct signatures; two-run determinism; cost
  table.

### Evidence

- proposed: `docs/dsp/tonewheel-v0.5-scanner-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`

### Size

M

---

## SET6-7: Tonewheel v0.6 — Preamp Drive

### Rationale

The preamp is where a held full-registration chord starts to growl. The
existing waveshaper family is a credible first stage; the slice's job is
placement, calibration, and restraint — not a new nonlinearity.

### Design Sketch

- Drive stage on the mono organ bus after the scanner slot and before
  the rotary slot: a blend of `tanh_drive_compensated` (`waveshaper.rs:8`)
  and `asymmetric_diode`, with a gentle pre/post tilt (one-pole) so drive
  brightens the way the reference material does (**tuned in-slice**
  against the harness drive proxy).
- Session control: `drive` (0..1) — engine setter plus
  `tonewheel set drive <v>` headless command. `drive = 0` is a true
  bypass (bit-identical, asserted).
- A wave-digital tube stage is explicitly out of scope; it is the named
  upgrade path only if harness deltas and the operator's ear reject this
  stage at `SET6-11` review.

### Acceptance

- THD-proxy table (odd/even harmonic growth) versus drive on a sustained
  full-registration chord; harness drive-proxy delta against a cataloged
  crunchy reference segment where provided; `drive = 0` bit-identity;
  determinism; cost table.

### Evidence

- proposed: `docs/dsp/tonewheel-v0.6-preamp-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`

### Size

S

---

## SET6-8: Rotary v0.1 — Two-Rotor Core

### Rationale

The rotary speaker is the emotional half of the sound and the one place
in this set where the physics is written as physics: two rotors with
different inertias whose spin-up asymmetry (the horn arrives seconds
before the drum) is the payoff of modeling instead of LFO-faking. It is
deliberately a source-agnostic engine-global layer: the organ is its
first client, not its owner.

### Design Sketch

New `mamut-dsp` block plus an engine-owned layer (pattern:
`gfm_layer.rs`), e.g. `rotary.rs` / `rotary_layer.rs`:

```rust
pub enum RotaryMode { Bypass, Chorale, Tremolo, Brake }

struct Rotor {
    angle: f32,          // wraps [0, 1)
    speed_hz: f32,       // approaches target by one-pole with per-direction tau
    target_hz: f32,
    tau_up_s: f32,
    tau_down_s: f32,
}

pub struct RotarySpeaker {
    crossover_lp: [TptStateVariableFilter; 2],  // cascaded for steeper split
    crossover_hp: [TptStateVariableFilter; 2],
    horn: Rotor,
    drum: Rotor,
    horn_delay: DelayLine,   // Doppler via read_linear; sized at construction
    // per-side AM/Doppler evaluated at two virtual pickup angles (stereo in SET6-9)
}
```

- Input: the mono organ bus (or any mono/mid feed). Crossover at the
  pinned frequency; the high band feeds the horn path: fractional-delay
  Doppler (`DelayLine::read_linear`, depth from the pinned geometry) plus
  an AM directivity lobe (raised-cosine shape, exponent/depth pinned);
  the low band feeds the drum path: shallower AM, gentler high roll-off
  (`OnePoleDamping`), smaller Doppler.
- Mode transitions set rotor targets (chorale/tremolo per-rotor speeds
  from `SET6-1`; brake targets zero); speeds move by per-direction
  one-pole time constants — the horn-leads-drum bloom emerges, it is not
  scripted.
- This slice renders a stereo pair from two fixed virtual pickup angles
  (constants); `SET6-9` makes the field controllable. When the layer is
  `Bypass` the input passes through untouched (bit-identical).
- Session controls (rotary concept): `mode` — engine setter plus
  `rotary mode <bypass|chorale|tremolo|brake>` headless command;
  `balance` (horn/drum mix trim) — `rotary set balance <v>`.
- Chain: the organ bus routes through the rotary layer at the organ's
  chain tail; an A/B example additionally routes an existing
  factory-patch render through the layer session-only (no patch change)
  to prove source-agnosticism.

### Acceptance

- Sine-sweep spectrograms at chorale and tremolo showing FM+AM per band;
  crossover recombination flatness within a stated dB bound at `Bypass`;
  transition captures (chorale→tremolo→brake) with measured per-rotor
  time-to-target compared against the pinned time constants **and**
  against harness spin-up/down times from a cataloged reference
  transition segment (this is the Emerson-school exhibit).
- `Bypass` bit-identity (hardcoded asserts); two-run determinism; cost
  table; the factory-patch A/B renders and is cited as the
  source-agnostic exhibit.
- Unit tests: rotor speed convergence times; angle wrap continuity; NaN
  sanitize on inputs; no allocation in the render path (code-shape
  review).

### Evidence

- proposed: `docs/dsp/rotary-v0.1-two-rotor-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`

### Size

L

---

## SET6-9: Rotary v0.2 — Cabinet, Stereo Field, Drive Interaction

### Rationale

A bare rotor pair sounds like a laboratory. The cabinet — a handful of
early reflections, the amp driving into the rotors, and a controllable
stereo pickup pair — is what turns it into a speaker in a room without
pretending to be a room simulator.

### Design Sketch

- A small fixed set of early reflections per side (delay taps with
  `OnePoleDamping`), levels/times pinned in `SET6-1` as cabinet
  constants (**tuned in-slice**); optional low-shelf cabinet resonance
  (one-pole).
- Amp drive placed **before** the rotors (`tanh_drive_compensated`), so
  drive interacts with the Doppler/AM motion — the classic growl-swirl
  coupling; `drive = 0` bypasses (bit-identical).
- Stereo field: the two virtual pickup angles become controllable —
  `width` scales the angle spread (0 = mono-compatible center).
- Session controls (completing the rotary concept's cap):
  `width` — `rotary set width <v>`; `drive` — `rotary set drive <v>`.

### Acceptance

- Stereo correlation table versus `width` (including the mono-fold
  check); reflection on/off A/B WAVs; harness AM/FM depth deltas against
  reference segments re-measured with the cabinet on (the numbers must
  move toward the reference, and the evidence says whether they did);
  `drive`/`width` zero-settings bit-identity; determinism; cost table.

### Evidence

- proposed: `docs/dsp/rotary-v0.2-cabinet-stereo-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`

### Size

M

---

## SET6-10: Tonewheel v0.7 — Leakage, Hum, Wheel Character (The `wear` Knob)

### Rationale

A factory-fresh model is sterile. The character layer — leakage bleed,
mains hum, per-wheel level irregularity and mild waveform coloration — is
what reads as a real instrument with history. It is one knob by doctrine:
`wear` scales the whole family together.

### Design Sketch

- Leakage: the `leak_gain` bank from `SET6-2` gets its pinned profile and
  a nonzero default level; the leak sum joins the organ bus pre-scanner.
- Hum: mains fundamental plus low harmonics at the pinned level (region
  choice pinned in `SET6-1`).
- Wheel character: `level_profile` gains a deterministic per-wheel spread
  (fixed-seed, pinned variance) and a mild per-wheel-group harmonic
  coloration — implementation choice in-slice between one extra
  precomputed harmonic dot product or folding into the per-wheel level
  tables; must keep the constant-cost rule.
- Session control: `wear` (0..1) scales click excess, leakage level, hum
  level, and level-profile spread along a pinned curve; `wear = 0` is the
  factory-fresh model, bit-identical to the pre-slice render (asserted).
- Engine setter plus `tonewheel set wear <v>` headless command.

### Acceptance

- Idle-organ noise-floor spectral table versus `wear`, compared against
  the harness's reference noise-floor metrics where an idle segment is
  cataloged; `wear = 0` bit-identity; audibility statement for the
  default `wear` (the click-audibility lesson applies to leakage too);
  determinism; cost table.

### Evidence

- proposed: `docs/dsp/tonewheel-v0.7-character-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`

### Size

M

---

## SET6-11: Consolidation — Cost Table, Docs Truth, Demo Script, Listening Checklist

### Rationale

The set ends the way Set 5 planned to: one honest ledger across the
concept, a reproducible demo, and the docs telling the same truth as the
code.

### Design Sketch

- **Cross-slice cost table**: one binary, one run per stage combination
  (generator only; +contacts; +percussion; +scanner; +preamp; +rotary;
  +character), at 48 kHz and 96 kHz, per-block cost and headroom on the
  reference host.
- **Honesty ledger**: nearest relatives named plainly (commercial
  tonewheel emulations; the two cited papers); what this implementation
  claims (calibration discipline, evidence-first engineering, the
  constant-cost shared generator inside this engine's identity) and what
  it does not claim (novel DSP).
- **Demo**: a `scenarios/*.toml` note script for `mamut-seq` plus a
  documented headless command sequence (registration moves, percussion
  spec, vibrato mode, rotary transitions) that reproduces the full-stack
  demo from a cold start; referenced from the evidence doc.
- **Listening checklist**: `docs/factory-bank-listening-checklist.md`
  gains a tonewheel section (registration purity, click character,
  robbing feel, scanner throb, rotary bloom, wear floor).
- **Docs truth pass**: `docs/README.md`, `README.md`, `CLAUDE.md` updated
  where the set touched reality; GUI `INSPECT` exposure explicitly
  remains deferred (optional stretch, cut without ceremony).
- **Final harness sweep**: the full metric set over the demo render
  versus the reference catalog, with per-axis deltas and the operator's
  accept/iterate call recorded.

### Acceptance

- The cost table, ledger, demo, checklist section, and harness sweep all
  exist and are referenced from one consolidation evidence doc; every
  open **pin** from `SET6-1` is either resolved or explicitly carried
  forward with an owner; the operator's listening verdict is recorded.

### Evidence

- proposed: `docs/dsp/tonewheel-v1.0-consolidation-evidence.md`

### Review Gates

- `sel4-integrated-systems-reviewer` (docs/architecture truth)

### Size

S

---

## Deferred Follow-Ups (Future Contract Set — Not Set 6)

Named so they are not re-litigated per slice:

- **Patch schema**: `[engine.tonewheel]` / `[engine.fx.rotary]` tables,
  schema version bump, factory-adjacent patches — a separately earned
  contract set (the note-strike precedent).
- **Controller-profile drawbar bindings**: mapping a nine-fader surface
  (the `PC4` has nine physical sliders) to the registration vector via
  the `mozaik_control` binding-kind precedent (SET5-8).
- **GUI**: `SOUND`/`INSPECT` screen modules for registration, percussion,
  scanner, rotary state (post ADR 0004 cutover).
- **`EPM2` parity**: identity/param surface alignment via the
  hardware-software pair review once any contract surface exists.
- **Expression/swell input** (post-SET4 UMP expressiveness) and
  velocity→click mapping as UMP-era slices.
- **Second manual and pedal division** (complex-wave pedal wheels are a
  different generator segment); out of scope for the 61-key compass.
- **Circuit-level upgrades** (wave-digital tube stage, solved busbar
  network, scanner circuit model) — only if `SET6-11`'s harness deltas
  and the operator's ear reject the component-model stages.
