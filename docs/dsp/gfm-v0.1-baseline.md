# GFM v0.1 Baseline Evidence

Date: 2026-04-30

This freezes the first audible `mamut-field` evidence set. It is an offline
Rust render baseline only; it is not integrated into `mamut-engine`.

## Render Contract

- Command: `cargo run --release -p mamut-field --example gfm_render`
- Source: `crates/mamut-field/examples/gfm_render.rs`
- Lattice: `GfmLattice<16, 16>`
- Seed for every render: `0x6A46_4D00`
- Sample rate: `48_000 Hz`
- Duration: `4s`
- Format: mono PCM16 WAV
- Output directory: `target/gfm-render`

## WAV Artifacts

| Render | Path | SHA-256 |
| --- | --- | --- |
| Slice A low gravity | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/slice_a_low_grav.wav` | `ef05b84d1731c38a833466f8f144b33a12219e9bb35f77b2f5816b98fc78f125` |
| Slice A high gravity | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/slice_a_high_grav.wav` | `b49d6674b796338063933077ba8629de8470131ded8894c7d08fd24a30ce5f77` |
| Horizont | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/horizont.wav` | `21787082a2ad24f6ad4ff643388dd6486f0942807b54b05d5bcd10b0e4cbc083` |
| Pec | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/pec.wav` | `5b12df001da973ac6db1ce15fa090b91b53940c47a7fd6622afea3c1c3b38754` |
| Baklja | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/baklja.wav` | `aa714cddd2e4814650da249ae25bc1c767a798bfd50fe1daada1235a33f4185b` |

## Render Stats

| Render | RMS | Peak | Max ruptures | Final health |
| --- | ---: | ---: | ---: | --- |
| Slice A low gravity | `0.0185` | `0.1918` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Slice A high gravity | `0.0492` | `0.2730` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Horizont | `0.0296` | `0.1880` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Pec | `0.0558` | `0.1825` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Baklja | `0.1371` | `0.5791` | `79` | `healthy=256 suspect=0 quarantined=0 recovering=0` |

## Musical Params

| Render | Posture | `grav_coupling` | `heat_dispersion` | `output_gain` | `spatial_spread` | `omega_dispersion` | `thermal_noise` | `ruin` | `rupture_threshold` | `rupture_response` | `rupture_quorum` |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Slice A low gravity | `Skeleton` | `0.08` | `0.25` | `0.92` | `0.36` | `0.18` | `0.0` | `0.0` | `1.24` | `0.0` | `4` |
| Slice A high gravity | `Skeleton` | `1.85` | `0.25` | `0.92` | `0.36` | `0.18` | `0.0` | `0.0` | `1.24` | `0.0` | `4` |
| Horizont | `Horizont` | `0.34` | `0.18` | `0.90` | `0.82` | `0.12` | `0.02` | `0.03` | `1.36` | `0.02` | `5` |
| Pec | `Pec` | `0.68` | `0.98` | `0.84` | `0.38` | `0.98` | `0.48` | `0.04` | `1.56` | `0.02` | `6` |
| Baklja | `Baklja` | `1.42` | `0.36` | `0.40` | `0.30` | `0.24` | `0.035` | `0.98` | `0.34` | `0.86` | `2` |

## Listening Verdict

User listening verdict:

- Slice A low/high gravity: the intended contrast works.
- Horizont: acceptable as-is.
- First Pec/Baklja pass: too similar.
- Retuned Pec/Baklja pass: accepted as the v0.1 baseline.

Engineering read:

- The current baseline proves audible character, not yet instrument behavior.
- `Pec` is the thermal/omega-dense render with no rupture events.
- `Baklja` is now separated by quorum rupture and edge/fracture output.
- Next evidence should be an event/gesture render before any engine integration.
