# GFM v1.1 Layer Gate Evidence

Date: 2026-04-30

This slice promotes the v1.0 offline-only GFM layer into an explicit
engine-owned feature gate. The v1.0 evidence is frozen as accepted based on the
user listening verdict in `docs/dsp/gfm-v1.0-engine-layer-evidence.md`.

There is no MIDI, ALSA, UI, standalone runtime, patch schema migration, or C
kernel integration in this slice.

## Engine Contract

- Mode type: `GfmLayerMode::{Disabled, Enabled { seed: u64 }}`
- Snapshot type: `GfmLayerSnapshot`
- Mode setter: `Engine::set_gfm_layer_mode(mode) -> GfmVoiceProgramSelection`
- Mode reader: `Engine::gfm_layer_mode() -> GfmLayerMode`
- Diagnostics reader: `Engine::gfm_layer_diagnostics() -> Option<GfmDiagnostics>`
- `EngineSnapshot` now exposes `gfm_layer: GfmLayerSnapshot`
- `GfmLayerSnapshot` contains mode, selected `GfmVoiceProgramSelection`,
  active `Option<GfmProgramId>`, and diagnostics `Option<GfmDiagnostics>`

`Engine::new` defaults to `GfmLayerMode::Disabled`. `load_patch` and `panic`
rebuild the GFM voice only when the current mode is `Enabled { seed }`;
disabled mode keeps no active GFM voice. The previous public `offline_*` engine
methods were removed rather than kept as compatibility wrappers.

## A/B Render Contract

- Command: `cargo run --release -p mamut-engine --example gfm_engine_layer_ab_render`
- Source: `crates/mamut-engine/examples/gfm_engine_layer_ab_render.rs`
- Seed for enabled renders: `0x6A46_4D40`
- Sample rate: `48_000 Hz`
- Duration: `14s`
- Block size used by example: `256 frames`
- Format: stereo PCM16 WAV, written by the example without `hound`
- Output directory: `target/gfm-render`

Program-aware chord gesture:

- `HorizontPerformance` / `BakljaPerformance`: `[48, 55, 60]`
- `PecPerformance`: `[60, 67, 72]`

## A/B Render Evidence

All generated files are under:

`/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render`

| Patch | Mode | Program | Scores `(H, P, B)` | RMS | Peak | Max ruptures | Final health | SHA-256 |
| --- | --- | --- | --- | ---: | ---: | ---: | --- | --- |
| Cathedral Bloom | Baseline | `HorizontPerformance` | `(0.8994, 0.1359, 0.0106)` | `0.2739` | `0.8532` | n/a | n/a | `ffd8cefcffd8a7ab621203e8e64d016d7caa7b9e1e46508a4669fba0001e1f69` |
| Cathedral Bloom | GFM | `HorizontPerformance` | `(0.8994, 0.1359, 0.0106)` | `0.2674` | `0.7832` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `e554af51f0b7734bcdb4989f86b85e1bb09ef20e392ac7a50b554fdfb16b5ff1` |
| Ember Vault | Baseline | `PecPerformance` | `(0.0337, 0.8188, 0.0872)` | `0.0191` | `0.5527` | n/a | n/a | `403e2a89775c03ff5d86f7842cd20cc37aabb147bfc152d0c6d090fcf7de681f` |
| Ember Vault | GFM | `PecPerformance` | `(0.0337, 0.8188, 0.0872)` | `0.0172` | `0.4720` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `0ea0ce68b94453221d2d632ded27e4254efbb04a0acfb297706cd17fe72bf8dc` |
| Razor Thaw | Baseline | `BakljaPerformance` | `(0.0290, 0.5180, 0.8865)` | `0.0195` | `0.7666` | n/a | n/a | `7bb528ef5fe0b6cea286897f9a5d212b100a27b335bdb91dd1d1b1ff2b03b5c3` |
| Razor Thaw | GFM | `BakljaPerformance` | `(0.0290, 0.5180, 0.8865)` | `0.0181` | `0.7130` | `81` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `d0f8b40ee2096be75a4f6ca8788d2d56209456893ade0d6d8f36221bf0102cce` |

Generated filenames:

- `ab_base_cathedral-bloom.wav`
- `ab_gfm_cathedral-bloom.wav`
- `ab_base_ember-vault.wav`
- `ab_gfm_ember-vault.wav`
- `ab_base_razor-thaw.wav`
- `ab_gfm_razor-thaw.wav`

`file target/gfm-render/ab_*.wav` reported RIFF/WAVE Microsoft PCM, 16-bit,
stereo, `48000 Hz` for all six artifacts.

## Test Evidence

- `cargo fmt --all`
- `cargo test -p mamut-engine`
  - 32 passed
- `cargo test -p mamut-field`
  - 18 passed
- `cargo build --release -p mamut-engine --examples`
- `cargo run --release -p mamut-engine --example gfm_engine_layer_ab_render`
- `sha256sum target/gfm-render/ab_base_cathedral-bloom.wav target/gfm-render/ab_gfm_cathedral-bloom.wav target/gfm-render/ab_base_ember-vault.wav target/gfm-render/ab_gfm_ember-vault.wav target/gfm-render/ab_base_razor-thaw.wav target/gfm-render/ab_gfm_razor-thaw.wav`
- `file target/gfm-render/ab_base_cathedral-bloom.wav target/gfm-render/ab_gfm_cathedral-bloom.wav target/gfm-render/ab_base_ember-vault.wav target/gfm-render/ab_gfm_ember-vault.wav target/gfm-render/ab_base_razor-thaw.wav target/gfm-render/ab_gfm_razor-thaw.wav`
- `cargo tree -p mamut-field`
- `cargo tree -p mamut-engine`
- `rg -n "offline_gfm_layer|enable_offline_gfm_layer|disable_offline_gfm_layer|offline_gfm" crates/mamut-engine/src crates/mamut-engine/examples crates/mamut-field/src crates/mamut-patch/src crates/mamut-standalone/src`
  - no matches
- `rg -n "alsa|midir|egui|eframe|mamut-standalone" crates/mamut-engine crates/mamut-field crates/mamut-engine/Cargo.toml crates/mamut-field/Cargo.toml`
  - no matches
- `rg -n "GfmLayerMode|set_gfm_layer_mode|gfm_layer" crates/mamut-standalone crates/mamut-patch patches profiles`
  - no matches

## Boundary Evidence

- `mamut-field` remains independent and has no crate dependencies in
  `cargo tree -p mamut-field`.
- `mamut-engine` still has no ALSA, MIDI runtime, UI, or standalone dependency.
- The patch schema and factory patches are unchanged.
- The standalone runtime, ALSA path, MIDI path, UI path, profiles, and patch
  schema were not modified.
- The render hot path performs no logging or file I/O; file I/O exists only in
  offline examples.

## Listening Verdict

User listening verdict: pending.
