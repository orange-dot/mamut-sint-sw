# EPM1 Backlog Set 5: Orbita, Mozaik, Kosava — Three New DSP Worlds

Date: 2026-07-12

Status: active. `SET5-3` and `SET5-4` (the Mozaik track) landed 2026-07-12
(commits `1ec7ddf`, `c8edd46`, review-fix `04375f9`); `SET5-1/2`, `SET5-5/6`,
and `SET5-7` are not started.
Each item ships as its own slice with its own evidence document and review
gates. Evidence file names are proposals; final numbering is assigned at
landing time, continuing each concept's own `docs/dsp/` chain the way
`gfm-v*` grew.

## Summary

Operator decisions (2026-07-12) — these are inputs to this backlog, not open
questions:

- **Three concepts selected**: `Orbita` (celestial mechanics: resonance
  capture, tidal dissipation, Roche breakup), `Mozaik` (quasicrystal
  order: cut-and-project oscillator with phason modulation), `Kosava`
  (wind engineering: gust spectra, vortex lock-in, flutter). They were
  chosen from a wider exploration; the rejected candidates and the full
  rationale live in the operator discussion, not here.
- **Second pillar, not new GFM regimes.** GFM's innovation lane (a hidden
  excitable lattice read by probes) stays what it is; the speculative GFM
  regime notes (`docs/dsp/gfm-wild-concepts.md`: Lavina, Brazda, Mraz) own
  that direction. Set 5 concepts are standalone DSP worlds at other
  architectural layers: a new oscillator class (`Mozaik`), a per-voice
  dynamics layer (`Orbita`), an ensemble excitation layer (`Kosava`).
  Biology is explicitly excluded as a source domain for this set.
- **Evidence-first, session-only.** Every concept walks the GFM ladder:
  offline render example plus evidence doc first, engine integration
  second, session-only controls third. No patch-schema growth anywhere in
  this set; a future contract set has to earn that separately (the
  note-strike precedent).
- **Naming** follows the ASCII identity convention (`Pec`, not `Peć`):
  `Orbita` = orbit, `Mozaik` = mosaic, `Kosava` = the Serbian wind.

Items:

1. `SET5-1` — Orbita v0.1: resonance-capture dynamics primitive
   (`mamut-dsp`) plus offline render evidence
2. `SET5-2` — Orbita v0.2: engine per-voice moon layer plus session
   controls
3. `SET5-3` — Mozaik v0.1: quasicrystal oscillator primitive
   (`mamut-dsp`) plus offline render evidence — **landed** (`1ec7ddf`)
4. `SET5-4` — Mozaik v0.2: engine voice-source integration plus session
   controls — **landed** (`c8edd46`, review-fix `04375f9`)
5. `SET5-5` — Kosava v0.1: gust-field and vortex lock-in primitives
   (`mamut-dsp`) plus offline render evidence
6. `SET5-6` — Kosava v0.2: engine ensemble wind layer plus session
   controls; the meteorological-arpeggiator demo
7. `SET5-7` — consolidation: cross-concept cost table, docs truth,
   honesty ledger
8. `SET5-8` — session-layer controller bindings: play the Set 5 layers
   from MIDI CC (touch surface / any controller) via the controller
   profile

## Assessment Inputs (What The Code Says Today)

Findings from the 2026-07-12 pre-backlog assessment, anchored to source:

- **`mamut-dsp` already carries every support primitive the three concepts
  need**: `NoiseRng` (seeded 32-bit LCG, deterministic),
  `color_noise_sample`, `PhaseAccumulator`, `LinearSmoother`,
  `SlewLimiter`, `sine_phase_sample`, `cubic_soft_clip`,
  `StateVariableFilter` / `TptStateVariableFilter`, and the safety kit
  (`sanitize_sample`, `flush_tiny_sample`, `DcBlocker`,
  `master_safety_limit`). All three v0.1 slices are new leaf modules
  beside these, using them, not modifying them.
- **The engine-owned layer precedent exists twice.** `GfmLayerMode
  { Disabled, Enabled { seed } }` plus `Engine::set_gfm_layer_mode`
  (`crates/mamut-engine/src/engine/lifecycle.rs`) is the model for a
  session-only, opt-in, *seeded* layer; `BcsLayerMode` /
  `set_bcs_layer_mode` (scenario-parameterized, `Enabled { scenario }`,
  not seeded) is the second precedent for the mode/snapshot/smoothing
  machinery. `Orbita` and `Kosava` copy the GFM seed shape; `Mozaik`
  follows the same mode/snapshot idiom as a per-voice source blend.
- **The offline evidence path is established.**
  `crates/mamut-field/examples/bcs_v0_1_render.rs` is the house pattern:
  deterministic scenario list, float-WAV writer local to the example, one
  printed metrics line per scenario. `crates/mamut-engine/examples/`
  (`gfm_engine_layer_ab_render.rs`, `audio_baseline.rs`,
  `core_output_safety_sweep.rs`) is the engine-side evidence and timing
  path; CI's release-smoke builds all engine examples.
  `crates/mamut-dsp` has no `examples/` directory yet; this set creates
  it. Note: dsp examples are built and run by the evidence commands, not
  by CI's release-smoke (which builds `mamut-engine` examples only);
  wiring dsp examples into CI is optional follow-up, not part of this
  set.
- **Session-control plumbing precedent**: the GFM layer reaches the
  runtime through a CLI option (`gfm_layer_seed`,
  `crates/mamut-runtime/src/commands.rs::gfm_layer_mode_from_seed`) and
  engine setters called where the session owns the engine. New session
  controls in this set follow that route plus the headless command
  surface; the GUI is out of scope for this set (ADR 0004 boundary
  untouched).
- **The per-event global resolve is expensive** (`SET4` assessment:
  `refresh_resolved_state()` runs a full identity + 105-parameter resolve
  per controller event). Set 5 concept parameters are engine-local state
  set by dedicated setters; they must not ride the patch-parameter resolve
  path at all.
- **Voices are 6, with rich per-voice state and deterministic seeds**
  (`crates/mamut-engine/src/state.rs::VoiceState`), so per-voice concept
  state (Orbita moon banks, Kosava lock-in strings) has a natural home
  and a natural seed source.

