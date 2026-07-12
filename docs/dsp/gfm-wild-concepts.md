# GFM Wild Concepts: Lavina, Brazda, Mraz

Date: 2026-07-07

Status: speculative concept note, not an implementation contract. Nothing in
this document is scheduled work, and no schema, control, or runtime behavior
described below exists yet. This is the deliberately imaginative sequel to
`gfm-related-concepts.md`: three synthesis concepts that, to the best of our
knowledge, do not exist as playable instruments anywhere, chosen so that each
one is programmable inside the existing `mamut-field` discipline.

Naming follows the identity family convention (`Horizont`, `Pec`, `Baklja`,
`Gravitacija`): *lavina* = avalanche, *brazda* = furrow, *mraz* = frost.

## Reading Contract

"The only limit is what we can program" is the brief, and this note makes that
limit precise. Every concept below must be able to honor the standing GFM
rules before it deserves a first render:

- **Deterministic.** Same seed plus same event stream produces the same
  output, bit for bit.
- **Bounded.** Fixed state size, clamped fields, fixed iteration budgets, no
  hidden iterative solvers on the audio path.
- **Allocation-free in steady state.** The ADR 0001 realtime rules are
  untouched by anything here.
- **Opt-in and bounded in the mix.** Like the current GFM layer: a
  behavior/color layer over the Mamut voice, never a replacement for it.
- **Evidence-first.** Offline `mamut-field` render examples precede any
  engine integration, mirroring the `gfm-v0.1 -> v2.3` ladder. First
  integrations are session-only controls, mirroring the note-strike v1
  precedent (`[engine.gfm]` gains no new patch fields until a contract
  slice earns them).

Each concept ends with a nearest-relatives section, in the spirit of the
"what is and is not novel here" discipline from `masnoca.md`. Wild is the
goal; overclaiming is not.

## The Substrate These Concepts Stand On

What GFM already provides, and what each concept below reuses instead of
reinventing:

- a 16x16 toroidal lattice of cells, each carrying `phase`, `velocity`,
  `detune`, `energy`, `strain`, `heat`, `coherence`, `fracture`, and a
  health state machine (`Healthy / Suspect / Quarantined / Recovering`)
  with damping, quarantine, and recovery laws (`lattice.rs`,
  `topology.rs`);
- a K=7 neighbor topology: four local links plus three seeded long-range
  links — the "gravitational" shortcuts (`build_topology`);
- rupture mechanics with quorum voting, cooldown inhibition, and posture
  gating;
- deterministic seeded RNG and full-field sanitization with per-cell
  ceilings;
- note strikes mapped into field space: pitch class selects the x band,
  octave selects the y band, seed-folded per-cell offset on top
  (`inject_strike`, `note_strike_position`);
- aftertouch pressure/heat excitation with posture-shaped footprints;
- a fixed 5-tap probe cross readout, stereo probe taps at +/-2 columns,
  and a read-only terrain snapshot on the `INSPECT` screen;
- the BCS playground as the sibling proof that deterministic nonlinear
  regimes (Hopf anchor, Duffing instability) are renderable and boundable.

One lived lesson binds all three concepts: **probe audibility**. The probes
read a fixed cross near the lattice center, so field activity far from the
probes is only indirectly audible. Every concept below states its readout
story explicitly instead of assuming the field will be heard.

## The Three Concepts at a Glance

| | Lavina | Brazda | Mraz |
| --- | --- | --- | --- |
| New physics | self-organized criticality: stress cascades | activity-dependent plastic topology | first-order phase transitions with latent heat |
| The playable axis | distance to catastrophe (branching ratio) | wear: plasticity, erosion, fatigue | heat flux plus direction (hysteresis) |
| Native time scale | milliseconds (events) | minutes to sessions (memory) | seconds (material state) |
| The player's verb | load, release | practice, rest | freeze, thaw, quench |
| Reuses | strike map, cell oscillators, rupture vocabulary | topology weights, health machinery, `INSPECT` | heat and coherence fields, stereo probes, strikes |

They are deliberately orthogonal: Lavina is about the *statistics of events*,
Brazda about the *memory of the medium*, Mraz about the *state of the
material*. All three are regime passes over the same cell substrate.

---

## Concept 1: Lavina — Criticality as a Performance Dimension

### The Fantasy

