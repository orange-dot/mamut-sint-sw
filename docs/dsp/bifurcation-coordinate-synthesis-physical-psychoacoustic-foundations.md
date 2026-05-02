# Bifurcation-Coordinate Synthesis: Physical And Psychoacoustic Foundations

Date: 2026-04-30

## Status

This is a research foundation note, not an implementation contract.

The goal is to ground `Bifurcation-Coordinate Synthesis` in physical musical
acoustics and psychoacoustics before inventing a Mamut-specific DSP model. The
mathematical companion note is:

- `bifurcation-coordinate-synthesis-math-foundations.md`

## Reference Discipline

External references used here:

- N. H. Fletcher, "Nonlinear theory of musical wind instruments":
  <https://doi.org/10.1016/0003-682X(90)90040-2>
- Joel Gilbert, Sylvain Maugeais, Christophe Vergez, "Minimal blowing pressure
  allowing periodic oscillations in a simplified reed musical instrument
  model": <https://doi.org/10.1051/aacus/2020026>
- Soizic Terrien, Baptiste Bergeot, Christophe Vergez, Samy Missoum, "Basins of
  attraction in a dynamical system with a time-varying control parameter: The
  case of attack transients in a simple model of reed musical instrument":
  <https://doi.org/10.1016/j.jsv.2025.119241>
- Arnold Myers, Robert W. Pyle, Joel Gilbert, Murray Campbell, John Chick,
  Shona Logie, "Effects of nonlinear sound propagation on the characteristic
  timbres of brass instruments": <https://doi.org/10.1121/1.3651093>
- Charlotte Desvages and Stefan Bilbao, "Two-Polarisation Physical Model of
  Bowed Strings with Nonlinear Contact and Friction Forces, and Application to
  Gesture-Based Sound Synthesis": <https://doi.org/10.3390/app6050135>
- Stefan Bilbao, `Numerical Sound Synthesis: Finite Difference Schemes and
  Simulation in Musical Acoustics`:
  <https://www.research.ed.ac.uk/en/publications/numerical-sound-synthesis-finite-difference-schemes-and-simulatio/>
- David Medine, "Dynamical Systems for Audio Synthesis: Embracing
  Nonlinearities and Delay-Free Loops":
  <https://www.mdpi.com/2076-3417/6/5/134>
- R. Plomp and W. J. M. Levelt, "Tonal Consonance and Critical Bandwidth":
  <https://doi.org/10.1121/1.1909741>
- Eberhard Zwicker and Hugo Fastl, `Psychoacoustics: Facts and Models`:
  <https://link.springer.com/book/10.1007/978-3-662-09562-1>
- Anne Caclin et al., "Acoustic correlates of timbre space dimensions":
  <https://doi.org/10.1121/1.1929229>
- Tudor Popescu et al., "The pleasantness of sensory dissonance is mediated by
  musical style and expertise": <https://doi.org/10.1038/s41598-018-35873-8>

## Physical Thesis

Many acoustic instruments are not simply linear resonators with a source
attached. They are nonlinear, self-excited, gesture-dependent systems.

Examples:

- A reed instrument couples a nonlinear exciter to resonant air-column modes.
- A brass instrument couples lip oscillation, bore resonance, and nonlinear
  propagation at high sound pressure.
- A bowed string couples stick-slip friction, string modes, bow force, bow
  velocity, and contact position.

The important point for synthesis:

```text
the sound-producing regime can change qualitatively when a physical control
crosses a threshold
```

That is the physical meaning of bifurcation-coordinate synthesis. The player
does not only move a timbre knob. The player moves an artificial instrument
through stability, onset, mode competition, branch switching, and instability.

## Acoustic Regimes As Playable States

### Non-Oscillating State

Physical analogy:

- too little blowing pressure in a reed/brass system
- bowing condition outside the playable region
- insufficient energy to sustain a mode

Psychoacoustic result:

- silence
- breath/friction noise
- unstable pre-onset transient

Synthesis use:

- expressive near-silence
- pressure-before-tone behavior
- playable threshold rather than envelope-only gate

### Stable Periodic Oscillation

Physical analogy:

- normal speaking reed tone
- stable brass note
- Helmholtz-like bowed-string motion

Psychoacoustic result:

- stable pitch
- high pitch strength
- repeatable timbre

Synthesis use:

- this is the pitch anchor
- every radical regime needs a way back here

### Multistability

Physical analogy:

- two or more stable periodic regimes under the same control values
- register ambiguity
- different basins of attraction selected by attack or initial condition

Reed-instrument bifurcation work is directly relevant here. The Acta Acustica
reed model studies equilibrium and periodic branches as blowing pressure
changes. Recent work on attack transients and basins of attraction frames the
player's attack path as a way to navigate which stable regime becomes audible.

