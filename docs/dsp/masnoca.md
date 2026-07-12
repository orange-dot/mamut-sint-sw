# Masnoca, Punoca, Toplina I Snaga

## Status

This is a curated tonal-architecture note for `EPM1`. It synthesizes prior local
research drafts into one repo-facing document.

It is not an implementation contract. Treat the numbers below as starting
points for listening tests and bounded DSP experiments, not as frozen product
requirements.

Raw research drafts and generated survey reports are intentionally ignored by
git. Evidence documents and live-session notes remain the source of truth for
what has actually been tested.

## Core Thesis

Analog instruments do not sound full because "analog" is a magic property.
They sound full when many small physical limits act together:

- the oscillator is a mechanism, not only a waveform
- phase, pitch, and reset behavior are never perfectly static
- the mixer, filter input, VCA, and output stage all carry some load
- nonlinearities are distributed rather than concentrated in one final effect
- voices are similar enough to stay musical but different enough to avoid
  sterile phase locking

For Mamut vocabulary:

- `Punoca` means the tone has no hollow center.
- `Masnoca` means mass in the fundamental and low-mid region, with grouped
  partials and mild compression.
- `Toplina` means fewer brittle edges, stronger lower harmonics, and a natural
  spectral tilt.
- `Snaga` means perceptual closeness and concentrated energy, not just higher
  peak level.
- `Zivost` means slow, bounded motion on musical axes: pitch, phase, amplitude,
  pressure, and bias.

Short form:

```text
masnoca is organized mass across the whole signal chain
```

It is not a single EQ move, drive knob, or post-effect.

## What Is And Is Not Novel Here

Intellectual honesty about this approach, so the rest of the document is not
mistaken for a novelty claim:

- Distributed nonlinearity, oscillator-as-mechanism, driven feedback-aware
  filters, and bus glue are the **established analog-modeling method**. Every
  serious circuit-modeled synth already spreads mild nonlinearity across the
  chain. None of this is a Mamut invention or a new algorithm.
- The DSP building blocks are standard and well known: `tanh` saturation,
  cubic soft-clip, asymmetric diode shaping, wavefolding, and a driven
  state-variable filter (`mamut-dsp::NonlinearResonantSvf`, selected as the
  `matter_driven` filter model). Composing them across stages is conventional
  voicing craft, not novel signal processing.
- What `EPM1` actually contributes here is **discipline, not algorithm**:
  bounded, deterministic, allocation-free, testable placement of standard
  nonlinearities, mapped through identity macros. The value is reliability and
  repeatability, not a sound no one else can make.
- The genuinely distinctive nonlinear-dynamics work in this program lives
  elsewhere — `mamut-field` (BCS bifurcation-coordinate synthesis, GFM phase
  lattice), not in this note. This document is voicing doctrine for a
  conventional VA-plus-saturation path.

## Mamut Context

`mamut-sint-sw` is the current software/runtime line for `EPM1`.
`mamut-sint-hw` is the hardware continuation in `EPM2`.

The healthy split is:

- software owns recall, deterministic state, patch validation, macro mapping,
  voice allocation, and repeatable tests
- hardware can later own physical weight, component-level nonlinearities,
  calibration behavior, and voice character

`EPM1` should not pretend that a generic final "warmth" block is enough. The
software engine should translate concrete physical lessons into explicit DSP
sites while keeping deterministic tests and realtime boundaries intact.

## Physical Sources Of Weight

### Oscillator Mechanism

An analog VCO is not only a saw, pulse, or triangle. It is a small physical
system:

- an exponential converter creates current from control voltage
- that current charges a timing capacitor
- a threshold decides when reset occurs
- the reset stage discharges the ramp
- sync injects force into that reset relationship

Reset depth, reset speed, threshold behavior, and tiny timing variation all
change the harmonic result. This is why a convincing oscillator model should
eventually describe more than an ideal phase accumulator.

### Beating And Drift

Unison thickness comes from controlled divergence:

- detuned sources produce slow amplitude and spectral beating
- higher harmonics spread faster than the fundamental
- randomized note-on phase avoids static cancellation
- slow drift keeps phase relationships moving without destroying pitch center

Useful drift is not random chaos. The pitch spine must remain readable.

### Load, Saturation, And Filter Pressure

Analog mixers, filter inputs, VCAs, and output stages are not neutral matrices.
When pushed, they:

- add even and odd harmonic content
- shorten crest factor
- shift energy into lower and middle harmonics
- soften hard transient edges
- make the sound feel closer without simply raising peak level

The filter is especially important because resonance is not only Q. A driven
filter is a nonlinear feedback system, so the signal and the filter affect each
other rather than passing through an isolated linear block.

### Shared Body

Analog voices are never perfectly isolated. Power rails, ground impedance,
component tolerances, and output stages create small shared-body effects. In
DSP terms, that means voice summing should not be treated as a forever-neutral
addition step.

The useful digital translation is subtle:

- bus saturation
- low-level cross-voice pressure
- dynamic headroom behavior
- final-stage memory or envelope-dependent bias

This should stay controlled and measurable. It is not an excuse for unstable
output.