You hold a chord and the field loads like a snow slope. Nothing happens.
Then a click. A scatter of clicks. A shelf lets go and a slide of
micro-strikes rolls diagonally through pitch space and dies out. The main
macro does not control how loud this is — it controls **how close the whole
field sits to the tipping point**. At `0.3` the instrument ticks like a
cooling roof. At `0.9` every note you release risks a landslide.

You do not play the events. You play the distance to the event.

### The Model

Lavina adds a stress lattice in the Olami-Feder-Christensen (earthquake
model) family on top of the existing cells. One new per-cell field:
`stress: f32`.

Loading has three sources:

- a slow uniform tectonic drive per sample (control-scaled);
- note strikes deposit stress through the existing strike footprint at the
  existing pitch-class/octave position;
- optionally, **NoteOff releases stress** (`release_drop`): lifting a key can
  be what starts the slide. The inversion is the point.

The topple rule, non-conservative so the system self-organizes toward the
critical regime instead of having to be tuned there:

```text
if stress[i] >= s_c:
    emit a topple event at cell i (cascade generation k)
    stress[i] -= s_c
    each of the 4 local neighbors j:  stress[j] += alpha      * s_c
    each of the 3 long links j:       stress[j] += alpha_long * s_c
    dissipation: 4*alpha + 3*alpha_long < 1
```

Cells pushed over threshold topple in later budget slots as generation
`k+1`. A cascade of topples is an avalanche. Because strikes land in
pitch-class bands and redistribution follows the lattice topology, an
avalanche is an audible walk through neighboring pitch classes, with the
three long links providing occasional far jumps — gravitational slides.

**The wild part is the governor.** Define the branching ratio `sigma` as the
average number of child topples per topple, measured over a sliding window
(a few thousand samples). `sigma < 1` is subcritical (cascades die),
`sigma ~ 1` is critical (scale-free avalanches, the edge), `sigma > 1` is
supercritical (self-sustaining roar). The `criticality` control sets a
target `sigma*`, and a slow, clamped, deterministic integral loop adjusts
dissipation to hold the measured `sigma` at the target while playing pumps
energy in. The performer commands a **statistical regime**, and the governor
holds the field there. No existing instrument exposes this closed loop as
its primary macro; density knobs on granular engines are open-loop by
comparison.

Realtime bounds, stated as mechanics rather than caveats:

- per-sample topple budget (order of 8); overflow enqueues into a fixed
  aftershock ring (capacity 256, drop-oldest, counted). The drain rate is a
  control. Earthquakes have aftershocks; the budget is not an apology, it is
  Omori's law with a knob.
- the budget and queue truncate the raw OFC statistics. Diagnostics must
  report demanded vs executed topples and queue high-water, so evidence
  tables state the distortion instead of hiding it.

### Readout (Probe-Audibility Answered)

Dual path:

1. **Topple-grain bus.** A fixed pool of 16 grain slots (steal-oldest, no
   allocation): each topple fires a short damped sine at the toppling cell's
   existing seeded base frequency, amplitude from released stress, summed
   into the GFM layer output before the soft limit. Audibility is therefore
   independent of where the avalanche runs relative to the probes.
2. **Field wake.** Each topple also kicks the cell's existing `energy`, so
   large slides leave a glowing wake the probes hear as the field's own
   bloom after the crackle.

### Sketch of the Control Surface

Session-only first, mirroring the note-strike v1 precedent. Schema below is
an illustration, not a contract:

```toml
[engine.gfm.lavina]   # sketch only — no such schema exists
criticality = 0.82    # target branching ratio, mapped to ~0.2..1.05
grain = 0.40          # topple-grain bus gain
aftershock = 0.35     # ring drain rate
locality = 0.60       # local vs long-link redistribution split
release_drop = 0.25   # how much stress a NoteOff releases
```

Identity affinity: `ruin` maps naturally onto loading rate, and
`rupture_response` onto grain edge. Baklja is the obvious posture cousin; a
Gravitacija-flavored variant would weight long-link redistribution up.

### Programmability Check

- Extra state: `stress` f32 x 256 (1 KiB), aftershock ring (1 KiB), 16 grain
  slots (a few hundred bytes). Total well under 4 KiB.
- Cost: the stress load and threshold scan ride the existing per-cell pass
  (one compare and a few adds per cell); topples are budget-bounded;
  `sigma` estimation is counter arithmetic. Cheaper than the existing
  K=7 neighbor sampling pass.
- Determinism: no new RNG draws required; the governor is deterministic and
  clamped.
- Safety: `stress` clamped to `[0, 4*s_c]`; grain amplitudes clamped; the
  existing health machinery is untouched (stress is regime-local state).