Psychoacoustic result:

- same note and current controls can feel different depending on how it was
  reached
- attack is part of timbre identity
- memory becomes audible without randomness

Synthesis use:

```text
performance path -> basin selection -> audible branch
```

This is a strong reason to pursue this concept. It creates a physically
grounded version of "the instrument remembers the phrase".

### Period Doubling And Subharmonics

Physical analogy:

- nonlinear systems can shift from one cycle per nominal period to a doubled
  period
- some instrument models show additional periodic branches through
  period-doubling behavior

Psychoacoustic result:

- octave-under or subharmonic growl
- increased instability
- stronger roughness if subharmonic components interact with existing partials

Synthesis use:

- `Baklja`-like edge without simply adding distortion
- controlled growl before chaotic collapse

Design caveat:

- if the period-doubled regime appears before pitch is anchored, the instrument
  can feel broken instead of expressive.

### Quasiperiodicity

Physical analogy:

- two oscillatory regimes interact without locking
- beating between modes
- mode competition without full collapse

Psychoacoustic result:

- slow beating becomes motion or fluctuation
- faster beating enters roughness territory
- spectral sidebands can read as shimmer, beating, or instability

Synthesis use:

- a safer middle ground between clean periodic tone and chaos
- useful for living pads, unstable brass-like tones, and swarm-like identity

### Chaos / Aperiodic Bounded Motion

Physical analogy:

- nonlinear acoustic systems can enter non-periodic regimes under strong
  forcing, delay, friction, coupling, or nonlinear propagation

Psychoacoustic result:

- reduced pitch strength
- broadband or noise-like components
- high salience if transient or bounded
- fatigue if continuously dense and unstructured

Synthesis use:

- not a default tone source
- useful as a bounded event, transition, or edge state
- must remain deterministic and finite

The musical goal is not "make chaos". The useful goal is to make a controllable
edge where a played gesture can approach and recover from instability.

## Wind-Instrument Grounding

The nonlinear wind-instrument literature matters because it gives a direct
physical bridge from control pressure to regime change.

Relevant grounded ideas:

- wind instruments include nonlinear source-resonator coupling
- blowing pressure can be treated as a bifurcation parameter
- equilibrium branches and periodic branches can be tracked numerically
- stable periodic regimes can correspond to different registers
- hysteresis and multistability are musically meaningful

For synthesis, a reed-like abstraction does not need to copy a clarinet. It can
borrow the regime structure:

```text
pressure below threshold -> no stable tone
pressure crosses Hopf/fold region -> speaking tone
pressure and resonance relation change -> register branch
attack path changes -> different basin
```

Mamut design implication:

```text
Pec can act like pressure/heat.
Horizont can enlarge stability margin and headroom.
Baklja can lower the distance to branch instability.
Gravitacija can deepen or bias an attractor basin.
```

This is an interpretation for future work, not a reference claim.

## Brass And Nonlinear Propagation

The brass reference is important for a second reason: nonlinear sound
propagation can change timbre by enriching high-frequency content. Myers et al.
connect brassiness with nonlinear propagation in brass bores and examine
spectral enrichment using spectral centroid.

Physical lesson:

```text
large acoustic amplitude + nonlinear propagation -> spectral enrichment
```

Psychoacoustic lesson:

```text
higher spectral centroid -> brighter / sharper / more projecting timbre
```

Synthesis implication:

- a bifurcation-coordinate voice can make instability audible as a brightening
  trajectory, not only as noise
- nonlinear spectral enrichment should be measured with spectral centroid,
  peak, and roughness proxies
- headroom must be part of the coordinate system, because uncontrolled
  nonlinear brightening can turn into clipping

## Bowed-String Grounding

Bowed strings are the clearest physical example of "playability region".

The Desvages/Bilbao model includes nonlinear contact and friction forces and
uses a numerical scheme designed for stability under highly nonlinear
conditions. The broader bowed-string literature treats bow force, bow velocity,
and bow position as gesture controls that decide whether motion is stable,
noisy, or unstable.

Physical lesson:

```text
gesture controls are not decorative; they decide whether the instrument speaks
correctly
```

Synthesis implication:

- coordinate ranges should be designed like playable regions
- "edge" controls should guide the player toward instability and recovery
- a model can be physically inspired without copying violin sound

This is especially relevant for live Mamut use: the player needs a surface that
can be learned. Randomized parameter chaos is not a playable surface.

## Psychoacoustic Axes

### Pitch Strength

Stable periodic motion usually supports stronger pitch perception than
aperiodic or weakly periodic motion. A radical voice can become expressive only
if it preserves a path back to pitch anchoring.

Practical rule:

```text
one stable periodic attractor should remain reachable from every radical state
```

### Roughness And Beating

