# DSP Core Polish v0.2 Output Safety Sweep Evidence

## Summary

This slice adds public per-block output-safety telemetry to `mamut-engine` and a
release-mode factory sweep example for the existing post-render safety path.

The render order remains:

```text
render_frame
  -> master DC blocker
  -> master safety limiter
  -> sanitize / tiny flush
  -> public output buffer and peak snapshot
```

No patch schema, controller profile, GUI, MIDI map, standalone runtime behavior,
or factory patch semantics were changed.

## Public Snapshot

`EngineSnapshot` now exposes:

- `output_safety.pre_safety_peak`
- `output_safety.post_safety_peak`
- `output_safety.safety_limiter_hits`
- `output_safety.max_safety_reduction`
- `output_safety.tiny_flush_events`

Compatibility aliases remain:

- `peak_output == output_safety.post_safety_peak`
- `clip_detected` remains derived from post-safety peak behavior

## Sweep Example

Command:

```bash
cargo run --release -p mamut-engine --example core_output_safety_sweep
```

The example renders all 9 factory patches offline across 5 scenarios:

- `single-note`
- `dense-chord`
- `macro-pressure`
- `gfm-layer`
- `bcs-layer`

It writes no WAV files and adds no dependencies. Output format is stdout CSV:

```text
patch,scenario,rms,pre_peak,post_peak,limiter_hits,max_reduction,tiny_flush,finite
cathedral-bloom,single-note,0.321563,0.698424,0.698424,0,0.000000,0,true
cathedral-bloom,dense-chord,0.277159,0.905460,0.905460,0,0.000000,0,true
cathedral-bloom,macro-pressure,0.211943,0.940072,0.933365,136,0.006707,0,true
cathedral-bloom,gfm-layer,0.296146,0.768166,0.768166,0,0.000000,0,true
cathedral-bloom,bcs-layer,0.298453,0.762922,0.762922,0,0.000000,0,true
ember-vault,single-note,0.066128,0.674220,0.674220,0,0.000000,0,true
ember-vault,dense-chord,0.044233,0.496399,0.496399,0,0.000000,0,true
ember-vault,macro-pressure,0.045651,0.505708,0.505708,0,0.000000,0,true
ember-vault,gfm-layer,0.043780,0.475723,0.475723,0,0.000000,0,true
ember-vault,bcs-layer,0.052210,0.490399,0.490399,0,0.000000,0,true
furnace-choir,single-note,0.361057,1.061742,0.951196,845,0.110546,0,true
furnace-choir,dense-chord,0.277638,1.061388,0.951179,705,0.110209,0,true
furnace-choir,macro-pressure,0.050603,0.513365,0.513365,0,0.000000,0,true
furnace-choir,gfm-layer,0.166980,0.855530,0.855530,0,0.000000,0,true
furnace-choir,bcs-layer,0.365551,0.962209,0.940537,629,0.021672,0,true
glass-tide,single-note,0.327926,0.638706,0.638706,0,0.000000,0,true
glass-tide,dense-chord,0.367460,0.766855,0.766855,0,0.000000,0,true
glass-tide,macro-pressure,0.239760,0.982211,0.944346,849,0.037865,0,true
glass-tide,gfm-layer,0.343614,0.704901,0.704901,0,0.000000,0,true
glass-tide,bcs-layer,0.346324,0.691247,0.691247,0,0.000000,0,true
granite-plain,single-note,0.504000,0.719527,0.719527,0,0.000000,0,true
granite-plain,dense-chord,0.552402,1.026108,0.949049,3409,0.077059,0,true
granite-plain,macro-pressure,0.074874,1.351610,0.956607,254,0.395002,0,true
granite-plain,gfm-layer,0.471350,0.818779,0.818779,0,0.000000,0,true
granite-plain,bcs-layer,0.476611,0.782571,0.782571,0,0.000000,0,true
gravity-wake,single-note,0.422540,0.753397,0.753397,0,0.000000,0,true
gravity-wake,dense-chord,0.292585,0.953319,0.938178,754,0.015142,0,true
gravity-wake,macro-pressure,0.078069,1.043676,0.950225,239,0.093451,0,true
gravity-wake,gfm-layer,0.129553,0.942911,0.934567,122,0.008344,0,true
gravity-wake,bcs-layer,0.354170,0.763588,0.763588,0,0.000000,0,true
molten-horizon,single-note,0.462402,0.684673,0.684673,0,0.000000,0,true
molten-horizon,dense-chord,0.461553,0.837564,0.837564,0,0.000000,0,true
molten-horizon,macro-pressure,0.192042,1.177223,0.954617,3297,0.222606,0,true
molten-horizon,gfm-layer,0.432431,0.813879,0.813879,0,0.000000,0,true
molten-horizon,bcs-layer,0.442175,0.718360,0.718360,0,0.000000,0,true
razor-thaw,single-note,0.117230,0.996405,0.946255,372,0.050150,0,true
razor-thaw,dense-chord,0.048028,0.655583,0.655583,0,0.000000,0,true
razor-thaw,macro-pressure,0.047197,0.525362,0.525362,0,0.000000,0,true
razor-thaw,gfm-layer,0.040706,0.453431,0.453431,0,0.000000,0,true
razor-thaw,bcs-layer,0.051125,0.754243,0.754243,0,0.000000,0,true
sawyer-rezz,single-note,0.200604,0.713057,0.713057,0,0.000000,0,true
sawyer-rezz,dense-chord,0.130850,0.844224,0.844224,0,0.000000,0,true
sawyer-rezz,macro-pressure,0.041226,0.443206,0.443206,0,0.000000,0,true
sawyer-rezz,gfm-layer,0.086335,0.751381,0.751381,0,0.000000,0,true
sawyer-rezz,bcs-layer,0.159312,0.685564,0.685564,0,0.000000,0,true
```

Aggregate observations:

- rows rendered: 45
- finite rows: 45
- maximum pre-safety peak: `1.351610`
- maximum post-safety peak: `0.956607`
- total limiter hits: `11611`
- maximum single-sample safety reduction: `0.395002`
- tiny flush events in this sweep: `0`

Every row stayed at or below the `0.96` post-safety ceiling.

## Validation

Commands run:

```bash
cargo fmt --all
cargo fmt --all --check
cargo test -p mamut-dsp
cargo test -p mamut-engine
cargo test -p mamut-standalone
cargo build --release -p mamut-engine --examples
cargo run --release -p mamut-engine --example core_output_safety_sweep
cargo build --release -p mamut-standalone
```

Observed results:

```text
mamut-dsp:               14 passed; 0 failed
mamut-engine:            53 passed; 0 failed
mamut-standalone:        46 passed; 0 failed
release engine examples: passed
release sweep example:   passed, 45 finite rows
release standalone:      passed
fmt check:               passed
```

Focused coverage added:

- neutral low-level block reports no limiter work
- tiny flush telemetry counter covers tiny-to-zero safety behavior
- hot render reports pre-safety over-ceiling, limiter hits, and bounded post peak
- `peak_output` remains a post-safety alias
- GFM and BCS scenarios are included in the release factory sweep

## Boundary

This is `DSP Core Polish v0.2`, not a new sound design pass. The only runtime
surface change is additional engine snapshot telemetry. The factory sweep is an
offline evidence tool, not a standalone playback path.
