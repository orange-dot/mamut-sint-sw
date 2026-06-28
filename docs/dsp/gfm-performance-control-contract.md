# GFM Performance Control Contract

Date: 2026-05-22

## Scope

This slice makes the current three-program GFM layer patch-controllable and
playable through live performance controls. It does not implement the older
seven-program experiment surface.

Current production program set:

- `auto`
- `horizont`
- `pec`
- `baklja`

## Patch Surface

`[engine.gfm]` is optional and defaults to a neutral live-control profile:

```toml
[engine.gfm]
program = "auto"
depth = 0.55
heat = 0.35
spread = 0.50
rupture = 0.25
recovery = 0.55
motion = 0.35
body = 0.50
brightness = 0.45
```

`program = "auto"` keeps the existing identity scoring path. A pinned
`horizont`, `pec`, or `baklja` program overrides identity scoring for the GFM
voice while keeping score diagnostics visible in the snapshot.

## Runtime Contract

- `GfmParams` remains the bounded low-level field model.
- `GfmPerformanceControls` is the musical control layer above `GfmParams`.
- Existing offline performance renders may still use `GfmPerformanceGesture`.
- Engine live GFM uses `GfmLayerAmount` as wet/gate amount and
  `ChannelAftertouch` as field pressure/expression. It does not use the
  hardcoded 14 second gesture.
- Runtime seed and layer enable/disable remain session controls, not patch
  state.

## v0.2 Live Response Contract

The live path is calibrated per production program:

- `horizont`: pressure opens the field and motion; normal low/mid/high live
  sweeps stay rupture-free.
- `pec`: pressure and amount mainly drive heat/body/brightness; normal
  low/mid/high live sweeps stay rupture-free.
- `baklja`: pressure and amount scale rupture through a thresholded edge
  response, so low/mid/high do not collapse to the same rupture count.

Acceptance for the calibration sweep:

- low, mid, and high produce distinct PCM signatures.
- `horizont` and `pec` report `max_ruptures = 0`.
- `baklja` reports a scaled rupture profile where high is stronger than low.
  The profile includes rupture count plus final suspect/recovering/quarantined
  health because stronger pressure can move cells out of immediate rupture and
  into recovery.
- all renders stay finite and under the master safety ceiling.

## Evidence Path

Use the A/B render example for first-pass listening:

```sh
cargo run --locked -p mamut-engine --example gfm_engine_layer_ab_render
```

Outputs are written under `target/gfm-render`:

- `ab_base_<patch>.wav`
- `ab_gfm_low_<patch>.wav`
- `ab_gfm_mid_<patch>.wav`
- `ab_gfm_high_<patch>.wav`

Optional filters:

```sh
cargo run --locked -p mamut-engine --example gfm_engine_layer_ab_render -- --patch razor-thaw --scenario high --quality listen
```

`--quality smoke` is the default 8 kHz / 3 second pass. `--quality listen`
writes 48 kHz / 6 second files with an `ab_listen_` prefix.

The console output reports selected program, active program, live amount and
pressure, controls, RMS, peak, rupture count, and health diagnostics.

The older offline performance render remains the place for full 14 second
scripted gesture takes.
