# GFM v0.7 Engine Block Evidence

Date: 2026-04-30

This freezes the first block-rendering `mamut-engine -> mamut-field` adapter
evidence. It extends the accepted `GFM v0.6` sample-step adapter with engine
shaped block helpers while keeping the live voice graph untouched.

There is no patch schema, MIDI, ALSA, UI, standalone runtime, or C kernel
integration in this slice.

## Engine Block Contract

- Adapter type: `GfmFieldVoice`
- Mono block render: `GfmFieldVoice::render_mono_block(&mut [f32])`
- Stereo block render: `GfmFieldVoice::render_stereo_block(&mut [f32], &mut [f32])`
- Source step: both helpers call `GfmFieldVoice::next_sample()`
- Input program IDs: `GfmProgramId::ALL_PERFORMANCE`
- Gesture: fixed `GfmPerformanceGesture::v0_4()`

The mono helper advances one frame per output sample. The stereo helper renders
dual-mono and advances once per paired left/right frame. The adapter render path
performs no allocation, logging, file I/O, patch lookup, MIDI handling, UI
access, or syscalls.

## Render Contract

- Command: `cargo run --release -p mamut-engine --example gfm_engine_block_render`
- Source: `crates/mamut-engine/examples/gfm_engine_block_render.rs`
- Lattice: `GfmLattice<16, 16>`
- Seed for every render: `0x6A46_4D40`
- Sample rate: `48_000 Hz`
- Duration: `14s`
- Block size used by example: `256 frames`
- Format: mono PCM16 WAV
- Output directory: `target/gfm-render`

## WAV Artifacts

| Render | Path | SHA-256 |
| --- | --- | --- |
| Engine Block GFM Horizont | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/engine_block_gfm_horizont.wav` | `6141d2eb6ac09bed495364941109399a2eb6544d78eefd42cdf2b45922b6efcb` |
| Engine Block GFM Pec | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/engine_block_gfm_pec.wav` | `8a1aec203b25def099ab711c8c79dc40d522a062978044225e752b237a23c9d5` |
| Engine Block GFM Baklja | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render/engine_block_gfm_baklja.wav` | `2cdab5458d7f4121874094ad586334c7b5244d7d0a15cfbfbae05c3684437027` |

## Render Stats

| Render | RMS | Peak | Max ruptures | Final health |
| --- | ---: | ---: | ---: | --- |
| Engine Block GFM Horizont | `0.0414` | `0.2162` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Engine Block GFM Pec | `0.0620` | `0.2253` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` |
| Engine Block GFM Baklja | `0.2586` | `0.7029` | `81` | `healthy=256 suspect=0 quarantined=0 recovering=0` |

## Evidence Read

- Block-rendered WAVs are byte-identical to the v0.6 sample-step and direct
  `mamut-field` v0.5 WAVs at PCM16 artifact level.
- Tests confirm mono block output matches repeated `next_sample()` and direct
  `mamut-field` rendering.
- Tests confirm chunking invariance for `17`, `64`, and `251` frame chunks.
- Tests confirm stereo block output is dual-mono and advances once per rendered
  frame.
- `HorizontPerformance` and `PecPerformance` remain rupture-free.
- `BakljaPerformance` produces rupture activity and returns to all `Healthy`
  final health.
- The current live `Engine::process_block` voice graph is unchanged.

## Listening Verdict

User listening verdict: accepted. The block-rendered engine artifacts sound
correct and are accepted as the `GFM v0.7` engine block baseline.