## Shared Boundary Constraints

- **Workspace lint posture untouched**: `unsafe_code = "forbid"`; clippy
  warns on `dbg!`/`expect`/`panic`/`todo`/`unwrap` stay warnings-clean.
  All three concepts are pure safe Rust with no new external
  dependencies; `mamut-dsp` keeps its zero-dependency posture.
- **Realtime discipline** (ADR 0001 standing rules): allocation-free in
  steady state on render paths, no logging/formatting/blocking there,
  bounded everything. Every concept's audio-rate state is fixed-size and
  always allocated; behavior differences are gain-gated, never
  allocation-gated.
- **Deterministic and bounded** (the `gfm-wild-concepts.md` reading
  contract, binding here): same seed plus same event stream produces
  bit-identical output; fixed iteration budgets; clamped fields; no
  hidden solvers. Two-run FNV-64 signature equality is a mandatory test
  in every evidence doc.
- **Opt-in and bounded in the mix**: each concept is a layer or source
  blend over the Mamut voice, disabled by default, exactly like the GFM
  layer. With a concept `Disabled`, the render output must be
  bit-identical to the pre-slice baseline. The baseline signatures are
  rendered at the parent commit, recorded in the evidence doc, **and
  hardcoded as asserts inside the A/B example itself** so the
  bit-identity bar is executable on every run, not prose (the `SET5-4`
  landing proved this mechanic; it is the standard for `SET5-2` and
  `SET5-6`).
- **No patch-schema growth.** `schema_version` stays 1; factory patches
  and the live set stay locked; all controls are session-only (CLI flag,
  headless command, engine setter). GUI exposure (including `INSPECT`
  views) is at most an optional stretch in `SET5-7` and may be cut
  without ceremony.
- **Session controls are capped at five per concept** (plus the on/off
  mode). If a design wants a sixth knob, something is wrong with the
  model; redesign instead of adding it.
- **Session-layer knob values do not survive a runtime engine rebuild**
  (patch or audio-config switch reapplies only the mode/seed with the
  on-enable defaults). This matches GFM/BCS and is accepted for the
  whole set — stated here once so it is not re-litigated per slice or
  re-flagged per review.
- **SET4 independence.** Nothing in this set touches transport, MIDI
  parsing, queue payloads, or the callback. Per-note extension points
  (Orbita perturbation kicks, Kosava per-note wind boost) are *named* in
  the design sketches but wired only after `SET4-3/4` land, as their own
  future slices. Set 5 must never block on Set 4, and vice versa.
- **Honesty discipline** (`masnoca.md` precedent): every evidence doc
  carries a nearest-relatives section separating established prior art
  from what is believed new here. Wild is the goal; overclaiming is not.
- **Docs and code describe the same truth in the same change**
  (`CLAUDE.md`, `AGENTS.md`, `README.md`, `docs/README.md` where
  touched).
- The mandated gates in `docs/review-gates.md` apply to every slice in
  full (including the single-file pass where that table requires it, and
  the integrated pass whenever a slice touches `docs/` — which every
  slice does via its evidence doc); the per-item gate lists below name
  the slice-specific *additions*, not the whole set. The helper scripts
  under `tools/review/` print the prompts.

## Execution Protocol For The Implementing Agent

Read these before the first line of code, in this order: `AGENTS.md`,
`CLAUDE.md`, this backlog, `docs/review-gates.md`,
`docs/dsp/gfm-wild-concepts.md` (for the reading contract and honesty
style), and one landed evidence doc
(`docs/dsp/gfm-v2.2-stereo-probe-evidence.md`) as the format model.

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
   caveats, nearest-relatives section).
5. Run the mandated review gates (`tools/review/*.sh` print the prompt
   and scope) and address findings.
6. Update `docs/README.md` (and `README.md`/`CLAUDE.md` only where the
   slice says so) in the same change.

Numbers marked **tuned in-slice** below are starting points chosen for
plausibility, not measurements; the slice must keep or move them based on
rendered evidence and say which happened. Struct sketches are shape
guidance, not frozen names — but parameter *semantics*, clamps, and
determinism requirements are binding.

---

## SET5-1: Orbita v0.1 — Resonance-Capture Dynamics Primitive

### Rationale

The payload of Orbita is a new kind of per-voice motion: partials
("moons") that drift toward, get captured by, and escape from
small-integer frequency ratios, with dissipation as the playable material
constant. That dynamic is testable pure math with no engine entanglement,
so it lands first as a `mamut-dsp` leaf module plus an offline render
that proves the capture behavior is audible, deterministic, and bounded.

### Design Sketch

New module `crates/mamut-dsp/src/orbit.rs`, exported via `lib.rs`:

- **Resonance table**: a `const` array of small-integer ratios spanning
  `[1, 4]`, each entry `(p, q, value, weight)` with `value = p/q`,
  `weight = 1.0 / (p*q)` (a consonance heuristic, stated as such):

  ```text
  (1,1) (5,4) (4,3) (3,2) (8,5) (5,3) (7,4) (2,1)
  (9,4) (7,3) (5,2) (8,3) (3,1) (7,2) (4,1)
  ```

- **Kirkwood rule**: a control `kirkwood` in `[0, 1]` marks entries
  *repulsive* when `weight < 0.12 * kirkwood`. At `0` every entry
  attracts; near `1` only the strong harmonics (`1/1`, `3/2`, `2/1`,
  `3/1`, `4/1`, …) attract while complex ratios repel — the field clears
  gaps the way Jupiter clears the asteroid belt. Repulsion uses the same
  force law with the sign flipped and the same clamps.
- **Moon state** (per moon): `ratio: f32` (current frequency ratio to the
  fundamental), `captured: Option<u8>` (table index), `hold: u16`,
  `ring_amount: f32`, `phase: [f32; 3]` (center plus two ring
  oscillators, always allocated, gain-gated). `OrbitalSystem<const K:
  usize>` owns `K` moons; v1 uses `K = 5`.
- **Init**: moons start at seeded, slightly detuned ratios (seeded from a
  `u64` the caller passes; use a small splitmix-style fold like the field
  crate does, or `NoiseRng` — decided in-slice) spread over `[1.2, 3.8]`,
  never exactly on a table value.
