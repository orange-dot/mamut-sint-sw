# Bifurcation-Coordinate Synthesis: Mathematical Foundations

Date: 2026-04-30

## Status

This is a research foundation note, not an implementation contract.

The goal is to ground `Bifurcation-Coordinate Synthesis` in established
mathematics before inventing the Mamut-specific instrument. Any Mamut mapping
below is marked as a design implication, not as a claim from the references.

The working idea:

```text
sound = observation of a controlled nonlinear dynamical system
timbre = position in parameter space relative to bifurcations
gesture = path through that parameter space
```

This differs from a normal subtractive or FM synth because the primary musical
controls do not merely change coefficients. They move a system between
qualitatively different dynamical regimes: silence, onset, stable periodic
motion, quasiperiodic motion, subharmonics, multistability, and bounded chaos.

## Reference Discipline

External references used here:

- John Guckenheimer, "Bifurcation", Scholarpedia:
  <https://www.scholarpedia.org/article/Bifurcation>
- Yuri A. Kuznetsov, "Andronov-Hopf bifurcation", Scholarpedia:
  <https://www.scholarpedia.org/article/Andronov-Hopf_bifurcation>
- Charles Tresser, Pierre Coullet, Edson de Faria, "Period doubling",
  Scholarpedia: <https://www.scholarpedia.org/article/Period_doubling>
- Antonio Politi, "Lyapunov exponent", Scholarpedia:
  <https://www.scholarpedia.org/article/Lyapunov_exponent>
- Takashi Kanamaru, "Van der Pol oscillator", Scholarpedia:
  <https://www.scholarpedia.org/article/Van_der_Pol_oscillator>
- Takashi Kanamaru, "Duffing oscillator", Scholarpedia:
  <https://www.scholarpedia.org/article/Duffing_oscillator>
- John Guckenheimer and Philip Holmes, `Nonlinear Oscillations, Dynamical
  Systems, and Bifurcations of Vector Fields`, Springer:
  <https://link.springer.com/book/10.1007/978-1-4612-1140-2>
- Nils Berglund, "Dynamic Bifurcations: Hysteresis, Scaling Laws and Feedback
  Control": <https://doi.org/10.1143/PTPS.139.325>
- David Medine, "Dynamical Systems for Audio Synthesis: Embracing
  Nonlinearities and Delay-Free Loops":
  <https://www.mdpi.com/2076-3417/6/5/134>
- Joel Gilbert, Sylvain Maugeais, Christophe Vergez, "Minimal blowing pressure
  allowing periodic oscillations in a simplified reed musical instrument
  model": <https://doi.org/10.1051/aacus/2020026>

## Core Mathematical Object

The basic object is a controlled dynamical system:

```text
dx/dt = f(x, lambda(t), u(t))
y(t) = h(x, lambda(t), u(t))
```

where:

- `x` is the state vector.
- `lambda` is a vector of slow or semi-slow synthesis coordinates.
- `u` is the event/gesture input: gate, velocity, aftertouch, pitch bend,
  macro motion, or local excitation.
- `y` is the audio observation.

For a voice, `x` can be small:

```text
x = [position, velocity]
x = [mode1_position, mode1_velocity, mode2_position, mode2_velocity]
x = [complex_mode1, complex_mode2]
```

The synthesis claim is not that every nonlinear ODE is useful. The claim is
that a small family of controlled ODEs can become an instrument if their
parameter space is organized around bifurcations and if audio readout is
bounded, deterministic, and perceptually useful.

## Bifurcation As Timbre Boundary

A bifurcation is a qualitative change in dynamics caused by changing
parameters. In synthesis terms, a bifurcation is a boundary where the instrument
changes kind, not only color.

Examples:

- An equilibrium loses stability and a limit cycle is born.
- A periodic orbit doubles its period.
- A stable and unstable cycle collide and disappear.
- A quasiperiodic torus appears or breaks down.
- Multiple stable regimes coexist, so the current sound depends on state
  history as well as current parameters.

