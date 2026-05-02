# Next Generation DSP Research Tracks

## Status

This is a research index, not an implementation contract.

The current `EPM1` implementation remains described by:

- `primitives-math.md`
- `render-path-math.md`
- `control-identity-math.md`
- `../../../mamit-sint-hw-remote-up-21-4/docs/dsp-subsystem-spec.md`
- `implementation-language-strategy.md`

This file only organizes candidate next-generation DSP directions.

Additional radical direction backlog:

- `radical-dsp-additional-directions.md`

## Tracks

| Track | File | Role |
| --- | --- | --- |
| `MamutMaterialCore` | `material-core-idea.md` | Near-term material-memory stage after voice summing |
| `GFM` | `gravitational-phase-lattice-idea.md` | Radical alternate voice engine based on a phase-coupled lattice |
| `BCS` | `bcs-v1.2-playable-midi-layer-evidence.md` | Playable PC4-controlled Hopf/Duffing layer behind explicit scenario mode |

## Track 1: MamutMaterialCore

`MamutMaterialCore` keeps the current subtractive voice engine and adds a
stateful nonlinear modal material stage after voice summing.

It is the lower-risk path:

- preserves the current oscillator/filter voice graph
- adds performance memory through energy, strain, fracture, and recovery
- can be disabled behind a patch or engine flag
- should be cheap enough to prototype directly in `mamut-engine`

Use this path when the goal is to add a physical material signature without
rebuilding how notes become audio.

## Track 2: Gravitacijska Fazna Mreza

`GFM` changes what a voice is. A voice becomes a 2D toroidal lattice of
phase-coupled oscillators, with probes reading the lattice as audio.

It is the radical path:

- replaces the VA-style voice chain with a field model
- turns `gravitacija`, `Pec`, `Baklja`, `Horizont`, and `Swarm` into field
  parameters instead of modulation labels
- makes memory intrinsic to the voice through residual phase distributions
- uses `ekk-runtime`-inspired anti-raspad discipline: `K=7` local influence,
  decaying fields, cell health, and threshold gates
- likely needs a playground before direct engine integration

Use this path when the goal is a genuinely new algorithmic instrument, not an
extension of the current synth.

Implementation language posture:

- Rust first for reference model, tests, offline rendering, and production
  integration
- C only for profile-proven hot kernels behind a narrow ABI
- no Python, Julia, C++, Zig, or Faust as required base workflow

## Track 3: Bifurcation-Coordinate Synthesis

`BCS` treats nonlinear regime boundaries as explicit synthesis coordinates.
The first implementation is deliberately narrower than GFM: one
Hopf/Stuart-Landau pitch anchor drives one Duffing instability mode in an
offline `mamut-field` playground.

Current evidence:

- `bifurcation-coordinate-synthesis-math-foundations.md`
- `bifurcation-coordinate-synthesis-physical-psychoacoustic-foundations.md`
- `bcs-v0.1-hopf-duffing-playground-evidence.md`
- `bcs-v1.0-engine-layer-smoke-evidence.md`
- `bcs-v1.1-standalone-runtime-flag-evidence.md`
- `bcs-v1.2-playable-midi-layer-evidence.md`

This is now an active research branch with an engine hook, standalone listening
path, GUI scenario panel, and PC4 playable controls. The accepted
implementation remains opt-in and has no patch schema or factory patch
contract.

## Relationship

The tracks should not be merged too early.

`MamutMaterialCore` is a practical stage:

```text
current voices -> material memory -> final stage -> effects
```

`GFM` is an alternate voice source:

```text
phase lattice voice -> final stage/effects
```

`BCS` is a smaller nonlinear voice playground:

```text
Hopf pitch anchor + Duffing instability -> bounded layer readout
```

Current playable engine shape:

```text
current voices/GFM layer -> note/activity/SW9/S9-gated BCS layer -> final crossfeed/output
```

They can share identity inputs, test discipline, and realtime constraints, but
they should stay separate until each has accepted audio evidence and a clear
runtime boundary.

## First Decision Gate

Before any track becomes a real engine feature, capture:

- a short WAV or rendered buffer demonstrating the core behavior
- deterministic output for the same event stream and initial state
- no allocation in the audio hot path
- finite output under macro sweeps
- a written reason why the result cannot be achieved by the current VA engine

## Recommendation

Prototype `MamutMaterialCore` first if the goal is fast integration.

Prototype `GFM` first if the goal is research risk and a stronger claim of
algorithmic novelty.

Keep `BCS` behind the explicit scenario mode and PC4 gate until it has an
accepted listening verdict and a reason to become a first-class patchable voice
instead of research evidence.