- **Tick** (control rate; `ORBITA_TICK_SAMPLES = 32`): for each moon,
  find the nearest table entry within a relative window
  `|r - v|/v < 0.06`; apply
  `r += -sign * rate * weight * (r - v)` where `sign` flips for
  repulsive entries and `rate = dissipation^2 * 0.12` per tick (tuned
  in-slice; the square gives the macro a usable low end). Clamp `r` to
  `[0.5, 6.0]` and sanitize (non-finite resets to the seeded init value).
- **Capture/escape hysteresis**: capture when `|r - v|/v < 0.002`
  (≈3.5 cents) holds for 8 consecutive ticks — then `r` slews to exactly
  `v` and stays. Escape only when a perturbation pushes
  `|r - v|/v > 0.015` (≈26 cents). Entry is ~7× tighter than exit; the
  lock has to feel like a detent, not a filter.
- **Perturbation**: `kick(&mut self, amount: f32)` adds a bounded,
  seed-deterministic per-moon offset (clamped `|Δr| ≤ 0.05 * amount`).
  This is the future SET4 per-note-bend/pressure entry point; in v0.1 it
  is driven by the scenario script.
- **Roche breakup**: control `proximity` in `[0, 1]`; per-moon threshold
  `roche_k = 0.55 + 0.08 * k` (staggered so moons shatter in sequence).
  `excess = smoothstep((proximity - roche_k) / 0.15)`; `ring_amount`
  slews toward `excess` (attack ≈ 30 ms, release ≈ 300 ms — asymmetric on
  purpose: breaking is fast, re-coalescing is slow). Output per moon:
  center sine at `f0 * r` with gain `1 - (2/3) * ring_amount`, plus a
  ring pair at `f0 * r * (1 ± δ)` with `δ = 0.02 + 0.10 * ring_amount`
  and gain `ring_amount / 3` each. Fixed three oscillators per moon,
  always running, gain-gated — allocation-free by construction.
- **Render**: `render_sample(f0_hz, sample_rate) -> f32` sums the moon
  bank (`K * 3 = 15` sines via `sine_phase_sample`), amplitude profile
  fixed per moon (`1 / (k + 1)`, normalized), output through
  `flush_tiny_sample` and a final clamp. No allocation, no branching on
  hot paths beyond the gain gates.

Everything above is pure: no I/O, no time source, no global state. All
"time" is tick counts; all randomness is the caller's seed.

Offline example `crates/mamut-dsp/examples/orbita_v0_1_render.rs`
(float-WAV writer copied from the `bcs_v0_1_render.rs` pattern, metrics
line per scenario) rendering scenarios:

| Scenario | Script | What it proves |
| --- | --- | --- |
| `capture_slow` | `dissipation = 0.25`, no kicks | audible glide into lock over seconds; capture times per moon |
| `capture_fast` | `dissipation = 0.9` | sub-second snap; contrast with `capture_slow` |
| `escape_kick` | capture, then scripted kicks of rising size | hysteresis: small kicks bend and return, one big kick escapes |
| `kirkwood_sweep` | `kirkwood 0 -> 1` over the render | complex ratios audibly evacuate; final ratio histogram |
| `roche_sweep` | `proximity 0 -> 1 -> 0` | staggered breakup and slow re-coalescing; ring metrics |
| `determinism` | `capture_slow` twice | FNV-64 equality, byte-identical |

Each scenario prints: per-moon final ratio, captured table entry (or
none), capture time in ticks, ring peak, RMS/peak, FNV-64 signature.

### Acceptance

- Unit tests (table-driven, in `mamut-dsp`): capture time monotone in
  `dissipation`; hysteresis band verified (kick just below exit threshold
  returns to lock, just above escapes); kirkwood flips force sign;
  ratios stay clamped and finite under hostile inputs (NaN/inf controls
  sanitize); two identical runs produce identical state.
- The offline scenarios above render with the stated contrasts audible in
  the WAVs and visible in the metrics.
- No allocation in `tick`/`render_sample` (code-shape review, not a
  benchmark claim).

### Evidence

- proposed: `docs/dsp/orbita-v0.1-capture-dynamics-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer` (`mamut-dsp` is a gated crate)

### Size

M

## SET5-2: Orbita v0.2 — Engine Per-Voice Moon Layer Plus Session Controls

### Rationale

Orbita becomes playable: each sounding voice carries its own
`OrbitalSystem` keyed to the voice's note, mixed into the voice pre-filter
so moons ride the voice's envelope and filter naturally. Session-only
controls expose the four continuous axes plus the momentary `kick`
(five controls, at the cap); nothing touches the patch schema.

### Design Sketch

- `OrbitaLayerMode { Disabled, Enabled { seed } }` plus
  `Engine::set_orbita_layer_mode` and an `OrbitaLayerSnapshot` (per-voice
  captured-count, mean ring amount, the control values), mirroring the
  `GfmLayerMode` / `BcsLayerMode` idiom in `api.rs` and
  `engine/lifecycle.rs`.
- Per-voice `OrbitalSystem<5>` lives beside the other per-voice DSP state
  in `VoiceState`; seeded from the layer seed folded with the voice's
  existing deterministic seed; reset on voice trigger (retrigger =
  a fresh system — moons re-capture per note, which is the musical
  point: slow dissipation means long notes crystallize and short notes
  never do).
- Tick runs at `ORBITA_TICK_SAMPLES` inside the voice render loop's
  existing block structure (off the per-sample path); render adds the
  moon-bank sample scaled by `orbita_mix` into the voice signal
  **pre-filter** (decision: pre-filter, so the voice filter and envelope
  shape the moons; revisit only with rendered A/B evidence).
- Controls (engine setters + `LinearSmoother` each, sanitized like
  `sanitize_gfm_selection_score`): `mix` `[0,1]`, `dissipation` `[0,1]`,
  `proximity` `[0,1]`, `kirkwood` `[0,1]`, and momentary `kick`
  (amount `[0,1]`, fire-and-decay). Five controls, at the cap.
  On-enable defaults (also the `reset_controllers` restore point):
  `mix = 0.35`, `dissipation = 0.5`, `proximity = 0`, `kirkwood = 0`,
  `kick` idle — exact values tuned in-slice under one binding
  constraint: a bare `orbita on` must be audible on a held `dry-run`
  note without any `set`.
