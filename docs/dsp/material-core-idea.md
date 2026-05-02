# Mamut Material Core Idea

## Status

This is an exploratory DSP idea note, not an implementation contract.

The current `EPM1` DSP engine is still described by:

- `primitives-math.md`
- `render-path-math.md`
- `control-identity-math.md`
- `../../../mamit-sint-hw-remote-up-21-4/docs/dsp-subsystem-spec.md`

This document captures a possible next-generation algorithmic DSP direction:
make `Mamut` behave less like a fixed signal chain and more like a physical
material that remembers how it was played.

Related research index:

- `next-generation-dsp-research-tracks.md`
- `implementation-language-strategy.md`
- `gravitational-phase-lattice-idea.md`

## Working Name

`MamutMaterialCore`

Musical name:

`Tektonska Memorijska Rezonanca`

English shorthand:

`Tectonic Memory Resonance`

## Core Thesis

Do not make the revolutionary part another oscillator or another filter.

Instead, add a stateful nonlinear modal material stage. Audio should not merely
pass through a chain. It should leave stress, fatigue, fracture, and memory in
the instrument, so later notes are rendered through the changed material state.

The result should feel like an instrument made of living mass:

- gentle playing keeps the material open and elastic
- heavy playing bends resonant modes downward
- repeated pressure creates strain and lowers headroom
- rupture creates asymmetric edge, sync-like shocks, and unstable color
- recovery is slow enough that phrasing matters

## Placement

Initial placement should be after voice summing and before the existing
time-domain effects:

```text
voices
-> current per-voice filter/body path
-> MamutMaterialCore
-> existing final body / mid-side character stage
-> chorus
-> reverb
-> output
```

The stage should be block-safe and realtime-safe. It should allocate no memory
inside `process_block`.

## State Model

Use a small fixed array of material cells, for example 8 or 12 cells.

Each cell owns:

```text
mode_state
energy
strain
fracture
memory
base_freq
base_q
base_weight
coupling
```

Interpretation:

- `energy` tracks recent acoustic impact.
- `strain` tracks accumulated stress.
- `fracture` tracks nonlinear damage or rupture.
- `memory` stores a slow tonal residue.
- `mode_state` renders a resonant mode.

## Frame Algorithm Sketch

For each frame:

```text
impact = abs(input) + grav_pull * low_mid_energy

for each cell:
  energy = decay(energy) + impact * coupling
  strain = strain + energy * strain_gain - strain * recovery

  fracture_target = hysteresis(strain, rupture_threshold, healing_threshold)
  fracture = smooth(fracture, fracture_target, fracture_rate)

  freq = base_freq
       * (1 - mass_bend * strain)
       * (1 + fracture * chaos_pitch)

  q = base_q
    * (1 + pec_pressure * pressure_q)
    * (1 - fracture * fracture_q_loss)

  excitation = input + neighbor_feedback + memory * fossil_feedback
  mode = nonlinear_resonator(excitation, freq, q, fracture)

  memory = mix(memory, mode, memory_write_rate)
  material_output += mode * base_weight + memory * fossil_weight
```

Then:

```text
output = mix(input, material_output, material_mix)
output = nonlinear_body_limit(output, headroom, fracture)
```

## Identity Mapping

The existing identity layer is the reason this idea belongs in `Mamut`.

`Gravitacija`:

- increases modal mass
- bends resonant frequencies downward
- reduces headroom
- slows recovery
- makes the material remember impact longer

`Pec`:

- increases pressure and internal heat
- strengthens low-mid body
- increases compression before rupture
- makes recovery thicker and slower

`Baklja`:

- lowers rupture threshold
- increases asymmetric fracture
- adds sync-like transient shock when fracture rises quickly
- increases high-edge instability

`Horizont`:

- opens modal spacing
- increases air and stereo dispersion
- reduces dense cross-coupling
- restores headroom and elasticity

`Swarm`:

- detunes modal centers as a moving group
- increases neighbor coupling
- creates small collective pitch and phase deviations
- can make the material feel granular without adding a grain engine

## What Makes It Different

This is not just a filter bank after the synth.

The important difference is historical state:

```text
same patch + same note != same sound
unless the material state is also the same
```

The instrument should become path-dependent. A phrase played softly before a
heavy chord should not leave the same material condition as the same chord
played first.

This gives the synth a performance memory that is still deterministic and testable.

## Minimal V1 Shape

Start with a conservative implementation:

```rust
struct MaterialCell {
    mode: MaterialMode,
    energy: f32,
    strain: f32,
    fracture: f32,
    memory: f32,
    base_freq: f32,
    base_q: f32,
    base_weight: f32,
    coupling: f32,
}

struct MamutMaterialCore {
    cells: [MaterialCell; 8],
    previous_output: f32,
    fracture_impulse: f32,
}
```

V1 should avoid FFT, convolution, dynamic allocation, and random behavior in the
render path. Fixed modal cells are enough to prove the musical idea.

## Acceptance Bar

The idea is worth keeping only if these claims become audible:

- aftertouch physically changes the next note, not only the current note
- repeated heavy playing makes the instrument darker, heavier, and more unstable
- release into silence still leaves a short recoverable material condition
- `Horizont` can open the material back up
- `Baklja` can push the material into audible rupture without clipping garbage
- the behavior remains deterministic for the same event stream and initial state

## Non-Goals

- Do not claim a new oscillator algorithm.
- Do not replace the existing subtractive engine immediately.
- Do not add a DSP graph library just to prototype it.
- Do not make the stage depend on wall-clock time or nondeterministic randomness.
- Do not let the material core hide clipping or invalid samples.

## First Prototype Cut

The first prototype should live behind an explicit engine flag or patch field.

Recommended first slice:

1. Add a disabled-by-default `MaterialCorePatch`.
2. Add `MamutMaterialCore` as a fixed-size state object in `mamut-engine`.
3. Feed it mono mid signal, then expand back to stereo through existing mid-side
   logic.
4. Drive only four identity inputs first: `grav_pull`, `pec_pressure`,
   `baklja_edge`, and `horizont_air`.
5. Add tests for determinism, finite output, and recovery after silence.

If that slice does not create a recognizable material memory, delete or redesign
the idea before it spreads through the parameter model.
