# GFM v1.2 Standalone Runtime Flag Evidence

Date: 2026-04-30

This slice wires the existing engine-owned GFM layer gate into
`mamut-standalone` as a launch-time opt-in. Default standalone behavior remains
`GfmLayerMode::Disabled`.

There is no patch schema, factory patch, profile, MIDI mapping, ALSA behavior,
or live UI toggle change in this slice.

## CLI Contract

- `play` accepts `--gfm-layer-seed <u64-or-0xHEX>`.
- `dry-run` accepts `--gfm-layer-seed <u64-or-0xHEX>`.
- Without the flag, the standalone engine keeps `GfmLayerMode::Disabled`.
- With the flag, standalone applies `GfmLayerMode::Enabled { seed }` immediately
  after `Engine::new`.
- The seed parser accepts decimal and `0x`/`0X` hexadecimal forms; underscores
  are accepted for Rust-style readability.
- The launch-time mode is preserved across standalone runtime rebuilds caused by
  patch switching, audio switching, or ALSA rollback/restart paths.
- Runtime `status` and `dry-run` print the same GFM summary line.

Status line forms:

- `gfm: mode=disabled`
- `gfm: mode=enabled seed=0x6A464D40 selected=HorizontPerformance active=HorizontPerformance scores=(0.8994,0.1359,0.0106) ruptures=0`

## Dry-Run Evidence

Command:

`cargo run -p mamut-standalone -- dry-run cathedral-bloom`

Result includes:

`gfm: mode=disabled`

Enabled commands:

- `cargo run -p mamut-standalone -- dry-run --gfm-layer-seed 0x6A464D40 cathedral-bloom`
- `cargo run -p mamut-standalone -- dry-run --gfm-layer-seed 0x6A464D40 ember-vault`
- `cargo run -p mamut-standalone -- dry-run --gfm-layer-seed 0x6A464D40 razor-thaw`

Observed GFM lines:

- Cathedral Bloom: `selected=HorizontPerformance active=HorizontPerformance scores=(0.8994,0.1359,0.0106)`
- Ember Vault: `selected=PecPerformance active=PecPerformance scores=(0.0337,0.8188,0.0872)`
- Razor Thaw: `selected=BakljaPerformance active=BakljaPerformance scores=(0.0290,0.5180,0.8865)`

The standalone dry-run renders one short block, so the Razor Thaw dry-run line
does not exercise the longer Baklja rupture/recovery window. That remains
covered by the engine/field GFM recovery tests and v1.1 A/B evidence.

## Test Evidence

- `cargo fmt --all`
- `cargo fmt --all --check`
- `cargo test -p mamut-standalone`
  - 38 passed
- `cargo test -p mamut-engine`
  - 32 passed
- `cargo test -p mamut-field`
  - 18 passed
- `cargo build --release -p mamut-standalone`
- `cargo tree -p mamut-field`
- `cargo tree -p mamut-engine`

Boundary searches:

- `rg -n "GfmLayerMode|set_gfm_layer_mode|gfm_layer|gfm-layer" crates/mamut-patch patches profiles`
  - no matches
- `rg -n "Gfm|gfm" profiles crates/mamut-patch patches crates/mamut-standalone/src/main.rs | rg "RuntimeUiCommand|parse_runtime_ui_command|profiles|patches|mamut-patch|gfm"`
  - GFM matches are limited to standalone launch option parsing, runtime build
    propagation, status/dry-run printing, and tests.

## Boundary Evidence

- `mamut-field` remains independent and has no crate dependencies in
  `cargo tree -p mamut-field`.
- `mamut-engine` dependency shape is unchanged.
- The patch schema and factory patches are unchanged.
- Profiles and MIDI mappings are unchanged.
- ALSA device selection, buffering, and recovery behavior are unchanged except
  that rebuilt engines receive the same launch-time GFM mode.
- No interactive runtime command or UI toggle was added for GFM mode.

## Live Take Follow-Up

The first headless standalone GFM runtime-flag take is locked in:

`docs/live-sessions/2026-04-30-gfm-cathedral-bloom-gravitacija-headless.md`

Summary:

- patch: `cathedral-bloom`
- focus: Cathedral Bloom with Gravitacija movement
- command mode: `play --headless --gfm-layer-seed 0x6A46_4D40`
- selected/active program: `HorizontPerformance`
- artifact:
  `target/gfm-live/cathedral-bloom-gravitacija-30s.wav`
- format: stereo `pcm_f32le` WAV, `44100 Hz`, `30.000000s`
- frames: `1323000`
- dropped recording frames: `0`
- SHA-256:
  `27c92b469efbd881c2337ec7cedb84566ac0752c3b03a58c018df0d7f9d8660f`

The artifact is finite and bounded:

- overall peak: `-2.899711 dB`
- overall RMS: `-19.669862 dB`
- NaN/Inf/denormal counts: `0/0/0`

Final live transport status reported `xrun_recoveries=3` and high underrun
counters. The recorder still completed the target duration with no dropped
frames, so this is kept as a host stability caveat rather than a GFM gate
failure.

## Listening Verdict

User listening verdict: accepted for the current v1.2 evidence lock.