- Runtime plumbing, following the GFM layer route: a `play`/`dry-run`
  CLI option (`--orbita [<seed>]`, default off) wired where
  `gfm_layer_mode_from_seed` is wired
  (`crates/mamut-runtime/src/commands.rs`), plus headless commands:
  `orbita` (status line), `orbita on|off`, `orbita set <param> <0..1>`,
  `orbita kick [<amount>]`. `EngineCommand` grows the matching variants
  riding the existing bounded control queue (not the priority path).
  The optional-seed form (`--orbita` alone enables with a default seed;
  `--orbita 42` seeds explicitly) deliberately copies the heuristic
  `SET5-4` landed for `--mozaik [<seed>]`. It is asymmetric with
  `--gfm-layer-seed` (mandatory arg) by design and safe because no
  factory patch is digit/hex-named; copy the `SET5-4` shape verbatim so
  a reviewer does not re-flag the asymmetry per concept. This slice also
  adds the `orbita_control` profile binding kind by mirroring the
  `SET5-8` `mozaik_control` shape (kind + action + routing through the
  same control path as the headless `set`).
- `reset_controllers` clears Orbita control smoothing to defaults;
  `panic` already silences voices, which resets systems on the next
  trigger — state that in the evidence, don't add special cases.
- New engine example `crates/mamut-engine/examples/
  orbita_engine_layer_ab_render.rs` (CI builds it): renders one factory
  patch A/B — layer `Disabled` vs `Enabled` under a fixed scripted note
  sequence — printing both signatures and the layer metrics.

### Acceptance

- **Disabled bit-identity**: with `OrbitaLayerMode::Disabled`, the A/B
  example's "A" signature equals the pre-slice baseline signature for
  the same command (baseline rendered at the parent commit, recorded in
  the evidence doc, and asserted in the example — the `SET5-4`
  mechanic).
- Enabled render shows per-voice independence: two overlapping notes
  where one voice's moons capture while the other's stay loose (scripted
  via note timing against a slow `dissipation`).
- Headless `orbita` round-trip works in `dry-run`; `status` shows the
  snapshot line; controls clamp and sanitize hostile values.
- `cargo test --locked` green; existing suite untouched.
- Per-block cost of the enabled layer measured with the
  `audio_baseline.rs` harness pattern and recorded (budget check happens
  in `SET5-7`; this slice only measures honestly).

### Evidence

- proposed: `docs/dsp/orbita-v0.2-moon-layer-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`
- `sel4-rust-execution-optimizer` (voice render loop is a hot path)

### Size

M–L

## SET5-3: Mozaik v0.1 — Quasicrystal Oscillator Primitive

### Rationale

Mozaik is the one concept whose novelty lives in the *source* itself: an
oscillator whose waveform is aperiodic yet perfectly ordered
(cut-and-project / Fibonacci-word structure), with a modulation axis —
the phason — that periodic oscillators cannot have. It is O(1) state,
integer-deterministic, and guaranteed audible, which also makes it the
cheapest first proof that this set's evidence discipline works.

### Design Sketch

New module `crates/mamut-dsp/src/quasicrystal.rs`, exported via `lib.rs`:

- **The word generator is a Bresenham accumulator.** State: `frac: u32`
  (Q32 fixed-point fractional position), advanced per tile by
  `slope_q32: u32` (the slope `σ` in Q32). The carry of
  `frac.overflowing_add(slope_q32)` *is* the tile kind:
  carry ⇒ `L`, no carry ⇒ `S`. This is exactly
  `⌊(n+1)σ + φ⌋ − ⌊nσ + φ⌋` with zero floating-point drift, deterministic
  forever, no tables, no allocation. The phason `φ` is the accumulator's
  seat: shifting it is `frac = frac.wrapping_add(Δφ_q32)`.
- **Slope is the order macro.** `σ` clamps to `[0.45, 0.75]`. At the
  golden value `σ* = 1/τ ≈ 0.6180339887` the word is the Fibonacci word
  (maximally quasiperiodic); at rational detents it is periodic and the
  spectrum collapses to harmonic. Ship `const` Q32 detents for
  `1/2, 3/5, 5/8, 2/3` and `1/τ`. (Q32 represents rationals only to
  2⁻³² — the "periodic" detents are astronomically-long-period
  approximations; state this in the evidence rather than hiding it.)
- **Tiles are Hann pulses.** Tile duration:
  `dur_samples = M * d_kind / d̄` with `d_S = 1`, `d_L = γ`
  (`γ` = contrast, clamp `[1.0, 2.2]`, default `τ`),
  `d̄ = (1-σ) + σγ`, and `M = sample_rate / (2 * f0)` so the mean tile
  rate is `2·f0`. Tile length is floored at
  `MOZAIK_MIN_TILE_SAMPLES = 4`: when `M * d_kind / d̄` computes below
  the floor (high `f0` at 44.1/48 kHz), the floor holds and the top
  octave degrades gracefully — the floor biases the mean tile rate up
  there, so the pitch-anchor table must include `f0 = 8000` Hz at
  48 kHz and the bias is a recorded number, not a surprise. Waveform
  per tile: a full-tile Hann window
  `0.5 * (1 - cos(2π·u))` with polarity `+` for `L`, `−` for `S`. Hann
  endpoints have zero value *and* zero slope, so tile joins are C¹ and
  the spectrum rolls off steeply without polyBLEP; the alias floor and
  the DC component (L/S asymmetry produces some) are measured in the
  evidence, and the existing `DcBlocker` is the stated downstream answer
  for the engine slice.
- **Phason changes latch at tile boundaries** (`pending_phason` applied
  when a tile ends), so a phason sweep is click-free by construction and
  audible as discrete micro-rearrangements of the pulse fabric at
  constant pitch — the axis that makes Mozaik Mozaik.
