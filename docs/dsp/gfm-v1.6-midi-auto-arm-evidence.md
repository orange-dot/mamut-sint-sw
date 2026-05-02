# GFM v1.6 MIDI Auto-Arm Evidence

Date: 2026-05-01

This slice adjusts the v1.5 momentary-control contract after live GUI/MIDI
testing. `GFM Disabled` no longer means that MIDI performance controls are
dead until the performer manually clicks the GUI enable checkbox. A nonzero
`K8 GFM Amount` event can auto-arm the GFM layer with the default seed inside
the running engine. No session restart is required.

## Control Contract

- GUI enable still works and still clears stale GFM `K8` and aftertouch values.
- `K8 GFM Amount` / CC `3` above zero auto-arms GFM when the engine is disabled.
- Auto-arm uses the shared default seed `0x6A464D40`.
- Auto-arm preserves the incoming K8 amount; it does not clear the live control
  that caused the arm.
- If the layer was auto-armed by MIDI, returning K8 to `0.0` disarms it again.
- If the layer was manually enabled in the GUI, returning K8 to `0.0` keeps the
  GFM voice ready but forces audible amount to `0.0`.
- Channel aftertouch opens the momentary gate only when K8 amount is above zero.
- With K8 at `0.0`, aftertouch does not bring GFM into the mix.
- With aftertouch at `0.0`, K8 can arm the voice but the audible amount remains
  `0.0`.
- Tiny K8/aftertouch noise below the GFM control deadzone is treated as `0.0`.
- When the final held note is released, the GFM-specific aftertouch gate is
  cleared to avoid a stale momentary gate on the next phrase.
- Effective audible amount remains:
  - `GFM Amount * aftertouch_gate`

This keeps the earlier protection against stale GUI enable color changes, while
allowing a player to leave the GUI unchecked and bring GFM in from the keyboard
controls.

## Runtime Notes

- `mamut-engine` now exposes `DEFAULT_GFM_LAYER_SEED`.
- The engine auto-arms from realtime `GfmLayerAmount` and from aftertouch if a
  nonzero GFM amount is already stored.
- The engine tracks whether the current enabled state came from MIDI auto-arm or
  from explicit GUI enable.
- The standalone GUI syncs its GFM checkbox state from engine snapshots, so an
  auto-armed layer becomes visible in the panel after the next UI refresh.
- No patch schema, factory patch, ALSA, or MIDI profile changes were required.

## Validation

Commands run:

- `cargo fmt --all`
- `cargo fmt --all --check`
- `cargo test -p mamut-engine gfm_layer_amount_zero -- --nocapture`
- `cargo test -p mamut-engine gfm_layer_midi_controls_can_reintroduce_layer_without_gui_enable -- --nocapture`
- `cargo test -p mamut-engine gfm_layer_ -- --nocapture`
- `cargo test -p mamut-standalone gfm -- --nocapture`
- `cargo test -p mamut-standalone`
- `cargo build --release -p mamut-standalone`

Results:

- `cargo test -p mamut-engine gfm_layer_ -- --nocapture`
  - 14 passed
  - includes `gfm_layer_amount_auto_arms_default_seed_without_pressure`
  - includes `gfm_layer_midi_controls_can_reintroduce_layer_without_gui_enable`
- `cargo test -p mamut-engine gfm_layer_amount_zero -- --nocapture`
  - 3 passed
  - includes `gfm_layer_amount_zero_disarms_auto_armed_layer`
  - includes `gfm_layer_amount_zero_keeps_manual_gui_enabled_layer_ready`
- `cargo test -p mamut-engine gfm_layer_midi_controls_can_reintroduce_layer_without_gui_enable -- --nocapture`
  - 1 passed
- `cargo test -p mamut-standalone gfm -- --nocapture`
  - 8 passed
- `cargo test -p mamut-standalone`
  - 43 passed
- `cargo build --release -p mamut-standalone`
  - passed

## Expected Live Behavior

- Start with the GUI GFM checkbox off.
- Move K8 above zero.
- Hold a note/chord.
- Press aftertouch to bring GFM in momentarily.
- Release aftertouch to return to the base synth.
- Turn K8 back to zero for a hard GFM mute and auto-disarm.

After K8 auto-arms the layer, the GUI should show GFM as enabled/ready on the
next snapshot refresh. That is state visibility, not an always-audible layer.
