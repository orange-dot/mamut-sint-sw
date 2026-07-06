# EPM1 SEQ v0.2 — Scenario Player Evidence (SET3-2)

Date: 2026-07-06

Slice: `SET3-2` of `docs/EPM1_BACKLOG_SET3_LAPTOP_MIDI_SEQUENCER.md` — scenario
schema plus deterministic player.

Evidence class: **synthetic development evidence**.

## Environment

- Host: Intel Core i7-4600U @ 2.10 GHz (4 threads), 8 GiB RAM
- Kernel: Linux 7.0.14-201.fc44.x86_64 (Fedora); ALSA 1.2.16
- Toolchain: rustc 1.96.0 (stable), edition 2024, MSRV 1.85

## Schema (v1)

TOML, `#[serde(deny_unknown_fields)]`, `schema_version` checked first (template =
`crates/mamut-patch`). Header: `schema_version`, `name`, optional `description`,
`channel`, `seed` (reserved; v1 expansion is deterministic). Timeline steps:
`wait`, `note`, `note_on`, `note_off`, `chord`, `cc`, `cc_ramp`, `aftertouch`,
`aftertouch_envelope`, `pitch_bend`, `program_change`, `loop`, `section`.

CC / aftertouch / gate controls are addressed by **profile control name**
(`"K8 GFM Gate"`, `"S9"`), resolved through `profiles/pc4-full.toml` at load time.
Both the full name and the leading token (`"K8"`) resolve. Unknown names are a
hard error naming the profile.

Timing model: only `wait`, `cc_ramp`, and `aftertouch_envelope` advance the
timeline cursor; all other steps are instantaneous triggers. `note`/`chord`
schedule their own note-off `duration_ms` later without moving the cursor.

## Determinism

Schedule expansion is a pure function `expand(scenario, profile, channel) →
Vec<AbsoluteEvent>`: same inputs yield a byte-identical, time-sorted event list.
This is exactly what `validate` prints. Covered by unit tests:

- `expand_is_deterministic` — two expansions of the same scenario are equal
- `reference_scenario_expands_against_locked_profile` — same, against the locked
  `profiles/pc4-full.toml`
- `note_off_pairing_and_timing`, `cc_value_clamps`, `loop_expands_count_times`,
  `unknown_control_is_a_hard_error`, `rejects_wrong_schema_version`,
  `rejects_program_out_of_range`, `rejects_empty_timeline`

`cargo test -p mamut-seq`: **16 passed, 0 failed**.

## Reference scenario

`scenarios/gfm-gate-arm.toml` (GFM live-bug playbook Pass 1) expands to 76
events. `validate` head, confirming name resolution (`K8 GFM Gate → CC3`) on
channel 2:

```
valid: scenarios/gfm-gate-arm.toml (gfm-gate-arm, 76 events, channel 2)
        0 ms  C1 01        program_change ch2 program=1
      250 ms  91 2D 64     note_on ch2 note=45 vel=100
      250 ms  91 34 64     note_on ch2 note=52 vel=100
      250 ms  91 39 64     note_on ch2 note=57 vel=100
      750 ms  B1 03 00     cc ch2 K8 GFM Gate(cc3)=0
      800 ms  B1 03 08     cc ch2 K8 GFM Gate(cc3)=8
      ...
```

Deterministic event **content** (bytes and order) is guaranteed; wall-clock
timing is measured, not claimed away.

## Measured send jitter (reference host)

Player uses monotonic absolute-deadline scheduling from a single `Instant` (no
cumulative drift). Jitter = `actual_send_time − deadline`, reported at exit. A
20-iteration probe (60 events: note on/off + CC + 50 ms waits, ~1 s total) on the
host above:

```
send jitter: max 0.245 ms, mean 0.140 ms
```

Userspace scheduling will jitter more under load and on other hosts; any
evidence-grade timing claim must carry its measured jitter for the host that
produced it.

## Boundary compliance

Pure MIDI source; no runtime changes; clippy clean on `--all-targets`; no new
external dependencies.