- **Params**: `f0` `[20, 8000]` Hz, `slope`, `contrast`, `phason`
  (free-running wrap), `gain`. Struct sketch:
  `QuasicrystalOsc { frac: u32, slope_q32: u32, tile_pos: f32,
  tile_len: f32, tile_sign: f32, pending_phason: Option<u32> }` plus a
  `next_sample(f0, sample_rate) -> f32` that computes the next tile's
  kind/length only at boundaries. No RNG anywhere — the module is
  seedless and purely structural (a `seed` in the engine slice merely
  offsets the initial phason).

Offline example `crates/mamut-dsp/examples/mozaik_v0_1_render.rs`
(house WAV/metrics pattern) rendering:

| Scenario | Script | What it proves |
| --- | --- | --- |
| `golden` | `σ = 1/τ`, fixed `f0` | stable discrete spectral peaks; autocorrelation shows no exact period |
| `detent_walk` | `σ` stepped through the detents | harmonic at rationals, golden comb between; peak tables per step |
| `slope_morph` | continuous `σ` sweep | the crystal↔quasicrystal morph is continuous and click-free |
| `phason_drift` | fixed `σ*`, slow phason ramp | constant pitch, audible rearrangement; flip count vs phason delta |
| `contrast` | `γ` `1.0 → 2.2` | timbre axis; DC and alias floor across the range |
| `determinism` | `golden` twice | FNV-64 equality |

Metrics per scenario: strongest-peak frequency vs `f0` (the pitch-anchor
honesty number), top-8 FFT peak table (an in-example DFT over a windowed
segment is fine — no external tools), autocorrelation peak lag, DC mean,
estimated alias floor, RMS/peak, FNV-64.

### Acceptance

- Unit tests: carry-word matches a directly computed
  `⌊(n+1)σ+φ⌋ − ⌊nσ+φ⌋` reference over ≥10⁵ tiles for the detents and
  `σ*`; L-tile density equals `σ` within 10⁻³ over 10⁶ tiles; phason
  latch never splits a tile; hostile params clamp; determinism.
- `golden` yields a stable, discrete, non-harmonic peak set;
  `detent_walk` shows the harmonic collapse at rationals.
- Strongest-peak-vs-`f0` relation documented (Mozaik does not promise
  that perceived pitch equals `f0`; it promises the relation is stable
  and stated).

### Evidence

- proposed: `docs/dsp/mozaik-v0.1-quasicrystal-osc-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`

### Size

M

## SET5-4: Mozaik v0.2 — Engine Voice-Source Integration Plus Session Controls

### Rationale

Mozaik joins the voice as a blendable oscillator source so it is played
like any Mamut sound — through velocity, envelopes, the filter, and the
existing safety chain — rather than remaining a lab curiosity.

### Design Sketch

- `MozaikMode { Disabled, Enabled { seed } }` + `set_mozaik_mode` +
  `MozaikSnapshot`, same idiom as `SET5-2` (the seed offsets the initial
  phason per voice, folded with the voice seed).
- Per-voice `QuasicrystalOsc` in `VoiceState`, always present,
  gain-gated by `mozaik_mix`. Insertion: summed with the existing
  oscillator mix **pre-filter**, tracking the voice frequency the same
  way the existing oscillators do (same `f0` source, no private tuning).
  Reset (`frac` reseat to the seeded phason) on voice trigger.
- Controls (setters + smoothers, capped): `mix` `[0,1]`,
  `slope` `[0,1]` mapped to `σ ∈ [0.45, 0.75]` with a gentle detent snap
  (within ±0.004 of a detent, snap; the snap is in the *mapping*, not
  the oscillator; where zones would overlap the nearest detent wins,
  and the golden value beats `5/8` — they sit ≈0.007 apart),
  `contrast` `[0,1]` → `γ ∈ [1.0, 2.2]`,
  `phason` `[0,1]` → phason offset, `drift` `[0,1]` → slow auto-phason
  rate (0 = frozen). Five controls, at the cap. On-enable defaults
  (also the `reset_controllers` restore point): `mix = 0.35`, `slope`
  at the golden detent, `contrast` at `γ = τ`, `phason = 0`,
  `drift = 0` — tuned in-slice under the same binding constraint as
  Orbita: a bare `mozaik on` must be audible on a held `dry-run` note.
- Runtime plumbing exactly as `SET5-2`: `--mozaik [<seed>]`, headless
  `mozaik` / `mozaik on|off` / `mozaik set <param> <0..1>`,
  `EngineCommand` variants on the bounded queue, `status` line from the
  snapshot.
- New engine example
  `crates/mamut-engine/examples/mozaik_engine_source_ab_render.rs`:
  factory patch, fixed note script, A/B `Disabled` vs `Enabled` at two
  slope settings (a detent and `σ*`); prints signatures and metrics.

### Acceptance

- Disabled bit-identity against the pre-slice baseline (same bar and
  method as `SET5-2`).
- Enabled: the blend passes through the voice filter and envelope
  (rendered evidence: filter cutoff sweep audibly shapes Mozaik);
  `drift` produces the constant-pitch rearrangement on a held note.
- Headless round-trip in `dry-run`; hostile values clamp; suite green;
  per-block cost measured via the `audio_baseline.rs` pattern.

### Evidence

- proposed: `docs/dsp/mozaik-v0.2-voice-source-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`
- `sel4-rust-execution-optimizer` (voice render loop)

### Size

M

## SET5-5: Kosava v0.1 — Gust Field And Vortex Lock-In Primitives

### Rationale

Kosava's claim is that *engineering wind* — a gust process with the right
spectral shape, plus vortex lock-in with hysteresis — makes an ensemble
of notes behave like cables in weather. Both halves are pure DSP with
crisp measurable properties, so they land first as `mamut-dsp` leaf
primitives with offline proof, before any engine wiring.

### Design Sketch

New module `crates/mamut-dsp/src/wind.rs`, exported via `lib.rs`:

