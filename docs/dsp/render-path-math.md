# DSP Render Path Math (EPM1)

## Purpose

This document describes the frame-level signal path implemented by the current
`EPM1` Rust engine. The normative topology remains in the sibling
`mamut-sint-hw` document `docs/dsp-subsystem-spec.md`; this file explains how
that topology is realized today in:

- `../../crates/mamut-engine/src/lib.rs`
- `../../crates/mamut-dsp/src/lib.rs`

## Block Processing

`Engine::process_block` receives sorted note and controller events plus an
optional stereo output buffer. For each frame in the requested block:

```text
apply controller events scheduled at or before frame
apply note events scheduled at or before frame
(left, right) = render_frame()
output[frame] = sanitize(left, right) when an output buffer exists
```

Controller changes refresh identity and direct parameters before later frames
render. Note events update voice state and envelopes. Output capacity can be
shorter than requested frames; engine state still advances across the requested
frame count.

## Voice Selection

The current engine uses 6 voices. On note-on, allocation chooses:

```text
first idle voice
else released voice with lowest amp envelope level, then oldest age
else sustained-released voice with lowest amp envelope level, then oldest age
else oldest held voice
```

On note-off, the oldest held voice for that pitch is released. If sustain is
down, the voice enters `SustainedReleased` and waits for pedal-up before its
envelope release begins.

## Per-Voice Pitch And Wave Mix

For each active voice, the slot index becomes a spread position:

```text
spread_position = 0 when voice_count == 1
spread_position = slot / (voice_count - 1) * 2 - 1 otherwise
spread_detune_semitones = spread_position * detune_spread_cents * 0.5 / 100
```

Pulse width is identity-sensitive:

```text
pulse_width = clamp(0.50 + baklja_edge * 0.18 - horizont_air * 0.05, 0.08, 0.92)
```

Oscillator 1 pitch:

```text
osc1_note = midi_note + pitch_bend + osc1_fine_cents / 100 + spread_detune
osc1_freq = 440 * 2 ^ ((osc1_note - 69) / 12)
```

Oscillator 1 mix:

```text
osc1_mix =
  (saw * osc1_saw
   + pulse * osc1_pulse
   + triangle * osc1_triangle
   + noise * osc1_noise * 0.85)
  / max(osc1_saw + osc1_pulse + osc1_triangle + osc1_noise, 1)
```

After oscillator 1 advances, a wrap can sync oscillator 2:

```text
if osc1_wrapped and sync_amount > 0:
  osc2_phase *= 1 - sync_amount
```

Oscillator 2 pitch and crossmod:

```text
osc2_note = midi_note + osc2_interval + osc2_fine_cents / 100 + spread_detune
crossmod_scale = clamp(1 + osc1_mix * crossmod_amount * 0.25, 0.25, 4.0)
osc2_freq = midi_note_hz(osc2_note) * crossmod_scale
```

Oscillator 2 has saw, pulse, and triangle, normalized by their mix sum.

The sub oscillator follows the note plus octave offset:

```text
sub_freq = midi_note_hz(midi_note + pitch_bend + sub_octave_offset * 12)
sub_mix = square(sub_phase) * sub_level
```

## Pre-Filter Body And Pressure

The sub body is made heavier by derived mass:

```text
mixer_body_gain = 0.4 + mixer_body_mix * 0.6
body_mix = sub_mix * mixer_body_gain * (0.62 + mass * 0.46)
```

The oscillator body is then loaded before the filter:

```text
pre_filter_gain = 1 + mixer_pre_filter_drive * 2
strain_bias = baklja_edge * 0.18
pre_filter =
  soft_clip((osc1_mix + osc2_mix + body_mix)
            * (pre_filter_gain + filter_drive * 0.8),
            strain_bias)
```

This is where oscillator density, `Pec` body, and pressure begin to become one
signal.

## Filter Cutoff And Voice Gain

Filter envelope, key tracking, and velocity all scale cutoff:

```text
filter_env = filter_env.next_sample()
keytrack = clamp(1 + ((midi_note - 60) / 48) * filter_tracking * 0.42, 0.55, 1.35)
velocity_filter_curve = velocity ^ 1.08
velocity_filter = 1 + velocity_filter_curve * velocity_to_filter * 0.85

cutoff_hz =
  clamp(direct_cutoff_hz
        * (0.42 + filter_env * filter_env_depth * 0.78)
        * keytrack
        * velocity_filter,
        20,
        sample_rate_hz * 0.42)
```

The per-voice resonant low-pass receives:

```text
filter.process(pre_filter, cutoff_hz, resonance, filter_drive, strain, sample_rate_hz)
```

Amplitude shaping:

```text
amp = amp_env.next_sample()
velocity_level = velocity ^ 0.78
velocity_gain = 1 - velocity_to_level + velocity_level * velocity_to_level
voice_level_gain = 0.40 + direct_voice_level * 0.60

sample = filtered * amp * velocity_gain * voice_level_gain
sample = soft_clip(sample * (1 + strain * 0.22), final_asymmetry * 0.14)
```

If a released voice envelope reaches idle, the voice is reset and no longer
contributes.

## Stereo Spread And Summing

Pan is slot spread multiplied by resolved stereo width:

```text
pan = spread_position * stereo_width
left_gain  = sqrt(clamp((1 - pan) * 0.5, 0, 1))
right_gain = sqrt(clamp((1 + pan) * 0.5, 0, 1))

left_sum  += sample * left_gain
right_sum += sample * right_gain
```

The equal-power square root keeps panned voices from losing too much apparent
energy.

## Final Character Stage

The engine keeps a slow body memory:

```text
body_rate = 0.022 + mass * 0.026
final_body_left  += (left_sum  - final_body_left)  * body_rate
final_body_right += (right_sum - final_body_right) * body_rate
```

Then it converts to mid/side:

```text
raw_mid  = (left_sum + right_sum) * 0.5
raw_side = (left_sum - right_sum) * 0.5
body_mid = (final_body_left + final_body_right) * 0.5
```

Focus narrows the side field under body pressure, gravity, and crossfeed:

```text
focus_amount =
  clamp(body_focus * 0.26
        + grav_pull * 0.22
        + stereo_crossfeed * 0.18,
        0,
        0.75)
```

Final mid and side:

```text
saturated_mid =
  soft_clip((raw_mid + body_mid * (0.30 + low_mid_emphasis * 0.58))
            * (1 + body_drive * 1.75 + final_saturation * 1.25),
            final_asymmetry * 0.70)

saturated_side =
  soft_clip(raw_side * (1 + final_saturation * 0.28),
            -final_asymmetry * 0.18)
  * (1 - focus_amount)

left  = saturated_mid + saturated_side
right = saturated_mid - saturated_side
```

This stage is mandatory in the DSP spec. It is where `Pec`, `Gravitacija`, and
late `Baklja` become an organism-level body rather than isolated voice color.

## Effects And Output

If enabled, chorus runs after the final character stage. If enabled, reverb runs
after chorus. Both are bypassable.

The final stereo crossfeed and output trim are:

```text
crossfeed = clamp(0.04 + stereo_crossfeed * 0.16, 0, 0.22)
left_out  = left  * (1 - crossfeed) + right * crossfeed
right_out = right * (1 - crossfeed) + left  * crossfeed

output_gain = 10 ^ (output_trim_db / 20)
```

The public output buffer receives sanitized samples. Peak and clip state are
recorded for snapshots, with clip detection currently treated as peak output at
or above `0.98`.