### Evidence Plan

Offline `mamut-field` example first (`lavina_regimes_render` or similar),
one run, one binary, deterministic seed, house-style table:

| Scenario | Intent | Key metrics |
| --- | --- | --- |
| `subcritical` | `sigma* = 0.4` ticking field | measured `sigma`, avalanche-size histogram (16 log bins), RMS/peak, PCM signature |
| `edge` | `sigma* = 0.95` scale-free crackle | same, plus largest avalanche and demanded-vs-executed topples |
| `supercritical` | `sigma* = 1.05` sustained roar | same, plus aftershock queue high-water |
| `release_slide` | chord release triggers a slide | topple count attributable to `release_drop` window |

Acceptance: the three regimes produce visibly distinct histograms and
distinct PCM signatures; the governor holds `sigma` within a stated band in
steady state; queue growth is bounded and reported.

### Failure Modes and Open Questions

- The governor can fight player dynamics; the loop must be slow (seconds)
  with a deadband, or the criticality knob feels haunted.
- At low `grain` the regime contrast may be too subtle; the field-wake path
  is the hedge.
- The concept lives or dies on the knob's statistical truth: if evidence
  cannot show three genuinely different event statistics, this is just a
  noise machine with lore.

### Nearest Relatives (Honesty)

Self-organized criticality is established physics, and SOC-flavored
*composition* experiments exist (sandpile-driven MIDI generation). Crackle
and dust generators exist (SuperCollider `Dust`, analog Geiger-style noise
sources) but are Poisson processes: memoryless, cascade-free, with no
critical regime to approach. Earthquake sonification exists as science
outreach, not as an instrument. Believed new here: a seeded, bounded,
deterministic SOC lattice as a playable layer whose primary macro is a
**governed distance-to-criticality**, with cascades embedded in pitch space
and aftershocks as a bounded, controllable mechanic.

---

## Concept 2: Brazda — A Field That Wears In

### The Fantasy

An instrument you break in. Weeks of your voicings leave furrows in the
field the way boots wear a path across grass. Play your home progression
and the field blooms early and warm, like a hall that has learned your set.
Play against the furrows and the field audibly resists — strained, a shade
dull — until you either cut a new furrow or come home. Two players with the
same patch end the month holding different instruments. And the wear
pattern is a file: a relief you can save, share, or wipe. A biography as
data.

The probe is a stylus: **the needle finds the groove you cut.**

### The Model

Today `build_topology` derives static per-seed link weights. Brazda makes
the seven per-cell link weights slow state: `w[i][k]` with the seeded
baseline `w0[i][k]` kept as the rest shape.

Activity is a slewed, non-negative measure of how excited each cell is
relative to rest (reusing `energy`; slew around 50 ms). The plasticity tick
runs at control rate: every 256 samples, one lattice row is updated
round-robin, so a full pass completes in about 85 ms at 48 kHz — bounded,
incremental, and far off the per-sample path.

Per link, the update is Hebbian growth with an Oja-style self-normalizing
term and erosion back toward the seed:

```text
dw = eta   * a[i] * a[j]        # co-active link strengthens: the furrow deepens
   - eta   * a[i]^2 * w         # Oja term: bounded, no runaway self-excitation
   - lambda * (w - w0)          # erosion: the field slowly forgets
w' = clamp(w + dw, 0.25 * w0, 2.50 * w0)
```

Three mechanics sit on top:

- **Fatigue.** Per-link wear counters accumulate flux while a link rides its
  ceiling. Worn links lose elasticity: their effective ceiling drops and
  they inject `strain` into both endpoint cells — which feeds the existing
  rupture, quarantine, and recovery machinery. Over-practiced paths turn
  brittle and can crack; rest heals them through the existing `Recovering`
  path. The instrument does not get bored of your licks; the material
  fatigues, which is better.
- **Consolidation.** A momentary control freezes plasticity and halves
  erosion: varnish. Hold what you have; stop learning.
- **The relief artifact.** Export `quantize_i8((w - w0) / w0)` for all
  256 x 7 links: 1792 bytes plus a small header (seed, version), with an
  FNV-64 signature for evidence tables. Session commands
  (`relief save/load/clear`) only in v1 — the patch schema is untouched,
  exactly like note strikes were session-only first. A relief is portable:
  someone else's month of playing, loaded into your field.

### Readout (Probe-Audibility Answered)

