# GFM v0.6 Engine Dry-Run Evidence

Date: 2026-04-30

This freezes the first one-way `mamut-engine -> mamut-field` dry-run adapter.
It proves that the engine crate can own and step the accepted `GFM v0.5`
performance contract without changing the live voice graph.

There is no patch schema, MIDI, ALSA, UI, standalone runtime, or C kernel
integration in this slice.

## Engine Adapter Contract

- Adapter type: `GfmFieldVoice`
- Constructor: `GfmFieldVoice::new(program_id, seed, sample_rate_hz)`
- Input program IDs: `GfmProgramId::ALL_PERFORMANCE`
- Gesture: fixed `GfmPerformanceGesture::v0_4()`
- State owner: `mamut-engine`
- Field model: `GfmLattice<16, 16>` from `mamut-field`
- Render step: `GfmFieldVoice::next_sample() -> f32`

The adapter owns the field lattice, program, gesture, sample rate, and frame
index. Its render step performs no file I/O, logging, patch lookup, MIDI
handling, UI access, or allocation.

## Render Contract

- Command: `cargo run --release -p mamut-engine --example gfm_engine_dry_run_render`
- Source: `crates/mamut-engine/examples/gfm_engine_dry_run_render.rs`
- Lattice: `GfmLattice<16, 16>`
- Seed for every render: `0x6A46_4D40`
- Sample rate: `48_000 Hz`
- Duration: `14s`
- Format: mono PCM16 WAV
- Output directory: `target/gfm-render`

## WAV Artifacts

| Render | Path | SHA-256 |
| --- | --- | --- |
| Engine GFM Horizont | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/engine_gfm_horizont.wav` | `6141d2eb6ac09bed495364941109399a2eb6544d78eefd42cdf2b45922b6efcb` |
| Engine GFM Pec | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/engine_gfm_pec.wav` | `8a1aec203b25def099ab711c8c79dc40d522a062978044225e752b237a23c9d5` |
| Engine GFM Baklja | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/engine_gfm_baklja.wav` | `2cdab5458d7f4121874094ad586334c7b5244d7d0a15cfbfbae05c3684437027` |

## Render Stats

| Render | RMS | Peak | Max ruptures | Final health |
| --- | ---: | ---: | ---: | --- |
| Engine GFM Horizont | `0.0414` | `0.2162` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Engine GFM Pec | `0.0620` | `0.2253` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Engine GFM Baklja | `0.2586` | `0.7029` | `81` | `healthy=256 suspect=0 quarantined=0 recovering=0` |

## Evidence Read

- Engine dry-run WAVs are byte-identical to direct `mamut-field` v0.5 WAVs at
  PCM16 artifact level.
- Engine tests compare the adapter against direct `mamut-field` rendering with
  the same program, seed, sample rate, and gesture.
- `HorizontPerformance` and `PecPerformance` remain rupture-free.
- `BakljaPerformance` produces rupture activity and returns to all `Healthy`
  final health.
- The current live `Engine::process_block` voice graph is unchanged.

## Listening Verdict

User listening verdict: accepted. The engine dry-run renders sound correct and
are accepted as the `GFM v0.6` engine sample-step baseline.
