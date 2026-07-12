# EPM1 Backlog Set 2: GFM Rupture Control And Cost

Date: 2026-07-06

Status: proposed backlog. No implementation from this document has landed.
Each item ships as its own slice with its own evidence document and review
gates. Evidence file names are proposals; final numbering is assigned at
landing time.

## Summary

This set has two coupled tracks on the GFM lattice
(`crates/mamut-field`, engine layer in `crates/mamut-engine`):

- **Cost and rate truth**: the lattice is the most expensive per-sample block
  in the instrument and there is no measurement of it yet. The live default
  is 96 kHz, cost scales linearly with sample rate, and the RPi3B headless
  target (`EPM1_RPI3B_MIOXM_HEADLESS.md`) almost certainly cannot pay
  full-rate cost. Measure first, then take the known semantics-preserving
  wins, then decide idle and rate policy as explicit contract decisions.
- **Rupture/recovery as a first-class performance control**: today rupture is
  reachable only through the `baklja` thresholded pressure path and recovery
  is fully automatic. Promoting them to explicit performable gestures answers
  open question 3 of `docs/dsp/gfm-related-concepts.md`.

Items:

1. `SET2-1` — GFM cost baseline bench
2. `SET2-2` — semantics-preserving cost pass
3. `SET2-3` — armed-idle policy decision
4. `SET2-4` — audio-rate versus control-rate decision
5. `SET2-5` — rupture/recovery performance gesture
6. `SET2-6` — seven-program surface: promote or archive

## Shared Boundary Constraints