Wear is made audible twice:

1. **Probe drift.** Each plasticity pass computes the wear centroid inside a
   bounded disc (radius ~4) around the lattice center and slews the probe
   cross toward it at no more than one cell per pass. The stereo taps keep
   their existing +/-2 column offset and straddle the furrow. The
   instrument gradually listens where you play. The `INSPECT` probe overlay
   already exists, so the drift is visible, not spooky.
2. **Earlier bloom.** Furrowed links raise phase coupling along learned
   paths, so practiced voicings cohere faster and ring fuller than
   unfamiliar ones — the difference between home and away is audible in the
   attack of the field, not just its steady state.

### Sketch of the Control Surface

```toml
[engine.gfm.brazda]     # sketch only — no such schema exists
plasticity = 0.40       # eta: how fast furrows cut
erosion = 0.20          # lambda: how fast the field forgets
fatigue = 0.35          # wear-to-brittleness sensitivity
# consolidate is a momentary session control (pedal-shaped), not a value
```

### Programmability Check

- Extra state: `w` f32 x 1792 (7 KiB), wear u16 x 1792 (3.5 KiB), activity
  f32 x 256 (1 KiB). Order of 12 KiB, fixed.
- Cost: 112 links touched per tick (one row x K=7) at control rate —
  noise-level next to the existing per-sample neighbor pass.
- Determinism: the rule draws no RNG; same seed plus same event stream
  yields a bit-identical relief. This is testable and must be a test.
- Interplay: posture or `spatial_spread` changes rebuild `w0`; whether live
  weights rebase proportionally or reset is a design decision to make
  explicitly, not by accident.

### Evidence Plan

| Scenario | Intent | Key metrics |
| --- | --- | --- |
| `determinism` | two identical runs | relief FNV-64 equality, byte-identical export |
| `learning_curve` | one 8-bar `mamut-seq` scenario looped 32x, plasticity on vs off | per-loop spectral distance from the loop-1 baseline: expect monotone divergence, then saturation |
| `fatigue_rupture` | 200 repetitions of one chord, fatigue on | rupture/health counts appearing in late loops; recovery run afterwards showing healing |
| `probe_drift` | off-center strike cluster | probe position trace, bounded wander radius respected |

The learning-curve table is the signature evidence: an instrument whose
measured response to the *same phrase* drifts with practice, reproducibly.

### Failure Modes and Open Questions

- **Inaudible subtlety** is the main risk: weight changes might vanish into
  the field's own motion. Probe drift and attack-coherence are the two
  designed amplifiers; if listening tests still can't tell a worn field
  from a fresh one, the concept fails honestly.
- **Habit lock-in**: erosion has a floor precisely so the field always
  forgets; a field that can only deepen is a rut, not an instrument.
- Does a relief transfer musically across patches of the same posture
  family, or is it patch-specific in practice? Unknown until rendered.

### Nearest Relatives (Honesty)

Scanned synthesis (Mathews/Verplank) is the closest single ancestor — a
performer deforms a slow dynamical terrain that is read at audio rate — but
its terrain has no memory beyond its immediate physics and learns nothing.
Adaptive-mapping instruments and IML tools (the Wekinator family) learn
*control mappings* while the sound-producing medium stays fixed. Neural
cellular automata learn offline and freeze at inference. Reservoir
computing keeps its random topology fixed by definition. Tape and vinyl
wear emulations simulate a *medium's* generic decay, not *this player's*
specific history. Played-in acoustic instruments are real folklore but not
data. Believed new here: bounded, deterministic, Hebbian-plastic topology
**inside the sound-producing lattice** of a realtime instrument, with
fatigue/recovery as musical mechanics and the wear pattern as a portable,
hashable artifact.

---

## Concept 3: Mraz — First-Order Phase Transitions You Can Play

### The Fantasy

The field freezes over. Hold a pad and pull the heat down: the shimmer
stiffens, crystals catch around your last strike, a front sweeps across the
stereo image left probe first, and the sound *sets* into glass. Push
aftertouch into it and it holds… holds… — you can feel the material
spending your heat — then gives way all at once, glass melting back to
breath.

Supercooling is the party trick: cool the field gently below freezing
without playing anything. Nothing happens. The field is now a trap, armed.
One quiet note seeds it and the whole lattice crystallizes in a single
audible rush, like a bottle of supercooled water snapping to ice when
tapped.