- **`GustField`** (control-rate; tick every `KOSAVA_TICK_SAMPLES = 64`
  samples): seeded white noise (`NoiseRng`) through three one-pole
  low-pass stages with cutoffs `0.05 / 0.5 / 5.0` Hz (coefficients
  computed from the actual tick rate) weighted `1.0 / 0.45 / 0.18`
  (weights tuned in-slice), normalized to unit-ish variance, clamped to
  `±3.0`. Target: power-spectrum slope ≈ `f^(-5/3)` (the Kaimal inertial
  range) over `[0.05, 5]` Hz within ±2 dB, **measured by the example's
  own periodogram over a long offline run and printed** — the slope is
  an evidence number, never an assumed property. Output:
  `wind(t) = wind_mean * max(0, 1 + intensity * 0.33 * g(t))`, slewed
  (~50 ms).
- **`VortexString`** (one per future voice; here standalone): given a
  structural frequency `f_v` and the wind:
  - shedding frequency `f_s` = log-map of normalized wind over the wind
    band `[110, 1760]` Hz (band constants; the engine slice may re-map);
  - **lock-in with hysteresis**: enter when `|log2(f_s/f_v)| < 0.10`
    oct, leave when `> 0.30` oct. Inside, excitation `e` rises toward 1
    with attack ≈ 120 ms scaled by gust energy, shaped through
    `cubic_soft_clip` (growth saturates — the honest nod that this
    borrows BCS's saturation *shape* while the excitation physics is
    external drive plus synchronization, not an autonomous oscillator);
    outside, `e` decays with τ ≈ 400 ms.
  - **synchronization pull**: while locked, the sounding frequency
    `f_w` slews from `f_s` toward `f_v` with pull `κ = 0.85` — the
    whistle *finds* the string, which is what real vortex lock-in does.
  - output: `sine(f_w) * e * a_tone + bandpass(gust noise, f_w, Q≈8)
    * e * a_breath` (SVF from the existing kit), through
    `flush_tiny_sample`.
- Determinism: one seed drives the gust `NoiseRng`; strings draw no
  randomness of their own. All state fixed-size; all params clamped and
  sanitized.

Offline example `crates/mamut-dsp/examples/kosava_v0_1_render.rs`:

| Scenario | Script | What it proves |
| --- | --- | --- |
| `gust_shape` | long gust-only run | measured slope vs the `f^(-5/3)` target, ±2 dB band stated |
| `lockin_sweep` | one string, slow wind ramp up then down | ignition at the entry band, extinction at the *exit* band — the hysteresis loop plotted as numbers (ignition/extinction wind values) |
| `chord_ignition` | four strings (`f_v` a chord), one wind ramp | sequential ignition order matches band crossings — the meteorological-arpeggiator kernel |
| `turbulence` | fixed mean wind, `intensity` low vs high | flutter-like intermittency: lock/unlock event counts |
| `determinism` | `chord_ignition` twice | FNV-64 equality |

Metrics: per-string ignition/extinction wind values and times, lock
event counts, `f_w` pull trajectory summary, RMS/peak, FNV-64.

### Acceptance

- Unit tests: hysteresis (entry band ≠ exit band, verified both
  directions); excitation bounded `[0, 1]` under hostile wind; pull
  converges monotonically while locked; gust output clamped; determinism.
- `gust_shape` slope within the stated band over `[0.05, 5]` Hz — or the
  weights are retuned in-slice until it is, with the change recorded.
- `chord_ignition` shows strictly ordered, audibly separated ignitions
  for a spread chord under the default band.

### Evidence

- proposed: `docs/dsp/kosava-v0.1-gust-lockin-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`

### Size

M

## SET5-6: Kosava v0.2 — Engine Ensemble Wind Layer Plus Session Controls

### Rationale

The wind meets the instrument: one global gust field, one `VortexString`
per voice keyed to the sounding note, and a single wind macro that makes
a held chord ignite note by note. This is the set's ensemble payoff and
its most performance-shaped slice.

### Design Sketch

- `KosavaLayerMode { Disabled, Enabled { seed } }` +
  `set_kosava_layer_mode` + `KosavaLayerSnapshot` (wind value, per-voice
  lock states, lock event counter), same idiom as `SET5-2`/`SET5-4`.
- One engine-owned `GustField` ticked at `KOSAVA_TICK_SAMPLES` inside the
  existing block structure; per-voice `VortexString` state in
  `VoiceState`, `f_v` = the voice fundamental (v1; partial-2/3 targeting
  is future work), reset on trigger.
- **Note-gated by construction**: a voice's string excitation is forced
  to decay (release ≈ 20 ms) whenever the voice is not sounding — Kosava
  never sounds without a played note; the layer posture survives.
- Insertion: per-voice pre-filter sum scaled by `kosava_mix` (same
  decision and rationale as Orbita; one consistent insertion story for
  the set).
- Controls (capped): `mix` `[0,1]`, `wind` `[0,1]` (mean wind),
  `turbulence` `[0,1]`, `damping` `[0,1]` (scales excitation attack down
  and release up — the structural-damping knob), `band` `[0,1]` (maps
  the wind band center over ± one octave). Five, at the cap. On-enable
  defaults (also the `reset_controllers` restore point): `mix = 0.35`,
  `wind = 0.4`, `turbulence = 0.3`, `damping = 0.5`, `band = 0.5` —
  tuned in-slice under the binding constraint that `kosava on` plus a
  held mid-range chord ignites at least one voice within a few seconds
  in `dry-run`, with no `set` required.
- Runtime plumbing as before: `--kosava [<seed>]` (optional-seed
  heuristic copied verbatim from the landed `--mozaik [<seed>]` — see
  the `SET5-2` note), headless `kosava` /
  `kosava on|off` / `kosava set <param> <0..1>`, `EngineCommand`
  variants, `status` snapshot line, plus the `kosava_control` profile
  binding kind mirroring the `SET5-8` shape. `reset_controllers` resets the
  controls to defaults; the gust field itself is *not* reseeded by
  reset (weather does not restart when the player resets controllers —
  reseed only on mode re-enable; state this in tests).
- New engine example
  `crates/mamut-engine/examples/kosava_engine_layer_ab_render.rs`:
  factory patch, held four-note chord script, wind ramp scenario —
  A/B `Disabled`/`Enabled`, printing ignition order/times, signatures,
  and metrics. This example *is* the meteorological-arpeggiator demo.

### Acceptance

