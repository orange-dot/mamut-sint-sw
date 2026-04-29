# DSP Primitives Math (EPM1)

## Purpose

This document is the math companion for the low-level DSP blocks used by
`EPM1`. The normative topology still lives in the sibling `mamut-sint-hw`
document `docs/dsp-subsystem-spec.md`.
This file describes the current Rust implementation in:

- `../../crates/mamut-dsp/src/lib.rs`

It is descriptive, not a request for new DSP behavior.

The primitives described here are direct Rust implementation code. They are not
delegated to a DSP graph/runtime library such as `fundsp`, `rodio`, `dasp`, or a
plugin SDK; host/MIDI/audio I/O dependencies live outside this primitive layer.

## Shared Conventions

Samples are `f32` and nominally live in `-1.0..=1.0`. Most user-facing controls
are clamped to `0.0..=1.0` before they reach a DSP primitive.

Two utility equations appear throughout the DSP path:

```text
mix(a, b, amount) = a + (b - a) * clamp(amount, 0, 1)

soft_clip(x, asymmetry):
  offset = clamp(asymmetry, -1, 1) * 0.35
  y = tanh((x + offset) * 1.25)
```

`sanitize_sample(x)` returns `clamp(x, -1, 1)` for finite values and `0.0` for
`NaN` or infinity.

## Oscillator

Each oscillator stores a normalized phase in one cycle:

```text
phase in [0, 1)
increment = clamp(frequency_hz / sample_rate_hz, 0, 0.5)
phase_next = phase + increment
if phase_next >= 1:
  phase_next -= 1
  wrapped = true
else:
  wrapped = false
```

The primitive waveforms are intentionally simple:

```text
saw(phase)      = phase * 2 - 1
pulse(phase, w) =  1 when phase < clamp(w, 0.05, 0.95)
                  -1 otherwise
triangle(phase) = 1 - 4 * abs(phase - 0.5)
square(phase)   = pulse(phase, 0.5)
```

Hard sync scales the slave phase toward zero:

```text
phase = phase * (1 - clamp(amount, 0, 1))
```

Musically, this is a cheap deterministic sync reset rather than an analog
emulation. It creates sharper motion as `amount` rises without adding an extra
runtime dependency.

## Noise

The noise source is a per-voice linear congruential generator:

```text
state = state * 1664525 + 1013904223   // wrapping u32 arithmetic
normalized = clamp(state / u32::MAX, 0, 1)
noise = normalized * 2 - 1
```

Each voice receives a deterministic slot-derived seed. That keeps dry runs and
tests repeatable while still giving every voice its own noise stream.

## ADSR Envelope

Envelope timings are clamped to at least `1 ms`; sustain is clamped to
`0.0..=1.0`.

```text
attack_samples  = max(attack_ms  * 0.001 * sample_rate_hz, 1)
decay_samples   = max(decay_ms   * 0.001 * sample_rate_hz, 1)
release_samples = max(release_ms * 0.001 * sample_rate_hz, 1)
```

Per sample:

```text
Idle:
  level = 0

Attack:
  level += 1 / attack_samples
  if level >= 1: level = 1; stage = Decay

Decay:
  level += (sustain - level) / decay_samples
  if abs(level - sustain) < 0.001: level = sustain; stage = Sustain

Sustain:
  level = sustain

Release:
  level += (0 - level) / release_samples
  if abs(level) < 0.0005: level = 0; stage = Idle
```

The attack is linear. Decay and release are one-pole approaches, so they feel
smooth and avoid hard corners.

## Linear Smoother

For a target reached over `n` samples:

```text
step = (target - current) / n
current += step while samples_remaining > 0
current = target when the counter reaches zero
```

If `n == 0`, the value jumps immediately. The engine uses this for direct DSP
parameters after macro and patch resolution, before the render path consumes
them.

## State Variable Filter

The filter is a low-pass state-variable style block with internal soft clipping.
It stores `low` and `band`.

Inputs are clamped first:

```text
cutoff_hz = clamp(cutoff_hz, 20, sample_rate_hz * 0.42)
resonance = clamp(resonance, 0, 1)
drive     = clamp(drive, 0, 1)
strain    = clamp(strain, 0, 1)
```

Coefficient and pressure setup:

```text
frequency  = clamp(sin(pi * cutoff_hz / sample_rate_hz) * 1.92, 0, 1.8)
damping    = clamp(1.95 - resonance * 1.34 - strain * 0.24, 0.12, 1.95)
input_gain = 1 + drive * 1.9 + strain * 0.35
stage_input = soft_clip(input * input_gain, strain * 0.20)
blend = clamp(0.42 + drive * 0.24 + strain * 0.12, 0, 1)
```

The internal update runs twice per sample:

```text
high = stage_input - low - damping * band
band += frequency * high * 0.5
band = soft_clip(band * (1 + drive * 0.10), strain * 0.05)
low += frequency * band * 0.5
low = mix(low, soft_clip(low * (1 + drive * 0.18), strain * 0.08), blend)
stage_input = low
```

Output:

```text
output = mix(low, low + band * 0.10, strain * 0.32)
output = soft_clip(output, strain * 0.14 + resonance * 0.04)
```

The important behavior is that `drive` and `strain` are not just post-filter
distortion. They change the input gain, internal state pressure, and output
blend, which is why the filter can feel loaded rather than merely darker.

## Chorus

The chorus owns stereo delay buffers sized at roughly `80 ms`:

```text
buffer_len = max(round(sample_rate_hz * 0.08), 4)
base_delay = sample_rate_hz * 0.014
modulation = sample_rate_hz * 0.006 * depth
```

Three sine LFO taps use phase offsets:

```text
lfo_a = sin(lfo_phase * tau)
lfo_b = sin(fract(lfo_phase + 0.31) * tau)
lfo_c = sin(fract(lfo_phase + 0.63) * tau)
```

Delay times:

```text
left_delay  = base_delay + modulation * lfo_a
right_delay = base_delay + modulation * lfo_b
cross_delay = base_delay * 0.74 + modulation * 0.45 * lfo_c
```

Reads use linear interpolation between adjacent samples. The write path feeds
back a small amount:

```text
feedback = 0.08 + depth * 0.12
left_buffer[write]  = sanitize(left  + delayed_left  * feedback)
right_buffer[write] = sanitize(right + delayed_right * feedback)
lfo_phase = fract(lfo_phase + max(rate_hz, 0.01) / sample_rate_hz)
```

Wet signal and output:

```text
wet_left  = delayed_left  * 0.66 + cross_left  * 0.22 + center * 0.12
wet_right = delayed_right * 0.66 + cross_right * 0.22 + center * 0.12
dry_gain = 1 - mix * 0.72
wet_gain = mix * 0.78
```

This is a scale and image effect. It is not required for the dry synth body to
work.

## Reverb

The reverb owns stereo buffers sized at roughly `340 ms`:

```text
buffer_len = max(round(sample_rate_hz * 0.34), 8)
```

For each sample:

```text
tap_a_offset = round(buffer_len * (0.16 + size * 0.28))
tap_b_offset = round(buffer_len * (0.31 + size * 0.22))
feedback = clamp(0.34 + size * 0.40, 0, 0.88)
diffusion = 0.16 + size * 0.22
damping_blend = 1 - damping * 0.88
```

The damping state follows delayed and cross-fed taps:

```text
damped_left  += ((delayed_left  + cross_left  * diffusion) - damped_left)  * damping_blend
damped_right += ((delayed_right + cross_right * diffusion) - damped_right) * damping_blend
```

The write path cross-feeds the damped sides:

```text
left_buffer[write]  = sanitize(left  * 0.42 + damped_right * feedback)
right_buffer[write] = sanitize(right * 0.42 + damped_left  * feedback)
```

Output:

```text
wet_left  = delayed_left  * 0.48 + tap_left  * 0.18 + cross_left  * 0.26
wet_right = delayed_right * 0.48 + tap_right * 0.18 + cross_right * 0.26
dry_gain = 1 - mix * 0.58
wet_gain = mix * (0.62 + size * 0.10)
```

The result is deliberately compact: enough space and tail to finish patches,
but not a general-purpose reverb architecture.

## Safety Notes

The primitives are allocation-free during steady-state per-sample processing
after their buffers are created. Long-tail effects sanitize their buffer writes,
and public output buffers are sanitized by the engine.