## Psychoacoustic Reading

People rarely hear masnoca as a pure bass boost. More often they hear:

- a strong enough fundamental
- second and third harmonics that support the center
- low-mid continuity between bass and upper mids
- mild saturation that lowers empty headroom
- motion that feels alive but not out of tune

Toplina is also not the same as darkness. A sound can be open and still warm if
the high end is not brittle and the lower harmonics are musically organized.

Snaga is not peak level. A signal with slightly lower crest factor and better
energy placement can feel stronger at the same peak.

## Digital Implications

Digital gets precision and recall for free. It does not get analog
imperfection for free. Every useful imperfection must be explicitly modeled.

| Aspect | Naive digital risk | Useful Mamut direction |
| --- | --- | --- |
| Oscillator phase | fixed phase cancellation | seeded note-on phase variation |
| Pitch | static perfection | slow per-voice random walk |
| Saw/sync | aliasing or ideal reset | band-limited reset behavior |
| Mixer | neutral summing only | pre-filter pressure and asymmetry |
| Filter | linear IIR color | driven feedback-aware filter behavior |
| Voice sum | isolated voices | subtle bus glue |
| Output | static waveshaper | dynamic final character stage |
| Noise floor | dead silence | optional very-low-level texture |

The danger is fake analog: random drift, generic distortion, or one final
"warmth" block that does all the work. The better path is distributed,
bounded, and testable behavior.

## Current EPM1 Posture

The current `EPM1` engine is already strong in several important areas:

- macro identity is not just one exposed knob
- `mass`, `strain`, `headroom`, `body_focus`, `rupture_threshold`,
  `rupture_response`, and `spatial_dispersion` create a useful hidden state
  vocabulary
- `Pec` and `Gravitacija` are not reduced to one distortion bus
- sub presence, pre-filter pressure, filter drive, and final body shaping are
  already treated as separate tonal sites
- the factory bank already authors weight from source, mix, filter, and final
  stage instead of relying only on effects

The weaker area is also clear:

- the oscillator source is still less physically specific than the downstream
  shaping path
- sync is functional but not yet as mechanically convincing as a saw-core reset
  model
- voice individuality is not yet a deep source of tone

That is acceptable for the current product phase, but it identifies the next
high-value DSP direction.

## Implementation Directions

### 1. Better Oscillator Source

The next meaningful sound-quality jump should come from the source:

- band-limited saw and pulse behavior
- finite reset behavior
- sync reset correction
- seeded phase variation
- frequency- and pressure-dependent color where justified by listening tests

This is more valuable than adding another generic effect.

### 2. Controlled Voice Individuality

Add individuality as bounded musical state:

- slow pitch drift per voice
- small envelope time variation per voice
- small filter bias variation per voice
- shared slow chassis-style movement where useful

The goal is body in chords, not unstable tuning.

### 3. Richer Pre-Filter Pressure

`Pec` belongs strongly in the pre-filter region:

- level-dependent oscillator weighting
- sub-specific drive law
- mild asymmetry for even-harmonic support
- body-dependent saturation curves

This should remain allocation-free and cheap enough for the render path.

### 4. Filter And Nonlinearity Discipline

Filter work should prefer physically meaningful nonlinear placement:

- saturation inside or around feedback when the model requires it
- anti-aliasing around nonlinear stages
- finite-output tests under macro sweeps
- clear cost accounting before any heavier model enters the hot path

ZDF or ladder-like approaches are candidates, but only after a bounded proof.

### 5. Dynamic Final Character Stage

The final stage should behave like part of the instrument, not a mastering
effect:

- low-mid weighted saturation
- mild asymmetry
- dynamic headroom behavior
- slow loaded response that changes between one note and dense chords

`Gravitacija` can map into drive, asymmetry, headroom, and strain together, but
the mapping must stay controllable.

## Starting Targets

These are listening-test starting points, not laws:

| Parameter | Initial range | Purpose |
| --- | ---: | --- |
| Unison detune spread | 3-8 cents | width inside the critical-band sweet spot |
| Pitch random walk | sigma around 2 cents | slow life without pitch collapse |
| Drift time scale | seconds, not frames | avoid jitter-like dirt |
| Note-on phase | seeded uniform phase | repeatable life without static cancellation |
| Very low texture floor | roughly -75 to -85 dBFS | optional air below conscious threshold |
| Envelope variation | around +/-3% | analog-like RC tolerance |
| Voice crosstalk/glue | around -50 dB | shared body without obvious modulation |
| Final-stage attack | around 10 ms | loaded response without hard pumping |
| Final-stage release | around 100 ms | sustained weight |

Any accepted value must be proven through:

- deterministic tests
- finite-output safety
- artifact or listening evidence
- no new allocation/blocking on the realtime path

## Boundary

This document is about tone architecture. It does not change:

- patch schema contracts
- transport posture (see ADR 0005)
- ALSA/runtime ownership
- evidence requirements
- factory-bank acceptance rules

The implementation truth remains in code, tests, patches, and evidence notes.
This document only names the tonal direction so future DSP work does not drift
into ad hoc "warmth" patches.
