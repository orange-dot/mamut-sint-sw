# GFM v1.0 Engine Layer Evidence

Date: 2026-04-30

This slice is the first offline-only engine graph integration of GFM. The
existing synth voice remains primary, and an opt-in GFM layer is mixed into the
`mamut-engine` stereo render path through `Engine::process_block`.

There is no MIDI, ALSA, UI, standalone runtime, patch schema migration, or C
kernel integration in this slice.

## Engine Layer Contract

- Opt-in API: `Engine::enable_offline_gfm_layer(seed)`
- Disable API: `Engine::disable_offline_gfm_layer()`
- Selection metadata: `Engine::offline_gfm_layer_selection()`
- Program metadata: `Engine::offline_gfm_layer_program_id()`
- Diagnostics: `Engine::offline_gfm_layer_diagnostics()`
- Factory source: `GfmFieldVoice::from_patch`
- Render path: `Engine::process_block` -> `Engine::render_frame`

The default `Engine` output is unchanged. GFM is only mixed when the offline
layer is explicitly enabled. Low-score patches still build no GFM voice and
render byte-identically to the baseline engine output.

## Mix Contract

- The GFM layer advances sample-by-sample inside the engine render path.
- The layer is gated by existing engine audio activity so it does not become a
  standalone drone when the synth path is silent.
- The layer is mixed after chorus/reverb and before final crossfeed/output gain.
- The caller still owns the audio buffers.
- File I/O exists only in the offline example.

## Offline Render Contract

- Command: `cargo run --release -p mamut-engine --example gfm_engine_layer_render`
- Source: `crates/mamut-engine/examples/gfm_engine_layer_render.rs`
- Seed for every render: `0x6A46_4D40`
- Sample rate: `48_000 Hz`
- Duration: `14s`
- Block size used by example: `256 frames`
- Format: stereo PCM16 WAV
- Output directory: `target/gfm-render`

The example renders a fixed program-aware chord gesture through the existing
engine voice path for three representative factory patches. `Horizont` and
`Baklja` use the original low/mid chord. `Pec` is rendered one octave higher
for the evidence artifact because `Ember Vault` is a sub-heavy bass patch and
the first v1.0 listening pass showed it as subjectively too quiet.

- `Cathedral Bloom` -> `HorizontPerformance`
- `Ember Vault` -> `PecPerformance`
- `Razor Thaw` -> `BakljaPerformance`

## Render Evidence

| Patch | Program | Scores `(H, P, B)` | RMS | Peak | Max ruptures | Final health | SHA-256 |
| --- | --- | --- | ---: | ---: | ---: | --- | --- |
| Cathedral Bloom | `HorizontPerformance` | `(0.8994, 0.1359, 0.0106)` | `0.3631` | `0.5051` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `b451b80857c6b3fdd4f8e05c976b1fa672bc9fc528e319f9de109a0e132b62d5` |
| Ember Vault | `PecPerformance` | `(0.0337, 0.8188, 0.0872)` | `0.4064` | `0.4565` | `0` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `aacd5a0aa089f17cacab8e7647dde9eaf7f908f4182ce79baa8d61d6b4e00d1f` |
| Razor Thaw | `BakljaPerformance` | `(0.0290, 0.5180, 0.8865)` | `0.4348` | `0.4899` | `81` | `healthy=256 suspect=0 quarantined=0 recovering=0` | `30e8402d04b0a232a1a7e99471c5c51b67558d3b269a08d9c8768df2751bea70` |

All generated WAV files are RIFF/WAVE stereo PCM16 at `48_000 Hz`.

## WAV Artifacts

All generated files are under:

`/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/gfm-render`

The output filenames are:

- `layer_gfm_cathedral-bloom.wav`
- `layer_gfm_ember-vault.wav`
- `layer_gfm_razor-thaw.wav`

## Test Evidence

- A GFM-disabled low-score patch renders byte-identically to baseline engine
  output.
- GFM-enabled representative patches render different PCM from baseline engine
  output.
- The Pec layer gain was raised after the first listening pass to make the GFM
  component audible over the bass-oriented engine patch.
- Horizont/Pec layer renders are finite, bounded, and rupture-free.
- Baklja layer render is finite, bounded, produces rupture activity, and
  recovers to all `Healthy` final health at the 48 kHz evidence rate.
- Loading a new patch rebuilds the offline GFM layer from the new patch identity.

## Boundary Evidence

- `mamut-field` remains independent of `mamut-engine`, `mamut-identity`,
  `mamut-patch`, ALSA, MIDI, UI, and standalone runtime crates.
- `mamut-engine` has a one-way dependency on `mamut-field`.
- The patch schema is unchanged.
- The live MIDI/ALSA/standalone paths are unchanged.
- The render hot path performs no logging or file I/O.

## Validation Commands

- `cargo fmt --all`
- `cargo test -p mamut-engine offline_gfm_layer -- --nocapture`
- `cargo test -p mamut-engine`
- `cargo test -p mamut-field`
- `cargo build --release -p mamut-engine --examples`
- `cargo run --release -p mamut-engine --example gfm_engine_layer_render`
- `sha256sum target/gfm-render/layer_gfm_*.wav`
- `file target/gfm-render/layer_gfm_*.wav`
- `cargo tree -p mamut-field`
- `cargo tree -p mamut-engine`

## Listening Verdict

User listening verdict: accepted.
