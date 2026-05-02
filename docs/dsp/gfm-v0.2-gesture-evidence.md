# GFM v0.2 Gesture Evidence

Date: 2026-04-30

This freezes the first `mamut-field` gesture evidence set. It proves an
offline attack-hold-release response only; it is not integrated into
`mamut-engine`, MIDI, ALSA, UI, or patch schema.

## Render Contract

- Command: `cargo run --release -p mamut-field --example gfm_gesture_render`
- Source: `crates/mamut-field/examples/gfm_gesture_render.rs`
- Lattice: `GfmLattice<16, 16>`
- Seed for every render: `0x6A46_4D20`
- Sample rate: `48_000 Hz`
- Duration: `6s`
- Format: mono PCM16 WAV
- Output directory: `target/gfm-render`

## Gesture Envelope

The same per-sample `GfmExcitation` envelope is used for all three postures:

| Time | Pressure |
| --- | ---: |
| `0.00s..0.02s` | ramp `0.0 -> 1.0` |
| `0.02s..1.20s` | `1.0` |
| `1.20s..2.10s` | ramp `1.0 -> 0.0` |
| `2.10s..6.00s` | `0.0` |

Derived excitation fields:

- `heat = pressure * 0.55`
- `rupture_bias = pressure * 0.70`

The probe remains fixed at the center of the lattice.

## WAV Artifacts

| Render | Path | SHA-256 |
| --- | --- | --- |
| Gesture Horizont | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/gesture_horizont.wav` | `cbfe0d9fae9fb947fff2081b6815e96fe6e7ba1a675cfc6e90ad6a19ffbb481e` |
| Gesture Pec | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/gesture_pec.wav` | `949ad1d642b585707c48e332e8307dd70c7641a264cde1367c1abe1c9e757533` |
| Gesture Baklja | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/gesture_baklja.wav` | `ce1262600cbeae5339c0b1f9ccb736e1258dd626e1fae9598297af4f7c13ed42` |

## Render Stats

| Render | RMS | Peak | Max ruptures | Final health |
| --- | ---: | ---: | ---: | --- |
| Gesture Horizont | `0.0671` | `0.3104` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Gesture Pec | `0.0624` | `0.2508` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Gesture Baklja | `0.1201` | `0.5781` | `83` | `healthy=256 suspect=0 quarantined=0 recovering=0` |

## Musical Params

The base posture params are from `GfmParams::{horizont, pec, baklja}` at
`48_000 Hz`, with example output gain overrides:

| Render | Posture | `output_gain` | `grav_coupling` | `heat_dispersion` | `spatial_spread` | `omega_dispersion` | `thermal_noise` | `ruin` | `rupture_threshold` | `rupture_response` | `rupture_quorum` |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Gesture Horizont | `Horizont` | `0.90` | `0.34` | `0.18` | `0.82` | `0.12` | `0.02` | `0.03` | `1.36` | `0.02` | `5` |
| Gesture Pec | `Pec` | `0.84` | `0.68` | `0.98` | `0.38` | `0.98` | `0.48` | `0.04` | `1.56` | `0.02` | `6` |
| Gesture Baklja | `Baklja` | `0.40` | `1.42` | `0.36` | `0.30` | `0.24` | `0.035` | `0.98` | `0.34` | `0.86` | `2` |

## Listening Verdict

User listening verdict: accepted. The attack-hold-release gesture response is
musically useful enough to freeze as the v0.2 baseline.

Engineering read:

- `GfmExcitation::none()` preserves the original no-gesture render path.
- `Horizont` and `Pec` stay rupture-free under the shared gesture.
- `Baklja` produces bounded quorum rupture during the gesture without callback failure or lattice-wide collapse.
- This is still offline evidence; engine integration remains a later slice.