This gives a different control philosophy:

```text
normal synth:
  cutoff = number
  drive = number
  oscillator_mix = number

bifurcation-coordinate synth:
  onset_distance = signed distance to oscillation birth
  fold_distance = distance to hysteretic regime change
  doubling_distance = distance to subharmonic instability
  torus_distance = distance to beating/quasiperiodic motion
  basin_bias = which attractor the next gesture is likely to enter
```

The same parameter value may not imply the same sound if the system is
multistable. This is not a bug if it is intentional and deterministic; it is
the mathematical form of performance memory.

## Attractors And Regime Vocabulary

The useful vocabulary is attractor-based.

`Equilibrium`:

- State settles to rest.
- In audio: silence, decay tail, or non-oscillating pressure state.

`Stable limit cycle`:

- Periodic orbit.
- In audio: pitched tone with stable waveform.

`Quasiperiodic torus`:

- Two or more incommensurate frequencies.
- In audio: beating, shimmer, unstable chorus-like motion, register
  interference.

`Period-doubled cycle`:

- A periodic orbit becomes a new orbit with twice the period.
- In audio: subharmonic, octave-under behavior, growl, instability.

`Chaotic attractor`:

- Bounded aperiodic motion with sensitive dependence on initial conditions.
- In audio: deterministic noise-like motion, unstable but repeatable texture.

`Multiple coexisting attractors`:

- Same parameters have several possible stable regimes.
- In audio: attack path, velocity, and previous state select the resulting
  tone.

## Local Bifurcations Worth Treating As Coordinates

### Saddle-Node / Fold

Canonical normal form:

```text
dx/dt = mu - x^2
```

The number of equilibria changes as `mu` crosses zero. The important synthesis
use is not the scalar equation itself. The useful concept is the fold: a stable
state can disappear, forcing the system to jump elsewhere.

Audio implications:

- hysteresis
- threshold behavior
- "it suddenly speaks"
- "it suddenly collapses"

Potential coordinate:

```text
fold_distance
```

A positive value means the current branch still exists. A value near zero means
the voice is close to a forced transition.

### Hopf / Andronov-Hopf

Hopf bifurcation is the birth of a limit cycle from an equilibrium when a
complex-conjugate eigenvalue pair crosses the imaginary axis.

Simple normal form:

```text
dz/dt = (mu + i * omega) * z - (a + i * b) * |z|^2 * z
```

For the supercritical case with `a > 0`, a stable limit cycle appears when
`mu > 0`, with radius approximately:

```text
r = sqrt(mu / a)
```

Audio implications:

- physically meaningful tone onset
- amplitude grows from zero instead of being imposed by an envelope only
- near-threshold playing has slow attack and high sensitivity
- subcritical variants can create jump-on/jump-off behavior

Potential coordinates:

```text
hopf_mu
hopf_frequency
hopf_saturation
hopf_twist
```

The synthesis question is whether the oscillator amplitude should be born by
the ODE itself or whether a normal envelope still gates it. The radical version
lets the ODE speak.

### Period Doubling

Period doubling creates or destroys a periodic orbit with double the period.
For maps, this is tied to a multiplier crossing `-1`. In audio, the immediate
musical form is subharmonic behavior.

Audio implications:

- octave-under growl
- every-other-cycle asymmetry
- controlled instability before chaos
- a route from stable tone to rougher motion without random noise

Potential coordinate:

```text
doubling_pressure
```

This coordinate should probably be exposed only after stable pitch anchoring
exists, because period-doubling near note onset can make the instrument feel
unplayable.

### Neimark-Sacker / Torus Bifurcation

For a Poincare map of a periodic orbit, a complex pair of multipliers crossing
the unit circle can create an invariant torus. The continuous-time audio
meaning is that a stable oscillation gains a second incommensurate motion.

Audio implications:

