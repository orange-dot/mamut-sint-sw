---
name: scenario-profile-name-contract
description: mamut-seq scenarios address controls by profile name (never raw CC); mod_wheel/sustain are schema step-kinds with the CC living in code, not scenario files
metadata:
  type: project
---

`mamut-seq` scenario contract (`scenarios/*.toml`, schema in
`crates/mamut-seq/src/scenario/model.rs`):

- CC/ramp/gate controls are addressed by **profile control name**
  (`"K8 GFM Gate"` or leading token `"K8"`), resolved through
  `profiles/pc4-full.toml` at load time. Unknown names are a hard error naming the
  profile. `profiles/pc4-full.toml` stays the single source of truth for CC numbers.
- `program_change` is validated to `0..=7` (the locked 8-slot live set). Out of
  range is a hard `ProgramOutOfRange` error.
- `mod_wheel` (CC1) and `sustain` (CC64) are first-class schema STEP KINDS, not
  raw CC in scenario files. Their CC numbers live in `crates/mamut-seq/src/midi.rs`
  (`CC_MOD_WHEEL`, `CC_SUSTAIN`) — same category as `pitch_bend`/`aftertouch`. This
  is the honest reconciliation of "no hardcoded CC in scenarios": standard
  performance controllers are named gestures, the locked profile still owns the
  mapped knobs.
- Test `all_shipped_scenarios_validate_and_expand` asserts each scenario's `name`
  field EQUALS its file basename (e.g. `gfm-gate-arm.toml` -> name = "gfm-gate-arm").

**How to apply:** when a scenario or the profile changes, verify control names
still resolve, program numbers stay 0..7, and no raw `cc = <n>` leaks into a
scenario file. New standard controllers belong as schema step-kinds, not literals.
