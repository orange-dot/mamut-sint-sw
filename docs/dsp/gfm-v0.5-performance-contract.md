# GFM v0.5 Performance Contract

Date: 2026-04-30

This freezes the first public `mamut-field` performance contract for future
engine work. It promotes the accepted `GFM v0.4` recovery-safe performance take
from local example/test code into Rust API, while staying offline-only.

There is no `mamut-engine`, MIDI, ALSA, UI, patch schema, identity mapping, or
C kernel integration in this slice.

## Public Contract

The contract is intentionally small:

- `GfmProgramId::{HorizontPerformance, PecPerformance, BakljaPerformance}`
- `GfmPerformanceProgram::new(id, sample_rate_hz)`
- `GfmPerformanceProgram::all(sample_rate_hz)`
- `GfmPerformanceProgram::{id, params, wav_file_name}`
- `GfmPerformanceGesture::v0_4()`
- `GfmPerformanceGesture::{duration_seconds, frames, excitation_at_frame}`
- `GFM_PERFORMANCE_BASELINE_SEED`
- `GFM_PERFORMANCE_DURATION_SECONDS`

`GfmPerformanceProgram` returns exact `GfmParams` values for the accepted v0.4
programs. `GfmPerformanceGesture` returns the fixed v0.4 gesture envelope. The
probe remains fixed at the center of the `16x16` lattice.

## Render Contract

- Command: `cargo run --release -p mamut-field --example gfm_performance_render`
- Source: `crates/mamut-field/examples/gfm_performance_render.rs`
- Lattice: `GfmLattice<16, 16>`
- Seed for every render: `0x6A46_4D40`
- Sample rate: `48_000 Hz`
- Duration: `14s`
- Format: mono PCM16 WAV
- Output directory: `target/gfm-render`

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

## Contract Invariants

- Same `GfmProgramId`, seed, sample rate, and `GfmPerformanceGesture::v0_4()`
  produce byte-identical PCM.
- `HorizontPerformance` and `PecPerformance` remain finite, bounded, and
  rupture-free for the fixed performance gesture.
- `BakljaPerformance` produces a rupture transient and returns to all `Healthy`
  final health.
- The example uses the public contract only; it no longer owns performance
  params or gesture shape.
- `mamut-field` remains dependency-free and Rust-only.

## Listening Verdict

User listening verdict: accepted. This v0.5 slice intentionally preserves the
same audio output as `GFM v0.4` while making the contract stable enough for
later engine integration.
