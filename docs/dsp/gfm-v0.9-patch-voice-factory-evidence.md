# GFM v0.9 Patch Voice Factory Evidence

Date: 2026-04-30

This slice turns the v0.8 patch identity selector into a reusable
`mamut-engine` patch-to-GFM voice factory. The goal is to make engine
integration mechanical: a caller can now provide a patch, seed, and sample
rate and receive either a disabled decision or a ready `GfmFieldVoice`.

There is no MIDI, ALSA, UI, standalone runtime, live `Engine::process_block`
routing, patch schema migration, or C kernel integration in this slice.

## Factory Contract

- Config type: `GfmPatchVoiceConfig`
- Decision type: `GfmPatchVoiceDecision`
- Factory entrypoint: `GfmFieldVoice::from_patch(&PatchFileV1, GfmPatchVoiceConfig)`
- Selection metadata: `GfmVoiceProgramSelection`
- Render path: `GfmFieldVoice::render_mono_block`

Factory input:

- `PatchFileV1`
- seed
- sample rate

Factory output:

- disabled decision if all GFM program scores are below the minimum score
- enabled decision carrying the selected program metadata and a constructed
  `GfmFieldVoice`

The selector still uses the existing patch identity and macro defaults only.
No new patch fields were added.

## Render Contract

- Command: `cargo run --release -p mamut-engine --example gfm_engine_patch_selection_render`
- Source: `crates/mamut-engine/examples/gfm_engine_patch_selection_render.rs`
- Factory call: `GfmFieldVoice::from_patch`
- Seed for every render: `0x6A46_4D40`
- Sample rate: `48_000 Hz`
- Duration: `14s`
- Block size used by example: `256 frames`
- Format: mono PCM16 WAV
- Output directory: `target/gfm-render`

## Factory Patch Output

The example now uses the shared factory contract instead of manually calling
`select_gfm_program_for_patch` and then constructing `GfmFieldVoice`.

Expected outputs:

- `engine_patch_gfm_cathedral-bloom.wav`
- `engine_patch_gfm_ember-vault.wav`
- `engine_patch_gfm_furnace-choir.wav`
- `engine_patch_gfm_glass-tide.wav`
- `engine_patch_gfm_granite-plain.wav`
- `engine_patch_gfm_gravity-wake.wav`
- `engine_patch_gfm_molten-horizon.wav`
- `engine_patch_gfm_razor-thaw.wav`
- `engine_patch_gfm_sawyer-rezz.wav`

Patches selecting the same GFM program still intentionally share the same audio
hash in this slice. Patch-specific audio shaping is left for later graph
integration.

## Render Evidence

| Patch | Program | RMS | Peak | Max ruptures | SHA-256 |
| --- | --- | ---: | ---: | ---: | --- |
| Cathedral Bloom | `HorizontPerformance` | `0.0414` | `0.2162` | `0` | `6141d2eb6ac09bed495364941109399a2eb6544d78eefd42cdf2b45922b6efcb` |
| Ember Vault | `PecPerformance` | `0.0620` | `0.2253` | `0` | `8a1aec203b25def099ab711c8c79dc40d522a062978044225e752b237a23c9d5` |
| Furnace Choir | `PecPerformance` | `0.0620` | `0.2253` | `0` | `8a1aec203b25def099ab711c8c79dc40d522a062978044225e752b237a23c9d5` |
| Glass Tide | `HorizontPerformance` | `0.0414` | `0.2162` | `0` | `6141d2eb6ac09bed495364941109399a2eb6544d78eefd42cdf2b45922b6efcb` |
| Granite Plain | `HorizontPerformance` | `0.0414` | `0.2162` | `0` | `6141d2eb6ac09bed495364941109399a2eb6544d78eefd42cdf2b45922b6efcb` |
| Gravity Wake | `PecPerformance` | `0.0620` | `0.2253` | `0` | `8a1aec203b25def099ab711c8c79dc40d522a062978044225e752b237a23c9d5` |
| Molten Horizon | `HorizontPerformance` | `0.0414` | `0.2162` | `0` | `6141d2eb6ac09bed495364941109399a2eb6544d78eefd42cdf2b45922b6efcb` |
| Razor Thaw | `BakljaPerformance` | `0.2586` | `0.7029` | `81` | `2cdab5458d7f4121874094ad586334c7b5244d7d0a15cfbfbae05c3684437027` |
| Sawyer Rezz | `PecPerformance` | `0.0620` | `0.2253` | `0` | `8a1aec203b25def099ab711c8c79dc40d522a062978044225e752b237a23c9d5` |

All generated WAV files are RIFF/WAVE mono PCM16 at `48_000 Hz`.

## Test Evidence

- Factory output is sample-identical to the direct v0.8 selection render.
- Same patch, seed, and sample rate produce byte-identical PCM signatures.
- A synthetic low-score patch returns a disabled decision and constructs no
  voice.
- Horizont/Pec selected patches render finite and rupture-free.
- Baklja selected patch renders finite, produces rupture activity, and recovers
  to a safe final health state.
- Probe remains fixed through the existing `GfmFieldVoice` contract.

## Boundary Evidence

- `mamut-field` remains independent of `mamut-engine`, `mamut-identity`,
  `mamut-patch`, ALSA, MIDI, UI, and standalone runtime crates.
- `mamut-engine` has a one-way dependency on `mamut-field`.
- The patch schema is unchanged.
- The live engine voice graph is unchanged.
- The render hot path remains `GfmFieldVoice::render_mono_block` over
  caller-owned buffers; file I/O exists only in the offline example.

## Validation Commands

- `cargo fmt --all`
- `cargo test -p mamut-engine`
- `cargo test -p mamut-field`
- `cargo build --release -p mamut-engine --examples`
- `cargo run --release -p mamut-engine --example gfm_engine_patch_selection_render`
- `sha256sum target/gfm-render/engine_patch_gfm_*.wav`
- `file target/gfm-render/engine_patch_gfm_*.wav`
- `cargo tree -p mamut-field`
- `cargo tree -p mamut-engine`

## Listening Verdict

User listening verdict: accepted. The factory-rendered patch-selection
artifacts sound correct and are accepted as the GFM v0.9 patch voice factory
baseline.
