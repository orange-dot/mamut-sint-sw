# DSP Control And Identity Math (EPM1)

## Purpose

This document explains how public controls become render-facing DSP parameters.
The normative DSP topology lives in the sibling `mamut-sint-hw` document
`docs/dsp-subsystem-spec.md`. This file describes the current Rust
implementation in:

- `../../crates/mamut-identity/src/lib.rs`
- `../../crates/mamut-engine/src/lib.rs`

## Control Layers

The engine keeps three control layers separate:

1. Raw public macro state: `gravitacija`, `bloom`, `heat`, `ruin`, `swarm`
2. Resolved identity state: `Horizont`, `Pec`, `Baklja`, and gravity fields
3. Direct DSP parameters consumed by oscillators, filter, envelopes, final
   stage, chorus, and reverb

No DSP primitive reads raw host-facing macro values directly. `Engine` refreshes
identity and direct parameters after macro/controller changes, then smooths
direct parameters before rendering.

## Macro Shaping

Every macro starts clamped to `0.0..=1.0`. Each patch provides a response curve:

```text
scaled = clamp(raw * sensitivity, 0, 1)
```

Curve variants:

```text
linear:
  shaped = scaled

soft_plus:
  shaped = mix(scaled, sqrt(scaled), softness)

late_rise:
  shaped = mix(scaled, scaled ^ 1.6, softness)

early_rise:
  shaped = mix(scaled, 1 - (1 - scaled) ^ 1.6, softness)
```

All shaped outputs are clamped again to `0.0..=1.0`.

`soft_plus` gives earlier audible motion. `late_rise` delays intensity until the
upper part of the control range. `early_rise` makes a control speak sooner and
then flatten.

## Late Stage Helper

Some identity terms only activate above a threshold:

```text
late_stage(value, threshold):
  if value <= threshold:
    0
  else:
    clamp((value - threshold) / (1 - threshold), 0, 1)
```

The current implementation uses:

```text
late_gravity = late_stage(gravitacija, 0.68)
extreme_swarm = late_stage(swarm, 0.82)
```

## Identity State

Identity state is the musical meaning layer. Bias values come from the patch.

`Horizont`:

```text
horizont_open =
  clamp01(bloom * 0.70
          + (1 - gravitacija) * 0.25
          + horizont_bias * 0.35
          - ruin * 0.10
          - heat * 0.05)

horizont_air =
  clamp01(bloom * 0.78
          + (1 - heat) * 0.08
          + horizont_bias * 0.22
          - late_gravity * 0.18)

horizont_span =
  clamp01(bloom * 0.58
          + swarm * 0.32
          + horizont_bias * 0.16
          - gravitacija * 0.16)
```

`Pec`:

```text
pec_mass =
  clamp01(heat * 0.60
          + gravitacija * 0.25
          + pec_bias * 0.40
          + swarm * 0.05
          - bloom * 0.05)

pec_heat =
  clamp01(heat * 0.80
          + gravitacija * 0.12
          + pec_bias * 0.25)

pec_pressure =
  clamp01(gravitacija * 0.58
          + heat * 0.24
          + ruin * 0.08
          + gravitacija_pressure_bias * 0.35)
```

`Baklja`:

```text
baklja_ready =
  clamp01(ruin * 0.56
          + gravitacija * 0.28
          + baklja_bias * 0.42
          + extreme_swarm * 0.08)

baklja_edge =
  clamp01(ruin * 0.68
          + late_gravity * 0.40
          + baklja_bias * 0.18)

baklja_sync_bias =
  clamp01(ruin * 0.72
          + heat * 0.06
          + baklja_bias * 0.20)

grav_pull = gravitacija
```

## Derived State

Derived state is hidden render pressure:

