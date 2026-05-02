# GFM v1.7 Smoothed Momentary Color Evidence

Date: 2026-05-01

This slice softens the live transition from the base synth color into GFM color.
The previous v1.6 behavior made K8/aftertouch functionally correct, but the
audible GFM layer could enter too abruptly during performance.

## Control Contract

- `K8 GFM Amount` still sets maximum GFM depth.
- Channel aftertouch still opens the momentary GFM gate.
- Effective target is still `GFM Amount * aftertouch_gate`.
- The audible GFM amount is now audio-rate smoothed toward that target.
- GFM live pressure excitation is also smoothed, so color/rupture pressure does
  not jump instantly when aftertouch crosses the deadzone.
- K8 returning to `0.0` still hard-targets the GFM layer to silence, but an
  auto-armed layer fades out before it fully disarms.

## Runtime Notes

- The smoothing lives in `mamut-engine`, not the GUI.
- No patch schema, factory patch, controller profile, MIDI mapping, ALSA, or
  standalone transport changes were required.
- Snapshot `gfm_layer.effective_amount` now reflects the smoothed audible amount
  rather than only the instantaneous target.

## Validation

Commands run:

- `cargo fmt --all`
- `cargo fmt --all --check`
- `cargo test -p mamut-engine gfm_layer_momentary_gate_smooths_in_and_out -- --nocapture`
- `cargo test -p mamut-engine gfm_layer_amount_zero -- --nocapture`
- `cargo test -p mamut-engine gfm_layer_ -- --nocapture`
- `cargo test -p mamut-standalone`
- `cargo build --release -p mamut-standalone`

Results:

- `cargo test -p mamut-engine gfm_layer_momentary_gate_smooths_in_and_out -- --nocapture`
  - 1 passed
- `cargo test -p mamut-engine gfm_layer_amount_zero -- --nocapture`
  - 3 passed
- `cargo test -p mamut-engine gfm_layer_ -- --nocapture`
  - 15 passed
- `cargo test -p mamut-standalone`
  - 43 passed
- `cargo build --release -p mamut-standalone`
  - passed

## Expected Live Behavior

- Set K8 above zero.
- Press aftertouch.
- GFM should bloom into the sound instead of switching in sharply.
- Release aftertouch or return K8 to zero.
- GFM should fade back out cleanly.