Plomp and Levelt's tonal consonance work and the Zwicker/Fastl psychoacoustic
model tradition connect sensory dissonance and roughness to beating inside
auditory critical-band behavior.

Practical synthesis meaning:

- slow modulation reads as movement or fluctuation
- mid-rate beating reads as roughness
- very fast or very dense interaction can become separate partials, noise, or
  brightness

The exact percept depends on frequency region, level, spectrum, and listener
context. The important engineering point is that bifurcation regimes are not
only mathematical categories; they land in known auditory categories:

```text
quasiperiodic beating -> fluctuation / roughness
period doubling -> subharmonic pitch and growl
chaotic bounded motion -> noisiness, reduced pitch strength, texture
nonlinear propagation -> brightness / sharpness
```

### Timbre Space

Timbre perception is multidimensional. The Caclin et al. timbre-space study
tests several acoustic correlates and finds attack time, spectral centroid, and
spectrum fine structure to be major determinants in dissimilarity judgments.
Older and related timbre-space work is consistent with treating timbre as a
mix of temporal and spectral cues rather than a single brightness number.

Practical measurements for future evidence:

```text
attack_time
spectral_centroid
spectral_flux
spectral_irregularity
roughness_proxy
pitch_strength_proxy
subharmonic_energy
noise_floor / broadband_energy
```

This gives a way to evaluate a radical oscillator without relying only on
subjective language.

### Sensory Dissonance Is Not Always Bad

The Scientific Reports study on sensory dissonance shows that pleasantness is
mediated by musical style and expertise. That matters for Mamut because the
goal is not to minimize roughness. The goal is to make roughness controllable,
recoverable, and stylistically meaningful.

Practical rule:

```text
roughness is a performance resource, not an error, until it breaks control,
headroom, or pitch intention
```

## Physical-To-Psychoacoustic Mapping

| Physical / dynamical event | Likely acoustic feature | Likely perceptual role |
| --- | --- | --- |
| Hopf onset | growing periodic waveform | tone birth, breath-to-note |
| fold / branch jump | sudden amplitude or spectrum change | threshold, snap, register change |
| stable limit cycle | harmonic or nearly harmonic periodicity | pitch anchor |
| period doubling | subharmonic components | growl, undertone, instability |
| quasiperiodic torus | beating, sidebands | motion, shimmer, roughness |
| mode coupling | energy transfer between partial regions | living body, register color |
| nonlinear propagation | high-frequency enrichment | brightness, brassiness, projection |
| bounded chaos | broadband deterministic irregularity | texture, danger, reduced pitch |
| multistability | path-dependent regime selection | performance memory |

## Coordinate Design From A Player's View

A playable bifurcation-coordinate instrument should expose physical-feeling
controls, not raw equations.

Candidate performer-level controls:

```text
pressure:
  distance from onset, amplitude drive, nonlinear energy

mass:
  inertia, slower response, deeper basin, lower apparent body

edge:
  distance to fold, period-doubling, or rupture

coupling:
  mode interaction, beating, register competition

openness:
  damping margin, spectral spread, recovery from dense regimes

memory:
  basin persistence, slow state recovery, attack-path sensitivity
```

Mamut mapping hypothesis:

```text
gravitacija -> mass / basin depth / mode locking
pec         -> pressure / thermal drive / nonlinear saturation
baklja      -> edge / doubling / rupture proximity
horizont    -> openness / damping margin / recovery
swarm       -> coupling spread / quasiperiodic motion
```

Again: this is a future design hypothesis, not something established by the
external references.

## Evidence Needed Before Implementation Claims

A future BCS prototype should produce both audio and measurements:

- baseline stable tone
- near-onset gesture
- branch-jump or hysteresis gesture
- period-doubling/subharmonic gesture
- quasiperiodic beating gesture
- recovery gesture back to stable pitch

For each render:

```text
WAV artifact
seed / initial state
event stream
peak and RMS
NaN / Inf / denormal count
spectral centroid
subharmonic energy
roughness proxy
pitch-strength proxy
regime diagnostic
listening verdict
```

The first listening bar should be simple:

```text
Can the player hear a physical threshold, branch, or instability that is not
just ordinary distortion, chorus, filter sweep, or random noise?
```

If not, the concept is not ready for engine integration.

## Boundary For Next Work

Do next:

- build a tiny offline playground
- choose one canonical model family
- render short A/B examples
- measure psychoacoustic proxies
- write evidence before live integration

Do not do yet:

- do not wire this into standalone
- do not expose GUI controls
- do not add patch schema fields
- do not call it a new instrument until evidence exists

The physical target is a deterministic nonlinear instrument that feels like it
has pressure, memory, threshold, and recovery. The psychoacoustic target is not
maximum novelty. It is audible control over pitch strength, brightness,
roughness, subharmonic motion, and recoverable instability.
