# GFM v1.8 Swapped K8/Aftertouch Controls Evidence

Date: 2026-05-01

This slice swaps the functional roles of K8 and channel aftertouch in the live
GFM path, while keeping the same physical MIDI inputs.

## Control Contract

- K8 / CC `3` is now the GFM gate/morph control.
- Channel aftertouch is now the GFM amount/depth control.
- The effective audible target is:
  - `aftertouch_amount * K8_gate`
- K8 still drives the smoothed GFM pressure/color excitation.
- Aftertouch still drives the base synth's existing aftertouch macro response.
- GFM only auto-arms when both controls are nonzero.
- Returning K8 to `0.0` fades the GFM layer out and disarms an auto-armed layer
  after the release ramp.
- Releasing the final held note clears the GFM aftertouch amount to avoid stale
  depth on the next phrase.

## Runtime Notes

- The public event name `GfmLayerAmount` is unchanged for compatibility with the
  existing MIDI profile parser.
- The PC4 profile label is now `K8 GFM Gate`.
- The GUI MIDI tile now shows `K8 gate <value> AT amt <value>`.
- No patch schema, factory patch, ALSA, or standalone transport changes were
  required.

## Validation

Commands run:

- `cargo fmt --all`
- `cargo fmt --all --check`
- `cargo test -p mamut-engine gfm_layer_gate -- --nocapture`
- `cargo test -p mamut-engine gfm_layer_ -- --nocapture`
- `cargo test -p mamut-standalone`
- `cargo build --release -p mamut-standalone`

Results:

- `cargo test -p mamut-engine gfm_layer_gate -- --nocapture`
  - 2 passed
- `cargo test -p mamut-engine gfm_layer_ -- --nocapture`
  - 15 passed
- `cargo test -p mamut-standalone`
  - 43 passed
- `cargo build --release -p mamut-standalone`
  - passed

## Expected Live Behavior

- Press aftertouch to set how much GFM is available.
- Move K8 up to morph GFM into the sound.
- Move K8 down to fade GFM out.
- Release aftertouch or release the phrase to clear GFM amount/depth.
