# Radical DSP Additional Directions

Date: 2026-04-30

Status: research capture / backlog. This is not an implementation contract and
does not change the current engine scope.

This note records additional radical DSP directions after reading:

- `deep-research-report.md`
- `DSP_ Edge, Unsolved Problems, Ecosystems.md`
- `State of the Art in Digital Signal Processing_ Bleeding-Edge Advancements, Persistent Challenges, and the Enterprise-Indie Divide.md`

It extends the current radical line:

- `gravitational-phase-lattice-idea.md`
- `bifurcation-coordinate-synthesis-math-foundations.md`
- `bifurcation-coordinate-synthesis-physical-psychoacoustic-foundations.md`
- `bcs-v0.1-hopf-duffing-playground-evidence.md`
- `material-core-idea.md`

## Core Reading

The strongest takeaway is not that `mamut-sint-sw` should become an AI synth.
The stronger claim is that the radical line should become an interpretable
nonlinear instrument family:

```text
phase-space state
+ bifurcation coordinates
+ local fields
+ material memory
+ explicit safety / anti-raspad rules
+ offline evidence
= radical DSP with an engineering path
```

Neural, DDSP, and SSM ideas are useful as controllers, approximators, or future
training surfaces. They should not replace the core instrument with a black-box
raw waveform generator.

The current split remains useful:

- `BCS`: small nonlinear regime playground, easiest to measure.
- `GFM`: larger field instrument, strongest identity claim.
- `MamutMaterialCore`: bridge from current engine to stateful material behavior.

We should continue both `BCS` and `GFM`. The next decision is not which branch
survives, but which branch gets the next deeper analysis and evidence slice.

## Design Principles

### Interpretability First

A radical voice should expose meaningful state:

- energy
- strain
- coherence
- phase / phase velocity
- rupture / fracture
- fatigue / recovery
- regime coordinate
- unsafe event counters

The point is not just to produce surprising audio. The point is to know which
regime produced it, why it stayed bounded, and whether it recovered.

### Nonlinear State Beats VA Decoration

The reports repeatedly point at nonlinear systems, phase spaces, attractors,
bifurcations, delay-free loops, and nonlinear system identification as hard
frontiers. For our radical line, this means the primitive itself should be
nonlinear. A VA oscillator plus colorful macro names is not enough.

Good radical primitives:

- Hopf / Stuart-Landau anchors
- Duffing instability modes
- Van der Pol / relaxation oscillators
- Kuramoto-like phase fields
- material stress / fracture dynamics
- sparse event rupture gates
- continuous-state modal memory

Weak radical primitives:

- standard oscillator into drive
- ordinary filter modulation with dramatic labels
- static wavetable morphing without state memory
- black-box generated audio without controllable internal dynamics

### Offline Evidence Before Engine Integration

Every new radical direction should first have an offline proof:

- deterministic render
- fixed sample rate and duration
- named scenario presets
- finite output
- bounded state
- regime metrics
- hashes for artifacts
- explicit no-engine boundary

The `BCS v0.1` pattern is the right evidence discipline: small model, clear
diagnostics, and no hidden integration changes.

### Neural Components As Assistants

DDSP and neural-physical hybrid work are relevant, but mainly because they show
how to keep DSP structure while learning difficult mappings.

Useful future roles:

- learn gesture-to-parameter curves
- estimate parameters from target audio
- approximate expensive nonlinear terms under strict bounds
- tune regime transitions from listening examples
- classify rendered scenarios for regression tests

Rejected for now:

- black-box neural raw audio as the primary instrument
- training pipeline as a prerequisite for the radical line
- opaque latent controls with no physical or musical interpretation

### Anti-Alias And Anti-Raspad Are Both Core

Radical nonlinear DSP has two separate failure modes:

- spectral failure: aliasing, harsh foldback, uncontrolled high harmonics
- numeric/system failure: NaN, Inf, unbounded state, stuck unsafe regimes

Oversampling is acceptable for offline playgrounds. For engine-grade work, we
should plan for one or more of:

- antiderivative antialiasing
- lookup-table antiderivative approximations
- bandlimited nonlinear readouts
- oversampled subgraphs only where needed
- bounded nonlinear functions with measured harmonic behavior

The anti-raspad side remains:

- state ceiling
- finite checks
- local thresholds
- degraded health states
- recovery windows
- bounded neighbor influence

## Direction 1: BCS v0.2 Multi-Regime Voice

This extends `BCS v0.1` from one Hopf anchor plus one Duffing mode into a small
multi-regime nonlinear voice.

Candidate model:

```text
Hopf pitch anchor
+ Duffing edge / subharmonic mode
+ Van der Pol or relaxation mode
+ fold / saddle-node coordinate
+ bounded readout mixer
```

The goal is to make bifurcation coordinates playable:

- `anchor`: how strongly the voice returns to the pitch center
- `edge`: distance to instability
- `pressure`: drive into nonlinear mode
- `fold`: sudden onset / disappearance coordinate
- `recovery`: return rate after unsafe or high-energy section
- `mix`: perceptual balance between stable anchor and unstable mode

Expected scenarios:

- `stable_anchor`: pitch remains near C3 or selected note.
- `fold_onset`: sound appears abruptly after crossing a control threshold.
- `period_doubling`: stable anchor yields to half-rate or lower apparent period.
- `relaxation_pressure`: smoother oscillator turns into pulse / reed-like motion.
- `edge_recovery`: high edge state returns to stable pitch after pressure release.

Diagnostics should extend `BcsDiagnostics` with:

- regime occupancy counts
- estimated dominant period per analysis window
- fold event count
- recovery time in frames
- alias-risk proxy, such as high-frequency zero-crossing density
- main vs shadow divergence windows, not only one averaged Lyapunov proxy

Why this matters:

`BCS v0.2` is the cleanest laboratory for nonlinear regime design. It can tell
us whether bifurcation coordinates are musically meaningful before we embed
them in a larger field.

Risks:

- Too many nonlinear modes can become a tuning swamp.
- Period metrics may become ambiguous when the readout is rich.
- It may sound like an experiment rather than an instrument unless scenarios
  are carefully designed.

Recommended next analysis:

Write a precise `BCS v0.2` design note before code. Pick only one additional
mode beyond Duffing for the first v0.2 slice.

## Direction 2: GFM v0.2 Event-Rupture Field

This extends `GFM` from a continuous phase lattice into a field that also emits
sparse rupture and recovery events.

The reading on event-based sensing and neuromorphic systems is useful here, not
because we need spiking neural networks, but because the audio analogy is strong:

```text
continuous field state
+ sparse threshold events
+ local event propagation
+ recovery / refractory behavior
= organic motion without all cells changing all the time
```

Candidate model additions:

- each cell keeps `event_charge`
- rupture fires only when local state and neighbor quorum agree
- fired events inject short energy into nearby cells
- cells enter a refractory / recovery interval after firing
- output has both phase-probe and event-probe components

Possible event types:

- `Rupture`: strain exceeds threshold and quorum opens.
- `Anneal`: heat/coherence returns a cell toward stability.
- `Slip`: phase gradient suddenly relaxes.
- `Lock`: local group synchronizes for a short window.
- `Scatter`: coherent neighborhood breaks into noisy local motion.

Readout options:

- center probe reads continuous phase as today
- event layer adds short impulses, filtered by local material state
- stereo or spatial versions can read event centroid and phase field separately
- event density can modulate brightness or roughness

Diagnostics:

- events per second
- event type histogram
- max simultaneous event count
- refractory violation count
- recovery count
- field coherence before and after event clusters
- boundedness and finite checks as in current GFM

Why this matters:

`GFM` already has rupture gates and health states. Turning rupture into a first
class event layer could make it more instrument-like: the performer hears the
field crack, slip, heal, and lock, not just smear into continuous modulation.

Risks:

- Event layer can become noisy click synthesis if not filtered.
- Too many event types will obscure the model.
- Realtime implementation cost can rise if event routing is not bounded.

Recommended next analysis:

Design one event path only: `Rupture -> refractory -> recovery`, with a single
bounded event readout mixed into existing GFM output.

## Direction 3: Radical Material Core

The current `MamutMaterialCore` idea is a practical post-voice stage. The
radical variant makes material behavior a primary memory operator, not just a
saturation or tone stage.

Candidate state:

- `stress`: short-term deformation from input energy
- `fatigue`: accumulated history that lowers future thresholds
- `temperature`: slow energy / brightness memory
- `fracture`: nonlinear discontinuity amount
- `grain`: micro-instability / local roughness
- `healing`: return force after excitation drops

Signal path:

```text
voice sum
-> material memory operator
-> nonlinear modal response
-> bounded readout
```

The material operator should not be a generic distortion. It should have
history:

- repeated notes change the response
- silence heals but not instantly
- high pressure leaves temporary fatigue
- low pressure can anneal the material
- macro identity changes material constants, not only output EQ

Possible modes:

- `Glass`: high coherence, sharp fracture, slow healing.
- `Ash`: low coherence, soft noisy rupture, fast damping.
- `Steel`: high memory, strong resonance, delayed fracture.
- `Membrane`: nonlinear tension, pitch shift under pressure.
- `Ember`: heat-driven brightness and delayed collapse.

Diagnostics:

- max stress
- fatigue integral
- fracture count
- recovery half-life
- input/output energy ratio
- alias-risk proxy after nonlinear stage

Why this matters:

This is the strongest bridge from the existing synth to radical behavior. It can
be inserted after current voices later, while still being designed with the
same nonlinear-state discipline as `BCS` and `GFM`.

Risks:

- If it is too polite, it becomes a normal character effect.
- If it is too broad, it duplicates `GFM`.
- It needs a clear reason why it is not just saturation plus compression.

Recommended next analysis:

Define one material with one gesture evidence render: repeated strikes that
prove fatigue and recovery.

## Direction 4: Continuous-State Modal Organism

This is inspired by SSM audio work, but without adopting a neural raw waveform
generator.

The useful idea is continuous-time compact state:

```text
small latent state
-> continuous-time update
-> modal bank parameters
-> bounded audio readout
```

Instead of training an SSM, we hand-design a compact state model that controls
a modal or resonator bank. The state becomes the organism, and the resonators
are its body.

Candidate state vector:

- `breath`
- `tension`
- `memory`
- `instability`
- `focus`
- `surface`
- `fatigue`

Candidate readout:

- modal bank with time-varying frequencies and damping
- noise/grain source controlled by instability
- nonlinear cross-coupling between modes
- soft-bounded final output

Why this matters:

This direction may produce rich organic audio with less cost than full GFM. It
also gives us a path toward sample-rate-flexible continuous dynamics, which the
SSM reports identify as important.

Diagnostics:

- modal energy distribution
- state norm
- state recovery time
- max modal gain
- finite/bounded flags
- deterministic render hash

Risks:

- It can collapse into a familiar modal synth unless the state dynamics are
  genuinely nonlinear.
- If the modal bank dominates, the latent state may become decorative.

Recommended next analysis:

Prototype as offline only after either `BCS v0.2` or `GFM v0.2`, not before.

## Direction 5: Differentiable-Ready DSP Contract

This is not a synthesis algorithm. It is a rule for how we design radical DSP so
future learning/control work is possible.

Every radical voice should expose:

- typed parameter vector
- typed state vector
- deterministic reset
- fixed-step render path
- scenario gestures
- diagnostic metrics
- artifact hashes
- no hidden random source unless seeded

This makes later DDSP-style work possible without committing to ML today.

Possible future use:

- fit `BCS` parameters to target rendered audio
- learn macro-to-gesture curves
- discover modulation paths that maximize regime contrast while staying bounded
- train a small controller to choose safe edge trajectories
- build regression classifiers for "stable", "edge", "subharmonic", "recovery"

Important boundary:

The trained component should control or approximate the instrument. It should
not erase the instrument's interpretable state.

Why this matters:

It keeps our radical line compatible with DDSP and neural-physical hybrid
research while preserving the thing that makes the instrument ours: explicit
state and regime semantics.

## Direction 6: Anti-Alias Nonlinear Readout Lab

The reports make ADAA and nonlinear aliasing too important to ignore. If `BCS`,
`GFM`, or material operators become sharper, oversampling alone may become too
expensive for realtime use.

This direction is a small utility research branch:

```text
known nonlinear functions
-> oversampled baseline
-> ADAA or LUT-antiderivative readout
-> spectrum / alias proxy comparison
```

Candidate nonlinearities:

- tanh / soft clip
- cubic soft fold
- wavefolder
- discontinuous rupture impulse shaped into a bounded pulse
- material fracture transfer curve

Evidence should include:

- offline WAVs
- spectrum summaries
- CPU rough cost
- alias proxy above a frequency threshold
- perceptual note after listening

Why this matters:

It is not the most exciting creative direction, but it is what allows the other
directions to become engine-safe later.

Recommended next analysis:

Do this only when a chosen radical prototype starts producing useful but
alias-risky audio.

## Ranking For Next Deep Analysis

Recommended priority:

1. `BCS v0.2 Multi-Regime Voice`
2. `GFM v0.2 Event-Rupture Field`
3. `Radical Material Core`
4. `Anti-Alias Nonlinear Readout Lab`
5. `Continuous-State Modal Organism`
6. `Differentiable-Ready DSP Contract`

Reasoning:

`BCS v0.2` is the fastest way to answer whether bifurcation coordinates are
musically useful. `GFM v0.2` is the strongest identity path, but it has more
moving parts. The material core is the best bridge to the existing engine.
Anti-alias work should follow whichever branch first proves compelling audio.
The modal organism is promising, but it risks being less distinct. The
differentiable-ready contract should be applied continuously rather than
treated as a separate feature.

## Decision Questions

For the next discussion, choose one of these questions:

- Do we want a small, measurable nonlinear proof next? Choose `BCS v0.2`.
- Do we want the boldest new instrument identity next? Choose `GFM v0.2`.
- Do we want the shortest path toward current-engine relevance? Choose
  `Radical Material Core`.
- Do we want to harden the DSP math before adding more wild behavior? Choose
  `Anti-Alias Nonlinear Readout Lab`.

The current recommendation is to keep both `BCS` and `GFM` alive, then do the
next deep analysis on `BCS v0.2` first because it can define clean regime
metrics that later inform `GFM`.