And how fast you let go matters: release heat slowly and the field anneals
into large, pure, ringing domains; dump it and you quench a strained,
brittle polycrystal that Baklja can shatter. **Release becomes metallurgy.**

Mamut is an ice-age instrument whose identity family so far only knows how
to burn (`Pec`, `Baklja`). Mraz is the missing cold pole.

### The Model

One new per-cell field: `crystal c in [0, 1]` (0 = liquid, 1 = frozen).
Temperature reuses the existing `heat` field and its existing diffusion.
Per-cell thresholds come from the existing seeded detune acting as an
impurity map, which gives deterministic domain structure:

```text
T_melt[i]   = T_m0 + jitter[i] * (1 - purity)
T_freeze[i] = T_melt[i] - undercool          # the hysteresis gap
```

Transitions ride the existing per-cell pass, with rates clamped so a
crystallization front crosses one cell per roughly 2–10 ms — a 16-cell
sweep takes an audible 30–160 ms rather than an instantaneous click:

```text
freezing — allowed only if nucleated (some neighbor has c >= 0.6, or a
strike seeded this cell while T < T_freeze):
    dc = +r_f * (T_freeze - T)     clamped to dc_max
    T  += L * dc                   # latent heat released: the front warms itself
                                   # (recalescence — fronts can audibly stall)
melting:
    dc = -r_m * (T - T_melt)       clamped
    T  -= L * |dc|                 # latent heat absorbed: temperature plateaus
                                   # under aftertouch until the phase gives way
```

Quenched strain: freezing while temperature is falling fast adds
`strain += q * |dT| * dc` — quenched-in stress. The existing rupture and
health machinery then makes quenched fields shatter-prone with no new
mechanics; slow annealing adds almost none.

Sound of the two phases:

- **Frozen** cells phase-lock: frozen-frozen neighbor pairs get their
  coupling scaled by `(1 + k_c * c_i * c_j)`, so `coherence` (an existing
  field) rises and the local spectrum coheres toward bell/glass. The probe
  taps add a `c`-gated crystalline component in the same style as the
  existing posture components.
- **Liquid** cells keep today's behavior: free-running detune, warm,
  breathing.

The hysteresis gap is the playable core: because `T_melt != T_freeze`, the
same heat-drive position sounds different depending on the direction you
came from. The `(T, c)` loop is drawable on `INSPECT` — a hysteresis curve
as a live instrument view.

### Readout (Probe-Audibility Answered)

Twice:

1. The stereo probe taps sit at +/-2 columns, so a traveling front crosses
   the left tap measurably before the right — freezing is *spatialized* by
   the existing readout with zero new plumbing (at ~5 ms per cell, a
   ~20 ms inter-channel envelope lag: subtle motion, clearly measurable).
2. The global crystal fraction `C = mean(c)` scales a readout blend, so a
   field freezing far from the probes still audibly stiffens.

### Sketch of the Control Surface

```toml
[engine.gfm.mraz]     # sketch only — no such schema exists
heat_drive = -0.30    # bipolar: negative cools, positive heats
undercool = 0.55      # hysteresis gap width (supercooling depth)
nucleation = 0.40     # how easily strikes seed crystals
purity = 0.70         # impurity jitter -> domain size (anneal quality)
latent = 0.60         # L: plateau length / how much the material resists
```

Aftertouch keeps its existing meaning (heat pressure) and becomes the thaw
gesture for free.

### Programmability Check

- Extra state: `c` f32 x 256 (1 KiB). Everything else reuses existing
  fields.
- Cost: a few compares, multiplies, and clamps per cell added to the
  existing pass; no solvers, no iteration.
- Determinism: thresholds derive from seeded per-cell state; no new RNG.
- Safety: `dc` and `T` updates clamped; sanitization and ceilings as today;
  health machinery reused untouched.
- One honest tension: `heat` currently carries tonal color semantics
  (e.g. the `Pec` readout). Making it double as thermodynamic temperature
  may fight those semantics; if it does, temperature becomes its own f32
  plane (+1 KiB) and the concept survives unchanged.

### Evidence Plan

| Scenario | Intent | Key metrics |
| --- | --- | --- |
| `freeze_over` | cool with nucleation on: front sweep | left-vs-right probe envelope lag (ms), mean-`c` trajectory, PCM signatures |
| `supercool_shatter` | deep undercool, no notes, then one strike | time from strike to full crystallization, strain/rupture profile of the cascade |
| `thaw_plateau` | aftertouch ramp into a frozen field | spectral centroid trajectory: a flat plateau segment while `c` falls is the latent-heat signature, visible in one plot |
| `quench_vs_anneal` | same phrase, fast vs slow cooling | quenched-in strain, rupture counts under a subsequent Baklja pass, spectral difference |