```text
mass =
  clamp01(pec_mass * 0.70 + pec_heat * 0.20 + grav_pull * 0.10)

strain =
  clamp01(baklja_edge * 0.40 + pec_pressure * 0.38 + grav_pull * 0.22)

headroom =
  clamp01(0.78 + horizont_air * 0.18 + bloom * 0.08
          - grav_pull * 0.46
          - heat * 0.14
          - baklja_ready * 0.08)

body_focus =
  clamp01(pec_mass * 0.50 + grav_pull * 0.28 + body_focus_bias * 0.35
          - bloom * 0.18)

rupture_threshold =
  clamp01(0.78 - baklja_ready * 0.22 - grav_pull * 0.16
          + rupture_threshold_bias * 0.22)

rupture_response =
  clamp01(baklja_edge * 0.55 + strain * 0.25 + ruin * 0.20)

spatial_dispersion =
  clamp01(horizont_span * 0.55 + swarm * 0.33 - grav_pull * 0.10)
```

In practical terms, `mass` makes the instrument heavier, `strain` pushes
nonlinearity, `body_focus` narrows and concentrates the organism, and
`spatial_dispersion` keeps wide motion available when the identity supports it.

## Performance Controls

Performance controls modify effective macro state before identity resolution:

```text
aftertouch = clamp(channel_aftertouch, 0, 1) ^ 0.82
mod_wheel  = clamp(mod_wheel, 0, 1) ^ 0.92

gravitacija += aftertouch * aftertouch_to_gravitacija
ruin        += aftertouch ^ 1.08 * aftertouch_to_baklja
bloom       += mod_wheel * mod_wheel_to_bloom
swarm       += mod_wheel ^ 0.88 * mod_wheel_to_swarm
```

Pitch bend is applied later at direct parameter resolution and clamped to the
patch's `bend_range_semitones`.

## Direct DSP Parameters

Direct parameters are the bridge from identity to DSP math.

Cutoff and stereo:

```text
cutoff_scale = 0.90 + bloom * 0.62 + horizont_air * 0.18 - gravitacija * 0.40
cutoff_hz = clamp(engine_filter_cutoff_hz * cutoff_scale, 20, 20000)

stereo_width =
  clamp(engine_voice_stereo_width + horizont_span * 0.18, 0, 1)

stereo_crossfeed =
  clamp(0.10 + body_focus * 0.28 + grav_pull * 0.10
        - spatial_dispersion * 0.10,
        0,
        1)
```

Oscillator pressure:

```text
sub_level =
  clamp(engine_sub_level * (0.70 + mass * 0.30), 0, 1)

osc2_interval =
  engine_osc2_interval + clamp(pitch_bend, -bend_range, bend_range)

sync_amount =
  clamp(engine_osc2_sync_amount + baklja_sync_bias * 0.30, 0, 1)

crossmod_amount =
  clamp(engine_osc2_crossmod_amount + baklja_ready * 0.24, 0, 1)

detune_spread_cents =
  clamp(engine_detune_spread_cents * (0.72 + swarm * 0.42), 0, 50)
```

Filter and envelopes:

```text
resonance =
  clamp(engine_resonance + baklja_edge * 0.14 + ruin * 0.06 - mass * 0.04,
        0,
        1)

filter_drive =
  clamp(engine_filter_drive + pec_heat * 0.18 + strain * 0.10, 0, 1)

filter_env_depth =
  clamp(engine_filter_env_depth + horizont_open * 0.12, 0, 1)

filter_tracking =
  clamp(engine_keytrack * (0.92 - mass * 0.12), 0, 1)
```

Final character:

```text
voice_level =
  clamp(0.75 + mass * 0.20 + velocity_to_level * 0.05, 0, 1)

body_drive =
  clamp(engine_body_drive + body_focus * 0.25, 0, 1)

final_saturation =
  clamp(engine_body_drive + mass * 0.14 + strain * 0.08, 0, 1)

final_asymmetry =
  clamp(engine_asymmetry + baklja_edge * 0.22, 0, 1)

low_mid_emphasis =
  clamp(engine_low_mid_emphasis + mass * 0.15, 0, 1)
```

Chorus and reverb enabled/mix/depth/rate/size/damping currently pass through
from the patch or direct parameter changes.

## Smoothing Boundary

The engine smooths selected direct parameters over:

```text
control_smoothing_samples =
  clamp(round(sample_rate_hz * 0.006), 8, 512)
```

The smoother runs after identity/direct resolution and before per-sample DSP
consumption. This is the boundary that prevents macro and controller motion from
becoming zipper noise in cutoff, drive, stereo, saturation, and related
continuous parameters.