- Disabled bit-identity against the pre-slice baseline (same bar as
  `SET5-2`/`SET5-4`: signatures asserted in the example, not just
  recorded).
- The chord scenario ignites notes in band order under a single wind
  ramp, audibly and in the printed ignition table; ramping back down
  extinguishes them at measurably different (hysteresis) wind values.
- Note-gating verified: zero layer output with no sounding voices at
  full wind (rendered silence check plus the forced-decay unit test).
- Headless round-trip in `dry-run`; hostile values clamp; suite green;
  per-block cost measured (gust tick + 6 strings + 6 SVFs) via the
  `audio_baseline.rs` pattern.

### Evidence

- proposed: `docs/dsp/kosava-v0.2-wind-layer-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`
- `sel4-rust-execution-optimizer` (voice render loop + per-block tick)

### Size

M–L

## SET5-7: Consolidation — Cost Table, Docs Truth, Honesty Ledger

### Rationale

Three concepts landed as six slices leave three questions open that no
single slice owns: what does the set cost together, do the docs tell one
story, and does the novelty ledger hold up against what was actually
built. Same closing role `SET4-8` plays for Set 4.

### Deliverables

- **Cross-concept cost table**: per-block cost on the dev host for every
  combination that matters (each layer alone, all three enabled, worst
  case: 6 voices, full Roche, full wind) at 48 kHz and 96 kHz, measured
  with the `audio_baseline.rs` harness pattern, with the RPi3B headless
  target explicitly flagged as untested-here (that budget conversation
  belongs to the `SET2` cost track; this table feeds it, honestly).
  The block-timing harness is currently copy-pasted per A/B example
  (`SET5-4` did this); by the time three copies exist, this item either
  extracts a shared `audio_baseline`-style helper or explicitly blesses
  the copy-paste in the cost table's method note — decided here, once,
  not per slice.
- **Docs truth pass**: `README.md` gains the three session-control
  surfaces (flags + headless commands); `CLAUDE.md` crate/architecture
  notes mention the new `mamut-dsp` modules and engine layers;
  `docs/README.md` indexes all six evidence docs; `AGENTS.md` checked
  (likely no change — no new crates, no new build commands). The
  `sound_lab` extension intent table (GFM/BCS enable hints in saved
  Sound Lab patches) carries no Orbita/Mozaik/Kosava intent — this pass
  decides whether to add the three hints or to record explicitly that
  session-only layers stay out of the intent table (the Mozaik v0.2
  evidence doc left this open by name).
- **Honesty ledger**, final form, per concept (the believed-new claims
  re-checked against what shipped):

  | Concept | Exists elsewhere | Believed new here |
  | --- | --- | --- |
  | Orbita | adaptive/Hermode-style tuning (note-level, no dynamics); mode-locking & PLL synthesis; Sethares consonance work | capture *dynamics* with dissipation and entry/exit hysteresis as the playable material; Kirkwood clearing as a spectral sieve; staggered Roche breakup as a note-death mechanic |
  | Mozaik | golden-ratio FM/additive experiments; low-discrepancy sequences in dithering; aperiodic-tiling art | a Bresenham cut-and-project *oscillator* with slope as a crystal↔quasicrystal morph and the phason as a first-class, constant-pitch modulation axis |
  | Kosava | aeolian-harp physical models (academic); flue-pipe models; self-oscillating filters; BCS's own saturation shape (internal relative) | engineering gust spectra (measured `f^(-5/3)`) as a seeded deterministic modulation source; vortex lock-in with hysteresis as a per-note ensemble mechanic; the one-macro meteorological arpeggiator |

- **Optional stretch (cut without ceremony)**: two `mamut-seq` scenarios
  (a Kosava chord-ignition script, an Orbita long-note capture script)
  so the demos are one command; read-only `INSPECT` lines for the three
  snapshots.

### Acceptance

- A new contributor can go from clone to hearing each concept
  (`dry-run`-rendered or `play`) using `README.md` alone.
- The cost table exists with real measurements and honest caveats.
- `sel4-integrated-systems-reviewer` docs-truth pass over the whole set.

### Evidence

- rolled into the six per-slice docs plus this item's docs changes; no
  seventh evidence doc unless the cost table warrants its own file
  (decided in-slice).

### Review Gates

- `sel4-integrated-systems-reviewer`

### Size

S–M

## SET5-8: Session-Layer Controller Bindings — Play The Layers From MIDI

### Rationale

The Set 5 layers are playable only from the headless prompt and CLI
flags; a live performer holds a controller, not a terminal. The
controller profile already maps arbitrary CCs to
`gfm_layer_amount` / `bcs_layer_gain` / `bcs_layer_enabled`
(`crates/mamut-runtime/src/types/profile.rs::ControllerBindingKind`,
`profiles/pc4-full.toml`), so the precedent for "CC drives a session
layer" exists in full. This slice extends it to Mozaik (landed) and
defines the shape Orbita/Kosava copy inside their own v0.2 slices. The
immediate consumer is the `pc4ms-touch-surface-android` `Mamut
Instrument` mode's configurable slide lane (tracked in that repo:
`docs/PC4MS-TOUCH-SURFACE-BACKLOG.md`, items `TS-1..3`), but any CC
source qualifies.

### Design Sketch

- New profile binding kind `mozaik_control` with `target` one of
  `mix | slope | contrast | phason | drift`, parsed into a matching
  `ControllerBindingAction` variant. Routing: the binding resolves to
  the **same** control path the headless `mozaik set <param> <0..1>`
  uses (`RuntimeUiCommand::MozaikSet(MozaikParam, f32)` →
  `EngineCommand` on the bounded control queue) — one truth for
  clamping, smoothing, and detent-snap; no second parameter path.
- CC value maps linearly `0..127 → 0.0..1.0` (the layer mapping owns
  any nonlinearity, e.g. the slope detent snap — already in the
  engine setter, not re-implemented here).
- Mode stays out of reach by design: no binding kind enables/disables
  a layer in v1 (`bcs_layer_enabled` is the precedent that it *can* be
  done; for the seeded layers, enable stays a deliberate CLI/headless
  act so a stray CC cannot re-seed a running texture; state this in
  the profile docs).