- beating
- quasiperiodic beating between modes
- slow shimmer or fast roughness depending on frequency separation
- a controlled route from pitch to motion without losing all identity

Potential coordinates:

```text
torus_distance
secondary_mode_ratio
secondary_mode_growth
```

### Hopf-Hopf

Hopf-Hopf is a codimension-two event where two Hopf bifurcation curves meet.
The Scholarpedia article describes the case where an equilibrium has two pairs
of purely imaginary eigenvalues. Nearby dynamics can include limit cycles,
tori, and complicated interactions.

Audio implications:

- two modal centers can become active at once
- mode competition can become a first-class timbre axis
- beating and branch switching can be represented in the same mathematical
  family

A minimal amplitude-level form:

```text
dr1/dt = r1 * (b1 + a11*r1^2 + a12*r2^2)
dr2/dt = r2 * (b2 + a21*r1^2 + a22*r2^2)
dphi1/dt = w1
dphi2/dt = w2
```

This is attractive for synthesis because the amplitudes can be shaped by a
small interaction matrix while phases produce audio-rate oscillation.

Potential coordinates:

```text
mode1_birth
mode2_birth
cross_suppression
cross_excitation
frequency_ratio
```

## Canonical Oscillator Families

### Van Der Pol

The van der Pol oscillator is governed by nonlinear damping:

```text
d2x/dt2 - epsilon * (1 - x^2) * dx/dt + x = 0
```

Near `x = 0`, damping is negative, so the equilibrium is unstable. For large
state magnitude, damping becomes positive, so trajectories are pulled into a
bounded region. The result is a stable limit cycle.

Synthesis value:

- self-sustaining tone
- automatic amplitude regulation
- useful model of excitation that feeds small motion and restrains large
  motion

Risk:

- relaxation-oscillation regimes can become too pulse-like if not tuned.

### Duffing

The forced Duffing oscillator:

```text
d2x/dt2 + delta*dx/dt + beta*x + alpha*x^3 = gamma*cos(Omega*t)
```

models a nonlinear spring. Positive and negative cubic stiffness correspond to
hardening and softening behavior in the common mechanical interpretation. The
forced system can exhibit nonlinear resonance, multistability, and chaos.

Synthesis value:

- pitch bends with amplitude through stiffness nonlinearity
- fold-like jump phenomena under forcing
- strong physical metaphor for material under stress

Risk:

- forcing at audio rate can easily create broadband content and aliasing if
  not oversampled or bounded.

### Stuart-Landau / Hopf Normal Form

The Stuart-Landau oscillator is the cleanest synthesis laboratory for Hopf:

```text
dz/dt = (mu + i*omega) z - (a + i*b) |z|^2 z
```

Synthesis value:

- explicit onset control through `mu`
- explicit pitch through `omega`
- explicit nonlinear saturation through `a`
- optional amplitude-dependent frequency shift through `b`

It is less physically specific than Duffing or van der Pol, but safer as a
first mathematical playground.

## Bifurcation Coordinates As A Control Space

The phrase "coordinate synthesis" should mean that controls are defined by
their dynamical role.

Possible coordinate families:

```text
onset:
  hopf_mu
  fold_distance
  negative_damping

stability:
  damping
  saturation
  lyapunov_guard

spectral shape:
  cubic_stiffness
  amplitude_frequency_twist
  observation_nonlinearity

motion:
  torus_distance
  secondary_mode_ratio
  coupling_strength

instability:
  doubling_pressure
  chaos_drive
  basin_bias
```

Design implication for Mamut:

```text
gravitacija -> attraction, damping, basin depth, mode locking
pec         -> drive, heat, saturation, nonlinear pressure
baklja      -> fold/doubling proximity, rupture, unstable edge
horizont    -> stability margin, torus openness, widened mode spacing
swarm       -> coupling heterogeneity, quasiperiodic mode spread
```

That mapping is not yet an implementation plan. It is the first coordinate
language to test against rendered evidence.

