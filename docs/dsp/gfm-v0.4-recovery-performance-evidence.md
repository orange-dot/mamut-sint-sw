# GFM v0.4 Recovery Performance Evidence

Date: 2026-04-30

This freezes the first recovery-safe performance take for `mamut-field`.
It remains offline-only evidence; there is no `mamut-engine`, MIDI, ALSA, UI,
patch schema, or C kernel integration.

## Render Contract

- Command: `cargo run --release -p mamut-field --example gfm_performance_render`
- Source: `crates/mamut-field/examples/gfm_performance_render.rs`
- Lattice: `GfmLattice<16, 16>`
- Seed for every render: `0x6A46_4D40`
- Sample rate: `48_000 Hz`
- Duration: `14s`
- Format: mono PCM16 WAV
- Output directory: `target/gfm-render`

## Performance Gesture

The same `GfmExcitation` take is used for all three postures:

| Section | Time | Shape |
| --- | --- | --- |
| Warm strike | `0.00s..0.58s` | `20ms` attack, controlled decay |
| Slow press | `2.00s..5.80s` | `1.0s` ramp, `1.8s` hold, `1.0s` release |
| Paired strikes | `6.70s..7.62s` | two medium strikes at `6.70s` and `7.28s` |
| Rupture accent | `8.80s..9.28s` | one Baklja-triggering accent, then recovery tail |

Derived excitation fields:

- `pressure = warm * 0.50 + slow * 0.32 + paired * 0.42 + accent * 0.52`, clamped to `0.60`
- `heat = pressure * 0.50 + slow * 0.10`
- `rupture_bias = pressure * 0.46 + accent * 0.10`

The probe remains fixed at the center of the lattice.

## WAV Artifacts

| Render | Path | SHA-256 |
| --- | --- | --- |
| Performance Horizont | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/performance_horizont.wav` | `6141d2eb6ac09bed495364941109399a2eb6544d78eefd42cdf2b45922b6efcb` |
| Performance Pec | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/performance_pec.wav` | `8a1aec203b25def099ab711c8c79dc40d522a062978044225e752b237a23c9d5` |
| Performance Baklja | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/performance_baklja.wav` | `2cdab5458d7f4121874094ad586334c7b5244d7d0a15cfbfbae05c3684437027` |

## Render Stats

| Render | RMS | Peak | Max ruptures | Final health |
| --- | ---: | ---: | ---: | --- |
| Performance Horizont | `0.0414` | `0.2162` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Performance Pec | `0.0620` | `0.2253` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Performance Baklja | `0.2586` | `0.7029` | `81` | `healthy=256 suspect=0 quarantined=0 recovering=0` |

## Performance Params

`Horizont` and `Pec` use their base posture params with output gain overrides.
`Baklja` keeps `GfmPosture::Baklja`, but uses a performance-safe field variant
so the rupture accent does not leave the full lattice stuck in `Suspect`.

| Render | Posture | `output_gain` | `grav_coupling` | `heat_dispersion` | `ruin` | `rupture_threshold` | `rupture_response` | `suspect_energy_ceiling` | `suspect_strain_ceiling` | `suspect_damping` | `recovery_after_samples` |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Performance Horizont | `Horizont` | `0.92` | `0.34` | `0.18` | `0.03` | `1.36` | `0.02` | `1.42` | `1.20` | `0.16` | `420` |
| Performance Pec | `Pec` | `0.86` | `0.68` | `0.98` | `0.04` | `1.56` | `0.02` | `1.28` | `1.08` | `0.24` | `360` |
| Performance Baklja | `Baklja` | `0.38` | `1.42` | `0.36` | `0.86` | `0.42` | `0.72` | `2.75` | `2.75` | `0.38` | `180` |

## Listening Verdict

User listening verdict: accepted. The recovery-safe performance take sounds
good across all three renders and is accepted as the `GFM v0.4` audio baseline.

Engineering read:

- `Horizont` and `Pec` remain finite, bounded, and rupture-free.
- `Baklja` produces a rupture transient without full-lattice health latch-up.
- The final health histogram returns to all `Healthy`, making this the first
  recovery-safe GFM performance evidence set.
- This is still not an engine integration signal; it is offline audio evidence
  for parameter and gesture shape only.
