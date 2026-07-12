# GFM Performance Control Contract

Date: 2026-05-22 (note-strike addendum 2026-07-07)

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

## v1 Note-Strike Excitation (Backlog `SET1-1`)

Since 2026-07-07 the armed live GFM layer also hears note events:

- `NoteOn` injects a spatial strike into the lattice at a deterministic,
  seed-folded cell: pitch class selects the x band, octave selects the y
  band, and the lattice seed XOR-folds a bounded per-cell offset on top
  (`note_strike_position` in `mamut-field`). Velocity scales strike
  strength. Program shaping mirrors the live excitation split: `horizont`
  and `pec` strikes never contribute rupture bias; `baklja` gains a
  thresholded rupture edge for velocities above roughly `0.58`.
- `NoteOff` injects a weaker release strike (`0.30 x` pressure, `0.50 x`
  heat, zero rupture bias) for the released voice. Release strikes fire on
  key-up regardless of the sustain pedal: the field hears the keyboard
  gesture, not the voice envelope.
- Strikes deposit bounded energy/heat/strain at the event boundary
  (`GfmLattice::inject_strike`): no allocation, no RNG draw, fixed
  footprint, clamped to the same per-cell ceilings the update loop
  enforces. The per-sample path is unchanged, and the aftertouch pressure
  path is unchanged — strikes are additive on top of the existing
  excitation contract.
- Note response is session-only in v1: `[engine.gfm]` gained no new patch
  fields. `Engine::set_gfm_note_strikes_enabled` is the session control
  (enabled by default); disabling restores the center-only excitation
  baseline.

Evidence: `gfm-v2.1-note-strike-excitation-evidence.md` and

```sh
cargo run --locked -p mamut-engine --example gfm_engine_note_strike_ab_render
```

Two further v1 changes ship alongside, neither touching the control surface
(no new `[engine.gfm]` fields):

- **Stereo field probe** (`SET1-2`): the armed GFM layer now reads a second,
  ±2-column-offset probe tap set, so left and right carry decorrelated field
  state. The former artificial mix-stage spread
  (`stereo_width`/`stereo_crossfeed`) is retired for the GFM layer only; those
  direct params still govern the dry synth path. The mono readout is unchanged
  for offline renders. Evidence: `gfm-v2.2-stereo-probe-evidence.md`.
- **INSPECT field view** (`SET1-3`): a read-only 16×16 terrain snapshot is
  published on the armed layer's `GfmLayerSnapshot::terrain` and rendered on
  the `INSPECT` screen. On-demand, off the audio path, no patch surface.
  Evidence: `gfm-v2.3-inspect-field-view-evidence.md`.

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
