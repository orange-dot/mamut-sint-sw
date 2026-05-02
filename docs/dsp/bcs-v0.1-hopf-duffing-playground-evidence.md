# BCS v0.1 Hopf/Duffing Playground Evidence

Date: 2026-04-30

This freezes the first offline-only `Bifurcation-Coordinate Synthesis` playground
inside `mamut-field`. It is a regime proof for a deterministic nonlinear voice,
not a production tone or runtime integration.

Foundation notes:

- `bifurcation-coordinate-synthesis-math-foundations.md`
- `bifurcation-coordinate-synthesis-physical-psychoacoustic-foundations.md`

## Render Contract

- Command: `cargo run --release -p mamut-field --example bcs_v0_1_render`
- Source: `crates/mamut-field/examples/bcs_v0_1_render.rs`
- Public API: `mamut_field::bcs::*`
- Model: one Hopf/Stuart-Landau pitch-anchor mode plus one Duffing instability mode
- Base pitch: MIDI note `48`, C3, `130.81278 Hz`
- Sample rate: `48_000 Hz`
- Duration: `8s`
- Solver: fixed-step RK4 at `4x` oversampling, decimated to `48_000 Hz`
- Format: mono IEEE-float WAV, 32-bit float samples
- Output directory: `target/bcs-render`

## WAV Artifacts

| Render | Path | SHA-256 |
| --- | --- | --- |
| Stable Anchor | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/bcs-render/bcs_v0_1_stable_anchor.wav` | `47dfd802c25f71571250034e1cfcbb248a27e0f56230ae364863440ae1d8dfa8` |
| Edge Sweep | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/bcs-render/bcs_v0_1_edge_sweep.wav` | `664ded3368fa659919eb89f64a6923d7ac417dcd5532660414e983499120b303` |
| Subharmonic Pressure | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/bcs-render/bcs_v0_1_subharmonic_pressure.wav` | `63d087420f0b05fc4ad6e2827b60ad93f1e4107964333a7ff8fe6d32fa418af4` |
| Recovery Return | `/home/dev/work-base-20260421/workspace/systems/mamut-sint-sw-remote-up-21-4/target/bcs-render/bcs_v0_1_recovery_return.wav` | `eb5c3a87ffc4d3275c41d54219464512c88a3e3f2545073ae3d797525d6cedf3` |

`file target/bcs-render/bcs_v0_1_*.wav` identifies every artifact as:

```text
RIFF (little-endian) data, WAVE audio, IEEE Float, mono 48000 Hz
```

## Render Stats And Diagnostics

| Scenario | Frames | RMS | Peak | Max state | Period Hz | Ratio | Lyapunov proxy | Zero crossings | Period count |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Stable Anchor | `384000` | `0.337041` | `0.472516` | `0.750055` | `130.79733` | `0.999882` | `-0.026332` | `785` | `784` |
| Edge Sweep | `384000` | `0.388968` | `0.762890` | `1.834090` | `142.91437` | `1.0925108` | `0.005162` | `571` | `570` |
| Subharmonic Pressure | `384000` | `0.556912` | `0.766610` | `1.333618` | `64.886536` | `0.49602598` | `-0.001133` | `325` | `324` |
| Recovery Return | `384000` | `0.365096` | `0.774080` | `1.685171` | `130.69191` | `0.99907607` | `-0.001362` | `262` | `261` |

All four renders are finite, have peak absolute output below `1.0`, and remain
under the `32.0` state ceiling. Any non-finite sample or state ceiling breach
fails `render_scenario`.

## Scenario Read

- `StableAnchor`: Hopf mode dominates the readout and estimates the C3 anchor.
- `EdgeSweep`: Duffing drive and edge ramp upward while staying bounded; the
  Lyapunov proxy and max state are higher than the stable anchor.
- `SubharmonicPressure`: Duffing pressure dominates the readout and estimates
  a half-rate ratio near `0.5`.
- `RecoveryReturn`: edge pressure bursts, then the final analysis window returns
  to the C3 anchor while remaining bounded.

## Validation

Commands used:

```bash
cargo fmt --all
cargo test -p mamut-field
cargo build --release -p mamut-field --examples
cargo run --release -p mamut-field --example bcs_v0_1_render
sha256sum target/bcs-render/bcs_v0_1_*.wav
file target/bcs-render/bcs_v0_1_*.wav
rg -n "Bcs|bcs|Bifurcation|Hopf|Duffing" crates/mamut-engine crates/mamut-standalone crates/mamut-patch patches profiles
```

## Boundary

This slice intentionally changes only `mamut-field` and DSP documentation.
There are no `mamut-engine`, standalone, GUI, MIDI, ALSA, patch schema, patch
asset, profile, or runtime integration changes.

Boundary search result: no matches in `crates/mamut-engine`,
`crates/mamut-standalone`, `crates/mamut-patch`, `patches`, or `profiles`.

Listening verdict: pending user listening acceptance.