## Dynamic Bifurcation And Gesture

Most bifurcation diagrams assume fixed parameters. A played instrument changes
parameters over time.

For a slowly varying parameter:

```text
dx/dt = f(x, lambda(t))
```

the trajectory may not switch exactly when the static bifurcation diagram says
the old regime is gone. Delay, hysteresis, and rate-induced transitions can
appear. Berglund's dynamic bifurcation review is relevant because musical
gesture is exactly this problem: the player pushes a parameter path through a
changing system.

Design implication:

```text
same endpoint + different attack curve -> different tone
```

This is valuable. It gives the instrument performance memory without random
state.

## Audio Observation

Do not assume `x` is directly audio. The readout is part of the instrument:

```text
y = h(x, lambda, u)
```

Useful readout choices:

- one state coordinate: `y = x1`
- weighted modal readout: `y = dot(w, x)`
- velocity readout: `y = dx/dt`
- nonlinear readout: `y = tanh(g * dot(w, x))`
- energy-normalized readout:

```text
y = dot(w, x) / sqrt(epsilon + energy(x))
```

The readout must be deterministic and bounded. A chaotic state with unsafe
readout is not an instrument; it is a failure mode.

## Numerical Issues

Medine's audio-synthesis paper is relevant because it treats nonlinear audio
systems as ODE networks and discusses explicit numerical solution for real-time
use. For a first Mamut prototype, the numerical target should be boring:

- fixed-size state
- no allocation in the render path
- deterministic update order
- parameter smoothing
- finite output for macro sweeps
- bounded energy
- clear sample-rate contract

Candidate solvers:

- explicit Euler: too crude except for slow control probes
- semi-implicit Euler: useful for some mechanical systems
- RK4: common reference solver and good for offline proof
- trapezoidal/implicit methods: safer for stiff systems but more expensive and
  harder to keep simple

For a first evidence slice, prefer a small non-stiff system with RK4 offline
and a measured path to cheaper integration later.

## Diagnostics Required Before Audio Claims

The synthesis engine should be able to report:

```text
finite: bool
peak_abs: f32
energy: f32
estimated_period: Option<f32>
regime: Silent | Periodic | Quasiperiodic | PeriodDoubled | ChaoticLike
max_state_abs: f32
lyapunov_proxy: f32
active_attractor_id: Option<u32>
```

The `lyapunov_proxy` does not need to be a formal proof in v0.1. It can be a
local divergence estimate from two nearby trajectories in offline analysis.
Formal Lyapunov exponents become useful once a candidate model survives
listening.

## First Mathematical Playground

The least risky first playground is not a full chaotic synth. It is a small
two-mode Hopf/Duffing hybrid:

```text
mode 1: stable pitch anchor
mode 2: nonlinear instability color
coupling: energy transfer and suppression
readout: bounded weighted sum
controls: onset, stiffness, coupling, doubling pressure
```

Reason:

- one mode can preserve playable pitch
- the second mode can carry bifurcation character
- coupling can create audible branch behavior
- diagnostics remain tractable

What not to do first:

- do not start with high-dimensional chaos
- do not expose all parameters as MIDI controls
- do not use random noise as a substitute for deterministic instability
- do not claim novelty until a rendered A/B shows behavior unavailable from
  the current EPM1 path

## Acceptance Bar For A Future Prototype

A future implementation slice should not be accepted unless it can show:

- deterministic output for the same event stream and initial state
- finite bounded output under macro sweeps
- at least one audible bifurcation-coordinate behavior:
  - onset threshold
  - subharmonic period doubling
  - hysteretic branch change
  - quasiperiodic beating
- a measurable regime diagnostic matching the rendered behavior
- a written reason why the result is not just ordinary distortion, LFO, chorus,
  or FM

The mathematical identity of this concept is not "chaos makes interesting
noise". The identity is controlled motion through the topology of nonlinear
regimes.
