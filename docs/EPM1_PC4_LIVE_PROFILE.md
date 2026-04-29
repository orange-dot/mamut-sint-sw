# EPM1 PC4 Live Profile

## Summary

This document locks the first full live-controller profile for `EPM1`:

- controller: `Kurzweil PC4`
- profile file: `profiles/pc4-full.toml`
- transport path: `PC4 -> mioXM -> EPM1 -> audio interface`
- performance posture: standalone-first live rig
- source capture: `synthetic-pc4-capture/real-captures/2026-04-26-pc4-take2-001`
- mod wheel supplement: `synthetic-pc4-capture/real-captures/2026-04-26-pc4-mod-wheel-001`

Current implementation status:

- `mamut-standalone play --controller-profile <path>` loads TOML controller bindings
- `tools/run-pc4-ag03.sh` loads `profiles/pc4-full.toml` by default
- windowed mode exposes `Live`, `PC4`, and `Debug` tabs
- the `PC4` tab is read-only: PC4 changes Mamut; GUI displays Mamut state
- no-profile mode keeps the old legacy fallback: `CC16..20` map to the five public macros
- standard expressive lanes remain live: notes, pitch bend, `CC1` mod wheel, `CC64` sustain, channel aftertouch
- `program change 0..7` selects the locked live-set slots

## Locked PC4 Map

Knobs:

- `K1` / `CC71` -> macro `Gravitacija`
- `K2` / `CC73` -> macro `Bloom`
- `K3` / `CC72` -> direct `amp_env_attack_ms` with log scaling
- `K4` / `CC79` -> direct `amp_env_release_ms` with log scaling
- `K5` / `CC14` -> macro `Swarm`
- `K6` / `CC17` -> macro `Heat`
- `K7` / `CC18` -> macro `Ruin`
- `K8` / `CC3` -> reserved no-op, trace-visible
- `K9` / `CC9` -> direct `reverb_mix`

Sliders:

- `S1` / `CC12` -> `osc1_saw_level`
- `S2` / `CC13` -> `osc1_pulse_level`
- `S3` / `CC22` -> `osc2_saw_level`
- `S4` / `CC23` -> `sub_level`
- `S5` / `CC24` -> `mixer_pre_filter_drive`
- `S6` / `CC25` -> `mixer_body_mix`
- `S7` / `CC26` -> `filter_cutoff_hz` with log scaling
- `S8` / `CC27` -> `chorus_mix`
- `S9` / `CC28` -> `final_stage_body_drive`

Switches:

- `SW1` / `CC80` -> `panic`
- `SW2` / `CC81` -> `reset_controllers`
- `SW3` / `CC82` -> previous favorite
- `SW4` / `CC83` -> next favorite
- `SW5` / `CC85` -> toggle `chorus_enabled`
- `SW6` / `CC86` -> toggle `reverb_enabled`
- `SW7` / `CC87` -> favorite slot `0`
- `SW8` / `CC89` -> favorite slot `4`
- `SW9` / `CC90` -> reserved no-op, trace-visible

## Standard Performance Lanes

- notes -> synth note on/off
- `CC1` -> mod wheel expressive lane
- `CC64` -> sustain
- channel pressure -> channel aftertouch expressive lane
- pitch bend -> standard 14-bit bend, patch bend range
- `program change 0..7` -> live slots:
  `molten-horizon`, `cathedral-bloom`, `ember-vault`, `razor-thaw`,
  `gravity-wake`, `furnace-choir`, `granite-plain`, `glass-tide`

## Policy

- The profile is explicit TOML, not hidden hard-coded controller policy.
- MIDI direction is one-way: `PC4 -> Mamut`. Mamut does not send MIDI, CC feedback,
  sysex, template updates, or state sync back to the PC4.
- Switches use a hybrid model: runtime actions, toggles, and reserved no-op slots.
- Delay control is reserved because the current engine has no delay block.
- Bank select remains ignored until multiple setlists are needed.
- Sprint 6 remains single-channel; use `--midi-channel <1..16>` to filter noisy ports.
- Patch switching keeps the hard reset model: notes/controllers are cleared before the new patch becomes authoritative.
- The runtime UI shows internal synth state, not assumed physical controller position.

## Smoke Expectations

Minimum software checks:

```bash
cargo test --locked
cargo run --locked -p mamut-standalone -- list-factory
cargo run --locked -p mamut-standalone -- dry-run molten-horizon
```

Minimum real-rig launch path:

```bash
tools/run-pc4-ag03.sh --trace-midi molten-horizon
```
