# GFM v0.3 Gesture Variants Evidence

Date: 2026-04-30

This freezes the first combined gesture-variants evidence set for
`mamut-field`. It remains offline-only evidence; there is no `mamut-engine`,
MIDI, ALSA, UI, patch schema, or C kernel integration.

## Render Contract

- Command: `cargo run --release -p mamut-field --example gfm_gesture_variants_render`
- Source: `crates/mamut-field/examples/gfm_gesture_variants_render.rs`
- Lattice: `GfmLattice<16, 16>`
- Seed for every render: `0x6A46_4D30`
- Sample rate: `48_000 Hz`
- Duration: `12s`
- Format: mono PCM16 WAV
- Output directory: `target/gfm-render`

## Gesture Sections

The same combined `GfmExcitation` take is used for all three postures:

| Section | Time | Shape |
| --- | --- | --- |
| Short strike | `0.00s..0.42s` | `15ms` attack, fast decay |
| Slow press | `3.00s..7.50s` | `1.0s` ramp, `2.2s` hold, `1.3s` release |
| Repeated strike | `8.20s..9.86s` | three short strikes at `8.20s`, `8.85s`, `9.50s` |

Derived excitation fields:

- `pressure = short * 0.80 + slow * 0.45 + repeated * 0.70`, clamped to `0.85`
- `heat = pressure * 0.58`
- `rupture_bias = pressure * 0.62`

The probe remains fixed at the center of the lattice.

## WAV Artifacts

| Render | Path | SHA-256 |
| --- | --- | --- |
| Variants Horizont | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/variants_horizont.wav` | `a60bfffd0f5beaedc162962d5dff4823f440df628644b1537c34fb2705abfb39` |
| Variants Pec | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/variants_pec.wav` | `7787db0f005222968e4530513dad9fc9ff20f8e8d0854232eac42c9a2a52f3ca` |
| Variants Baklja | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/variants_baklja.wav` | `5ca5298fc239cdf1720c7ed25e0f5ed24c2f4d20b2c9429ec55663a41579d263` |

## Render Stats

| Render | RMS | Peak | Max ruptures | Final health |
| --- | ---: | ---: | ---: | --- |
| Variants Horizont | `0.0425` | `0.2486` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Variants Pec | `0.0617` | `0.2260` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Variants Baklja | `0.1189` | `0.5660` | `78` | `healthy=1 suspect=255 quarantined=0 recovering=0` |

## Musical Params

The base posture params are from `GfmParams::{horizont, pec, baklja}` at
`48_000 Hz`, with example output gain overrides:

| Render | Posture | `output_gain` | `grav_coupling` | `heat_dispersion` | `spatial_spread` | `omega_dispersion` | `thermal_noise` | `ruin` | `rupture_threshold` | `rupture_response` | `rupture_quorum` |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Variants Horizont | `Horizont` | `0.90` | `0.34` | `0.18` | `0.82` | `0.12` | `0.02` | `0.03` | `1.36` | `0.02` | `5` |
| Variants Pec | `Pec` | `0.84` | `0.68` | `0.98` | `0.38` | `0.98` | `0.48` | `0.04` | `1.56` | `0.02` | `6` |
| Variants Baklja | `Baklja` | `0.40` | `1.42` | `0.36` | `0.30` | `0.24` | `0.035` | `0.98` | `0.34` | `0.86` | `2` |

## Listening Verdict

User listening verdict: accepted. The combined gesture variants are musically
useful as-is; `Pec` and `Baklja` remain somewhat close, but the set is worth
keeping as the `GFM v0.3` baseline.

Engineering read:

- `Horizont` and `Pec` remain rupture-free across the combined gesture take.
- `Baklja` stays finite and bounded, with max rupture count under the combined
  gesture limit used by tests.
- `Baklja` ends mostly `Suspect`, so this evidence is intentionally not a green
  light for engine integration. A later slice should add gesture recovery or
  lower-energy performance variants before live voice-graph work.