- CC on a disabled layer behaves exactly like `mozaik set` on a
  disabled layer today (accepted into the pending control state or
  ignored — whichever the landed `SET5-4` semantics are; do not invent
  a third behavior).
- New example profile `profiles/android-touch.toml`: channel-1
  instrument-mode surface — macros CC16–20, expression CC11, and free
  CCs 21+ bound to `mozaik_control` targets (mix first), with comments
  naming the app-side preset (`Profile CC 21–31`) they pair with.
- Orbita/Kosava: their v0.2 slices add `orbita_control` /
  `kosava_control` by mirroring this kind + action + routing shape —
  noted in `SET5-2`/`SET5-6` by reference, not duplicated here.
- Profiles are session-side files, not patches: **no schema growth**,
  `schema_version` untouched, factory bank untouched.

### Acceptance

- With `--controller-profile profiles/android-touch.toml`, a CC bound
  to `mozaik_control mix` audibly moves the enabled layer's mix on a
  held note, with the same smoothing as headless `mozaik set mix`;
  `--trace-midi` names the binding like existing bound CCs.
- Parsing tests: kind/target matrix, unknown target rejected with a
  useful error; routing test proving CC and headless `set` converge on
  the same `EngineCommand`.
- Disabled-layer behavior matches the landed headless semantics
  (asserted in a test, stated in docs).
- `README.md` controller-profile section and `docs/README.md` updated
  in the same change; the touch-surface backlog reference recorded.
- `cargo fmt --all --check`, `cargo test --locked` green.

### Review Gates

- `sel4-rust-systems-reviewer` (`mamut-runtime` parsing/routing)
- `sel4-integrated-systems-reviewer` (cross-repo contract + docs truth)

### Size

S–M

## Suggested Order And Dependencies

1. The three v0.1 slices (`SET5-1`, `SET5-3`, `SET5-5`) are mutually
   independent and independent of everything in Set 4; any order works.
   If one must go first, `SET5-3` (Mozaik v0.1) is the recommended
   opener: smallest state, zero RNG, guaranteed audible — the cheapest
   proof that the set's evidence discipline holds.
2. Each v0.2 slice needs only its own v0.1
   (`SET5-2` ← `SET5-1`, `SET5-4` ← `SET5-3`, `SET5-6` ← `SET5-5`).
3. The three v0.2 slices are mutually independent but share the
   per-voice pre-filter insertion and the mode/snapshot idiom — land one
   first, let the reviewers bless the shape, then mirror it in the other
   two rather than inventing three shapes.
4. `SET5-8` needs only the landed `SET5-4` (it binds CCs to the landed
   Mozaik control path) and can run in parallel with the Orbita/Kosava
   tracks; its `orbita_control`/`kosava_control` mirrors land inside
   `SET5-2`/`SET5-6`, not here.
5. `SET5-7` closes after all six concept slices (`SET5-8` may land
   before or after it; the docs-truth pass covers whatever has landed).

Interplay with the active Set 4: none required in either direction. The
named extension points (Orbita `kick` from per-note bend/pressure,
Kosava per-note wind boost) become candidate slices only after `SET4-4`
lands, in whichever set is active then.

## Out Of Scope (This Set)

- Patch-schema growth of any kind (`[engine.orbita]` etc. do not exist;
  factory bank and live set untouched)
- GUI work beyond the optional read-only `INSPECT` stretch in `SET5-7`
  (ADR 0004 boundary untouched)
- Per-note expression coupling (extension points named, wired post
  `SET4-4` as future slices)
- New workspace crates, new external dependencies, `voice_count` changes
- GFM lattice coupling (Orbita/Mozaik/Kosava do not read or write GFM
  state; any cross-concept composition is a future concept note, not
  this backlog)
- Frequency-domain Mozaik (a golden-comb additive mode is a possible
  future v0.3; v0.1–0.2 are strictly the time-domain tile train)
- Kosava partial targeting (`f_v` beyond the fundamental) and any
  standalone wind output not gated by a sounding voice

## Risks And Notes

- **Orbita audibility** is the set's biggest musical risk: a 3.5-cent
  capture detent may be too subtle outside slow exposed notes. The v0.1
  scenario contrast (`capture_slow` vs `capture_fast` vs `escape_kick`)
  is the early kill-or-keep gate; if the WAVs don't carry the story,
  stop at v0.1 and rethink before any engine work.
- **Mozaik pitch honesty**: the strongest peak need not sit at `f0`.
  The primitive promises a *stable, stated* relation, not equality; the
  engine slice inherits whatever the evidence measured. If listening
  says the anchor is wrong, fix the `M` calibration constant in-slice
  and re-render — do not paper over it in prose.
- **Kosava slope honesty**: the three-pole gust approximation may miss
  the `f^(-5/3)` band at the default weights; the slice retunes weights
  until measured-in-band or records the best achievable deviation. No
  "Kaimal-shaped" claim without the printed periodogram.
- **Chord-density limits**: lock-in bands (±0.10 oct entry) overlap for
  notes closer than ≈ a whole tone, making ignition order ambiguous in
  cluster chords. Accepted for v1 and stated in the evidence; `band`
  plus `damping` are the player's mitigation.
- **Cost**: worst case adds ~15 sines/voice (Orbita) + 1 tile osc/voice
  (Mozaik) + 1 sine + 1 SVF/voice + a control-rate gust tick (Kosava).
  Trivial on the dev host in expectation, but *measured, not guessed*
  (`SET5-2/4/6` measure; `SET5-7` consolidates). The RPi3B question
  stays with the `SET2` track.
- **Three-layer sprawl**: three new modes, three control surfaces, three
  snapshots is real API surface. The mitigation is the shared idiom
  (mode/snapshot/setter/CLI/headless shapes identical across the three)
  and the five-control hard cap per concept.
- **Q32 rational detents** are approximations (period ≈ 2³² tiles);
  musically indistinguishable from periodic, stated once in the Mozaik
  evidence and never re-litigated.
- **Determinism across sample rates** is *not* promised: tick counts and
  tile lengths are sample-rate-relative. Same seed + same event stream +
  same sample rate ⇒ bit-identical; that is the claim, verbatim, in
  every evidence doc.