- all work stays inside `mamut-field`, `mamut-engine`, and (for controls)
  the existing engine command surface; no transport-level changes (written
  under the transport freeze, since rescinded by ADR 0005; the constraint
  stands on its own for this set's scope)
- `mamut-field` stays external-dependency-free; benches use `std::time`
  in example binaries, not a bench framework dependency
- realtime paths stay allocation-free; workspace lint posture preserved
- the production contract document is
  `docs/dsp/gfm-performance-control-contract.md`; every contract-shaped
  decision in this set lands as an update there, not as silent behavior

## SET2-1: GFM Cost Baseline Bench

### Rationale

No number exists for lattice cost per sample. Every later decision in this
set (optimization, idle policy, rate policy, RPi3B posture) needs a baseline.

### Design Sketch

- a measurement example binary (for example
  `crates/mamut-engine/examples/gfm_cost_bench.rs`, or a `mamut-field`
  example) using `std::time::Instant`; no new dependencies
- measures ns/sample of the lattice step for: each program preset
  (`slice_a`, `horizont`, `pec`, `baklja`, `stress`), with and without live
  excitation, and the armed-idle case (gate closed, amount `0.0`)
- run in `--release`, document host, CPU governor, and exact command
- reference host for recorded numbers is the lab bench machine
  (i7-4600U / 8 GiB); add RPi3B numbers when that rig is available

### Acceptance

- reproducible numbers (report a 3-run spread)
- evidence doc records the table plus derived headroom at 44.1/48/96/192 kHz
  (samples per second times measured ns/sample versus one core)

### Evidence

- proposed: `docs/dsp/gfm-cost-v0.1-baseline-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer` (example code only; no hot-path change)

### Size

S

## SET2-2: Semantics-Preserving Cost Pass

### Rationale

Verified in current code, `crates/mamut-field/src/`:

- `excitation_footprint(x, y, ...)` is a pure function of `(x, y, posture)`
  with a fixed center, but is recomputed — including a `sqrt` — per neighbor
  per cell per sample (~2,048 evaluations per sample on the 16x16 / K=7
  lattice: 7 neighbor lookups plus 1 local per cell)
- `center_excitation` factors into a pure `(x, y, posture)` term times a
  params-linear term
- both are cacheable per cell through the existing
  `topology_key`/`topology_dirty` refresh mechanism, bit-exactly (cache the
  exact `f32` the function returns today)

### Hard Constraint

Bit-identical PCM against the pre-change baseline. The slice adds an A/B
determinism check (render a fixed scenario before and after the change and
compare bit patterns) in addition to the existing determinism tests.

### Deliverables

- per-cell cached footprint/center factors refreshed on topology dirt
- any additional loop-invariant hoists that survive the bit-exactness rule
- before/after bench numbers against SET2-1

### Acceptance

- measured speedup reported honestly (no target promised in this backlog)
- bit-identical renders across all program presets and an excited scenario
- `cargo test --locked`, clippy, fmt green

### Evidence

- proposed: `docs/dsp/gfm-cost-v0.2-footprint-cache-evidence.md`

### Review Gates

- `sel4-rust-execution-optimizer` (mandatory, hot path)
- `sel4-rust-systems-reviewer`

### Size

M

## SET2-3: Armed-Idle Policy Decision

### Rationale

`Engine::apply_gfm_layer` (`crates/mamut-engine/src/engine/layers.rs`) steps
the lattice voice *before* checking the effective amount, so an armed GFM
layer pays full lattice cost while fully inaudible — and, deliberately or
not, the field keeps evolving silently while the gate is closed.

This is a musical contract question disguised as an optimization:

- (a) status quo — the field lives while closed; opening the gate reveals an
  evolved state
- (b) freeze while closed — zero idle cost; opening resumes a frozen state
- (c) decimated idle stepping — reduced idle cost, approximate evolution

### Deliverables

- listening A/B for (a) versus (b) (and (c) if (b) is musically wrong):
  does "the field was alive while closed" audibly matter at gate-open?
- decision recorded in `gfm-performance-control-contract.md`
- implementation of the chosen policy plus idle-cost measurement against
  SET2-1

### Acceptance

- an explicit recorded decision with a listening note — no silent behavior
  change
- measured idle cost matches the chosen policy

### Evidence

- folded into the contract update plus a short evidence note; proposed:
  `docs/dsp/gfm-cost-v0.3-armed-idle-policy-evidence.md`

### Review Gates

- `sel4-rust-execution-optimizer`, `sel4-rust-systems-reviewer`,
  `sel4-integrated-systems-reviewer` (contract)

### Size

S–M

## SET2-4: Audio-Rate Versus Control-Rate Decision

### Rationale

Open question 5 of `gfm-related-concepts.md`, now load-bearing: the live
default is 96 kHz and lattice cost scales linearly with rate. The RPi3B
headless target needs an explicit answer: full rate, decimated lattice, or
GFM off.

### Design Sketch

- offline prototype first: step the lattice every N frames, hold or
  linearly interpolate the probe output between steps (existing smoothers
  absorb steps), N in {2, 4, 8}
- A/B listening against full rate at 48 kHz and 96 kHz across the three
  production programs, with rupture-heavy `baklja` takes as the stress case
- decide per-host-class policy: laptop default stays full rate; RPi3B gets
  decimation N or GFM off

### Acceptance

- A/B renders plus listening verdict recorded
- decision lands in `gfm-performance-control-contract.md` and is reflected
  in `EPM1_RPI3B_MIOXM_HEADLESS.md` posture
- if adopted, the runtime flag defaults to current behavior on laptop-class
  hosts

### Evidence

- proposed: `docs/dsp/gfm-cost-v0.4-rate-decision-evidence.md`

### Review Gates

- `sel4-rust-execution-optimizer`, `sel4-rust-systems-reviewer`,
  `sel4-integrated-systems-reviewer` (contract)

### Size

M

## SET2-5: Rupture/Recovery Performance Gesture

### Rationale

Rupture is the most distinctive audible event the lattice produces (quorum
voting plus inhibition cooldown), but the performer can only reach it
indirectly through `baklja` pressure. Recovery — the health state machine
walking cells back through `Recovering` — is completely automatic and
invisible. Making both performable answers open question 3 of
`gfm-related-concepts.md` and is the natural continuation of the v0.2 live
response calibration.

### Design Sketch (non-binding)

- **strike/rupture gesture**: a momentary control event that injects a
  bounded `rupture_bias` burst with a spatial footprint; reuses the SET1-1
  strike machinery when available, otherwise uses the center footprint
- **recovery visibility**: expose the health histogram and a recovery
  progress signal through the engine snapshot (pairs with the SET1-3
  `INSPECT` view; a perform-rail indicator is a candidate)
- contract decision inside the slice: is the gesture `baklja`-only (keeps
  `horizont`/`pec` rupture-free guarantees intact) or posture-independent
  with per-program response scaling? Default position: `baklja`-only first.
- control mapping: development and testing run through `mamut-seq`
  (Backlog Set 3) on any named control; the physical PC4 mapping is a
  separate decision because `EPM1_PC4_LIVE_PROFILE.md` is locked
  (candidates: `SW8` or an `S` slider) and is deferred until the gesture
  proves musical

### Acceptance

- scripted A/B: the gesture produces bounded, scaled rupture counts clearly
  distinct from the no-gesture baseline (report rupture counts and final
  health histograms per take)
- `horizont`/`pec` guarantees from the v0.2 live response contract hold
  unless the in-slice contract decision explicitly says otherwise
- recovery progress is visible in diagnostics/snapshot
- finite output, master safety ceiling respected, no hot-path allocation

### Evidence

- proposed: `docs/dsp/gfm-v2.4-rupture-gesture-evidence.md`

### Review Gates

- `sel4-rust-systems-reviewer`, `sel4-rust-execution-optimizer`,
  `sel4-integrated-systems-reviewer` (contract)

### Size

M

## SET2-6: Seven-Program Surface — Promote Or Archive

### Rationale

The v2.0 experiment (`gravity`, `swarm`, `rupture`, `recovery` programs on
top of the three production programs) was reverted; current code ships three
performance programs plus `auto`, and
`docs/dsp/gfm-v2.0-seven-program-evidence.md` carries an experimental status
note. The experiment should not sit in limbo indefinitely.

### Deliverables

Time-boxed decision task:

- re-render the candidate programs from the history-referenced parameters
  under today's 8-control production surface
- criterion: each candidate must earn an audibly distinct identity that the
  existing three programs plus controls cannot already produce
- outcome A: promote a surviving subset into the production contract
  (follow-up sized M)
- outcome B: a short archival note declaring three-plus-`auto` final and
  marking the v1.9/v2.0 docs historical

### Acceptance

- a recorded decision either way; no silent limbo

### Evidence

- decision note appended to `gfm-performance-control-contract.md` or a
  standalone archival note in `docs/dsp/`

### Review Gates

- `sel4-integrated-systems-reviewer` (contract/docs); implementation gates
  only if promotion happens

### Size

S (decision) + M (only if promotion)

## Suggested Order And Dependencies

1. `SET2-1` (measure first)
2. `SET2-2` (take the free wins)
3. `SET2-3` and `SET2-4` (policy decisions, can run in parallel)
4. `SET2-5` (spend the earned headroom on the new gesture; prefers SET1-1
   strike machinery)
5. `SET2-6` (last; depends on how much headroom and control surface exist)

Cross-set: acceptance passes throughout this set get materially cheaper once
Backlog Set 3 (`mamut-seq`) can script pressure/CC gestures and load
patterns without the physical rig.

## Out Of Scope

- transport, queue, or callback boundary changes
- SIMD/parallelization of the lattice (revisit only after SET2-2 and SET2-4
  land and only with bench evidence; any such work would still respect
  `unsafe_code = "forbid"`)
- patch schema `v2`
- PC4 locked-profile changes (mapping decision explicitly deferred)
