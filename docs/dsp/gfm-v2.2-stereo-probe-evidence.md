# GFM v2.2 Stereo Field Probe Evidence

Date: 2026-07-07

Backlog: `EPM1_BACKLOG_SET1_GFM_PLAYABLE_FIELD.md`, item `SET1-2`.

Before this slice `read_probe_output` read a single 5-tap center cross and the
engine duplicated that mono sample to both channels; all stereo character of
the GFM layer came from the artificial mix-stage spread
(`stereo_width`/`stereo_crossfeed`) in `mamut-engine`. This slice reads a
second, spatially offset tap set so left and right carry genuinely
decorrelated field state, and retires the artificial spread for this layer.
Probe reads are read-only passes over the lattice; the field update is
unchanged, so the cost is a second center-cross read.

## What Landed

`mamut-field`:

- `GFM_STEREO_PROBE_OFFSET_COLUMNS = 2` and
  `GfmLattice::stereo_probe_positions()` — the left/right tap centers are the
  mono center offset by ±2 columns (toroidal), same row.
- `GfmLattice::next_sample_stereo{,_with_excitation}` — one field step read
  through both offset tap sets, returning `(left, right)`. The field update
  (`sample_neighbors` + `update_fields_health_and_phase`) is byte-identical to
  the mono path; only the readout differs. `read_probe_output` is refactored
  to `read_probe_output_at(center_x, center_y)` and the mono entry point calls
  it at the center — the mono render path is unchanged.
- Diagnostics record the mono fold-down `(L+R)/2` as `last_output` and the
  louder channel as the peak, so existing safety/diagnostics semantics carry
  over.

`mamut-engine`:

- `GfmFieldVoice::next_sample_stereo_with_live_control` and
  `stereo_probe_positions()`. The mono and stereo live paths share
  `live_step_excitation`, so excitation shaping is identical; only the readout
  differs.
- `Engine::apply_gfm_layer` consumes the stereo pair. The former
  `stereo_width * 0.16 + stereo_crossfeed * 0.04` artificial spread is removed
  for this layer; the layer gain (`gate * program_gain * effective_amount`) is
  applied equally to both channels, and each channel takes its own field tap.
  The `final_asymmetry` soft-clip bias is retained.

The existing mono readout stays for offline renders and every existing test;
the mono offline performance/gesture examples are unchanged.

## Decorrelation Measurement

Harness: `crates/mamut-field/examples/gfm_stereo_probe_ab_render.rs`. Per
production program it renders a mono take (center probe) and a stereo take
(offset pair, plus a lockstep mono reference for cross-correlation), and
hard-fails if the render is non-finite, non-deterministic across runs, the
L/R correlation reaches 0.999, or either channel exceeds the unit ceiling.

48 kHz, 6 s, v0.4 performance gesture, seed `0x6A46_4D40`, measured
2026-07-07:

| program | corr(L,R) | corr(L,mono) | corr(R,mono) | L−R diff RMS | fold RMS | fold peak | mono RMS | mono peak |
|---|---|---|---|---|---|---|---|---|
| horizont | 0.2804 | 0.2539 | 0.2920 | 0.0525 | 0.0350 | 0.1501 | 0.0474 | 0.2162 |
| pec      | 0.1482 | 0.3577 | 0.3722 | 0.0765 | 0.0444 | 0.1756 | 0.0657 | 0.2253 |
| baklja   | 0.4973 | 0.2901 | 0.7688 | 0.2914 | 0.2514 | 0.6973 | 0.2509 | 0.7029 |

- The left/right channels are strongly decorrelated (cross-correlation
  0.15–0.50, well under the 1.0 a dual-mono duplicate would show).
- The mono fold-down `(L+R)/2` stays bounded (fold peak ≤ 0.70) and close in
  level to the pre-change mono render (compare fold RMS to mono RMS), so a
  mono monitor loses no musical body.
- Both channels finite and under the unit ceiling.

## Cost Note

A stereo step is the mono field update plus a second read-only center-cross
tap (5 taps): no extra lattice update, no allocation, no branch on the field
state. The per-sample delta is one additional probe read; SET2-1's bench (not
yet landed) is the place to quantify it. Qualitatively it is negligible next
to the 16×16 neighbor-aggregate update the step already runs.

## Engine Integration Evidence

`mamut-engine` tests (`crates/mamut-engine/src/tests/gfm_layer.rs`):

- `gfm_field_voice_stereo_live_control_is_decorrelated_and_deterministic` —
  the exact readout `apply_gfm_layer` consumes: per program the live stereo
  pair is bit-identical across two voices (determinism), finite, unit-bounded,
  decorrelated (corr < 0.999), and channel-distinct (diff RMS > 0.001).
- `gfm_layer_stereo_mix_output_is_finite_bounded_and_not_dual_mono` —
  end-to-end, an armed layer over a near-mono patch lands measurable side-band
  energy in the final mix (side/mid power ratio > 1e-6), stays finite and
  under the master safety ceiling — a dual-mono layer would leave the side
  band at zero.

## Determinism

Same seed plus same excitation yields a bit-identical `(left, right)`
sequence — covered at field level
(`same_seed_stereo_probe_renders_bit_identical_pair_sequence`) and engine
level (the determinism half of the live-control test above). The offline
example also re-renders each stereo take and hard-fails on any signature
drift.

## Read-Only Probe Proof

`stereo_stepping_matches_mono_field_evolution` steps a stereo-read lattice and
a mono-read twin over an identical excitation history, then asserts identical
diagnostics (frame index, max energy/strain, rupture count, damping events,
health histogram) and continued bit-identical mono output afterward — the
stereo readout does not perturb the field.

## Validation

Commands run (2026-07-07):

- `cargo fmt --all --check` — clean
- `cargo check --workspace --all-targets --locked` — clean
- `cargo test --locked -p mamut-field` (stereo/terrain subset shown green;
  full crate green in the workspace run)
- `cargo test --locked -p mamut-engine` (stereo/terrain subset green; full
  crate green in the workspace run)
- `cargo run --locked -p mamut-field --example gfm_stereo_probe_ab_render` —
  all in-example assertions green; table above

Stereo artifacts under `target/gfm-render/`: `stereo_ab_mono_<program>.wav`
(mono) and `stereo_ab_pair_<program>.wav` (stereo).

## Boundary Compliance

- no changes to the audio callback boundary, queue architecture, or MIDI
  ingress; stereo readout is a per-sample read-only pass, additive to the
  existing render structure
- `mamut-field` remains external-dependency-free
- no allocation, logging, formatting, or blocking added; the field update loop
  is untouched (only the readout is parameterized by tap center)
- factory bank and live-set slots untouched
- the artificial GFM-layer spread is retired in favor of real field stereo;
  the `stereo_width`/`stereo_crossfeed` direct params still govern the dry
  synth path, only the GFM layer stopped using them
