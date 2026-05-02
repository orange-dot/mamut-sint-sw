# DSP Core Polish v0.1 Output Safety Evidence

## Summary

This slice adds a compact output-safety pass to the existing `EPM1` DSP core.
It does not change patch schema, controller maps, GUI layout, oscillator/filter
identity, or GFM/BCS control semantics.

The implemented safety path is:

```text
render_frame
  -> master DC blocker
  -> master safety limiter
  -> sanitize / tiny flush
  -> public output buffer and peak snapshot
```

## Changes

- `mamut-dsp` now exposes a shared tiny-value flush threshold and helper.
- `sanitize_sample` clears non-finite and tiny samples before clamping.
- `DcBlocker` flushes tiny internal output state to avoid denormal tails.
- `mamut-dsp` now exposes `master_safety_limit`, neutral below `0.92` and
  bounded at `0.96`.
- `mamut-engine` applies the limiter once after the master DC blocker, covering
  dry core, GFM layer, and BCS layer output together.
- `render-path-math.md` documents the post-render output safety order.

## Validation

Commands run:

```bash
cargo fmt --all
cargo test -p mamut-dsp
cargo test -p mamut-engine
cargo test -p mamut-standalone
cargo fmt --all --check
cargo build --release -p mamut-standalone
```

Observed results:

```text
mamut-dsp:        14 passed; 0 failed
mamut-engine:     50 passed; 0 failed
mamut-standalone: 46 passed; 0 failed
release build:    passed
fmt check:        passed
```

Focused coverage added:

- tiny sample flush in `sanitize_sample`
- DC blocker tiny-tail flush
- master limiter neutrality below the knee
- master limiter finite bounded behavior for extreme inputs
- core dense chord output bounded by the master safety ceiling
- GFM and BCS engine-layer renders bounded by the post-master ceiling

## Boundary

This is an output-safety polish slice, not a tone redesign. It does not retune
GFM/BCS gains or change layer gating. A fresh real direct-output WAV capture was
not run in this session; the previous direct-output follow-up remains the live
artifact reference until the next hardware/audio run.
