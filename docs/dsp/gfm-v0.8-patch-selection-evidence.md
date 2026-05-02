# GFM v0.8 Patch Selection Evidence

Date: 2026-04-30

This freezes the first offline patch/identity-to-GFM program selection evidence.
It maps existing factory patch identity state to the accepted GFM performance
programs without changing patch schema or the live engine voice graph.

There is no MIDI, ALSA, UI, standalone runtime, live `Engine::process_block`
routing, patch schema migration, or C kernel integration in this slice.

## Selection Contract

- Selection type: `GfmVoiceProgramSelection`
- Frame selector: `select_gfm_program(ResolvedIdentityFrame)`
- Patch selector: `select_gfm_program_for_patch(&PatchFileV1)`
- Render path: `GfmFieldVoice::render_mono_block`
- Program outputs:
  - `HorizontPerformance`
  - `PecPerformance`
  - `BakljaPerformance`

The selector scores the existing `ResolvedIdentityFrame`:

- `HorizontPerformance`: `horizont_open`, `horizont_air`, `horizont_span`, and shaped `bloom`
- `PecPerformance`: `pec_mass`, `pec_heat`, `pec_pressure`, derived `mass`, and shaped `heat`
- `BakljaPerformance`: `baklja_ready`, `baklja_edge`, `baklja_sync_bias`, derived `rupture_response`, and shaped `ruin`

If all scores are below the minimum program score, the selector returns `None`.
Current factory patches all select a program.

## Render Contract

- Command: `cargo run --release -p mamut-engine --example gfm_engine_patch_selection_render`
- Source: `crates/mamut-engine/examples/gfm_engine_patch_selection_render.rs`
- Lattice: `GfmLattice<16, 16>`
- Seed for every render: `0x6A46_4D40`
- Sample rate: `48_000 Hz`
- Duration: `14s`
- Block size used by example: `256 frames`
- Format: mono PCM16 WAV
- Output directory: `target/gfm-render`

## Factory Patch Selection

| Patch | Selected program | Scores `(H, P, B)` | RMS | Peak | Max ruptures | Final health | SHA-256 |
| --- | --- | --- | ---: | ---: | ---: | --- | --- |
| Cathedral Bloom | `HorizontPerformance` | `(0.8994, 0.1359, 0.0106)` | `0.0414` | `0.2162` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `6141d2eb6ac09bed495364941109399a2eb6544d78eefd42cdf2b45922b6efcb` |
| Ember Vault | `PecPerformance` | `(0.0337, 0.8188, 0.0872)` | `0.0620` | `0.2253` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `8a1aec203b25def099ab711c8c79dc40d522a062978044225e752b237a23c9d5` |
| Furnace Choir | `PecPerformance` | `(0.2690, 0.8021, 0.1138)` | `0.0620` | `0.2253` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `8a1aec203b25def099ab711c8c79dc40d522a062978044225e752b237a23c9d5` |
| Glass Tide | `HorizontPerformance` | `(0.8524, 0.1571, 0.0315)` | `0.0414` | `0.2162` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `6141d2eb6ac09bed495364941109399a2eb6544d78eefd42cdf2b45922b6efcb` |
| Granite Plain | `HorizontPerformance` | `(0.3421, 0.2079, 0.0118)` | `0.0414` | `0.2162` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `6141d2eb6ac09bed495364941109399a2eb6544d78eefd42cdf2b45922b6efcb` |
| Gravity Wake | `PecPerformance` | `(0.3703, 0.5020, 0.3135)` | `0.0620` | `0.2253` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `8a1aec203b25def099ab711c8c79dc40d522a062978044225e752b237a23c9d5` |
| Molten Horizon | `HorizontPerformance` | `(0.6741, 0.3190, 0.1195)` | `0.0414` | `0.2162` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `6141d2eb6ac09bed495364941109399a2eb6544d78eefd42cdf2b45922b6efcb` |
| Razor Thaw | `BakljaPerformance` | `(0.0290, 0.5180, 0.8865)` | `0.2586` | `0.7029` | `81` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `2cdab5458d7f4121874094ad586334c7b5244d7d0a15cfbfbae05c3684437027` |
| Sawyer Rezz | `PecPerformance` | `(0.1344, 0.5991, 0.5553)` | `0.0620` | `0.2253` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `8a1aec203b25def099ab711c8c79dc40d522a062978044225e752b237a23c9d5` |

## WAV Artifacts

All generated files are under:

`/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render`

The output filenames are:

- `engine_patch_gfm_cathedral-bloom.wav`
- `engine_patch_gfm_ember-vault.wav`
- `engine_patch_gfm_furnace-choir.wav`
- `engine_patch_gfm_glass-tide.wav`
- `engine_patch_gfm_granite-plain.wav`
- `engine_patch_gfm_gravity-wake.wav`
- `engine_patch_gfm_molten-horizon.wav`
- `engine_patch_gfm_razor-thaw.wav`
- `engine_patch_gfm_sawyer-rezz.wav`

Patches selecting the same GFM program intentionally share the same audio hash
in this slice. Patch-specific audio shaping is left for later integration.

## Evidence Read

- Factory patch selection is deterministic and covers all three GFM programs.
- Selected patch renders are finite and bounded.
- `BakljaPerformance` is selected for `Razor Thaw`, produces rupture activity,
  and returns to all `Healthy` final health.
- The current live `Engine::process_block` voice graph is unchanged.

## Listening Verdict

User listening verdict: pending.