Acceptance: the plateau is visible in the centroid trace; the front lag
matches the configured front speed; quench and anneal are measurably and
audibly different materials.

### Failure Modes and Open Questions

- A latent-heat plateau can read as a **broken knob** ("aftertouch stopped
  working"). Mitigations: the `INSPECT` `c`-plane makes the spent heat
  visible, and the transition itself should carry a faint crystalline
  crackle (borrowable from Lavina's grain bus) so effort is audible even
  while pitch-space output holds.
- The two phases must differ by more than brightness — the crystalline
  phase-lock has to be audible as *coherence*, or the whole mechanic
  collapses into a filter sweep with extra steps. Listening gate, early.
- Freeze/thaw asymmetry needs tuning so hysteresis feels like material
  memory, not lag.

### Nearest Relatives (Honesty)

Morphing synthesis is the anti-model: continuous crossfades by
construction, no state, no history. Spectral "freeze" effects hold a static
snapshot — no thermodynamics, no fronts, no cost to freezing. Ising-model
sonifications exist as science-art demos, not bounded playable instruments.
Hysteresis appears in circuit emulation (tape, transformers, Schmitt
triggers) as *component* behavior, never as a macroscopic playable state
machine with latent heat. Granular "ice" sound design is authored texture,
not mechanics. Believed new here: a designed first-order phase transition
with latent heat, nucleation, supercooling, recalescence, and quench/anneal
as **performance mechanics** in a realtime instrument.

---

## How the Three Compose

The concepts are orthogonal in axis but share one substrate, so their
interactions come nearly for free:

- **Lavina x Mraz — ice-quakes.** Quench a field, then load it: brittle
  crystal plus stress cascades means the shatter *is* an avalanche in a
  frozen medium. Both mechanics already write `strain` and read the rupture
  machinery; no new coupling code, just physics agreeing with itself.
- **Brazda x Mraz — freezing your furrows.** Crystal does not learn: frozen
  regions have zero plasticity. Freezing is therefore the natural
  consolidation gesture — varnish as an act of weather. Thaw to resume
  breaking the instrument in.
- **Lavina x Brazda — avalanche scars.** Topple flux counts as activity, so
  big slides carve furrows: geology in fast-forward, and the field's event
  history becomes visible relief on `INSPECT`.

If more than one of these is ever built, the honest architectural seam is a
small fixed set of regime hooks in the lattice update loop over the shared
`GfmCell` + topology + health machinery — not a plugin system, not a trait
zoo. Explicitly not a contract; recorded so the first implementation slice
doesn't invent three private lattices.

## What Stays True No Matter What

- The ADR 0001 realtime rules bind any implementation: bounded queues, no
  allocation/logging/blocking on the render or MIDI paths, drop visibility.
- Opt-in layer posture: everything here is a layer over the Mamut voice,
  gated exactly like the current GFM layer.
- Session-only first: no `[engine.gfm]` patch-schema growth until a
  contract slice earns it (the note-strike precedent).
- No transport, GUI-boundary, or schema claims are made here; the review
  gates in `docs/review-gates.md` apply to any code that follows.
- Evidence ladder: offline `mamut-field` example render with a house-style
  evidence doc first, exactly like `gfm-v0.1-baseline.md`, before any
  engine layer work.
- Every regime states its probe/readout story before its first render.

## Honesty Ledger

| Concept | Exists elsewhere | Believed new here |
| --- | --- | --- |
| Lavina | SOC theory; sandpile composition experiments; Poisson crackle units; earthquake sonification | governed distance-to-criticality as the primary live macro; deterministic bounded cascades embedded in pitch space; aftershocks as a mechanic |
| Brazda | scanned synthesis; adaptive mappings / IML; neural CA; media-wear emulations | plasticity inside the resonant medium, deterministic and bounded; fatigue/recovery as musical mechanics; the relief as a portable, hashable artifact |
| Mraz | Ising sonification; spectral freeze; morphing synths; component-level hysteresis emulation | first-order transition with latent heat, nucleation, supercooling, and quench/anneal as playable performance mechanics |

None of this is scheduled. The next honest step for any of the three is a
single offline render example in `mamut-field` with a deterministic seed and
an evidence document — the same first step GFM itself took.
