# EPM1 SEQ v0.3 — Scenario Library Evidence (SET3-3)

Date: 2026-07-06

Slice: `SET3-3` of `docs/EPM1_BACKLOG_SET3_LAPTOP_MIDI_SEQUENCER.md` — scenario
library v1 (the "replace PC4 for dev" payload).

Evidence class: **synthetic development evidence**. These scenarios run on the
`mamut-seq` virtual port; they do not replace real-rig runs in
`docs/live-sessions/`. Final live-rig failure classification still requires the
physical `PC4 → mioXM → EPM1` path.

## Environment

- Host: Intel Core i7-4600U @ 2.10 GHz (4 threads), 8 GiB RAM
- Kernel: Linux 7.0.14-201.fc44.x86_64 (Fedora); ALSA 1.2.16
- Toolchain: rustc 1.96.0 (stable), edition 2024, MSRV 1.85

## Shipped scenarios

All under `scenarios/` (peer of `patches/` and `profiles/`). Every scenario
addresses controls by profile name through `profiles/pc4-full.toml`; no CC
numbers are hardcoded. `mamut-seq validate` output (event counts, channel 2):

| Scenario                        | Events | Source                                      |
|---------------------------------|-------:|---------------------------------------------|
| `gfm-gate-arm.toml`             |     76 | GFM playbook Pass 1 (SET3-2 reference)      |
| `gfm-mapping-sweep.toml`        |    114 | GFM playbook Pass 2                          |
| `gfm-host-load.toml`            |    866 | GFM playbook Pass 3 (~90 s dense load)      |
| `bcs-layer-basic.toml`          |     31 | SW9 enable, S9 gain, lowest-note follow      |
| `masnoca-listening.toml`        |     39 | single sustained notes + dense chord blocks  |
| `factory-bank-pass.toml`        |    112 | program change 0..7 + phrase per slot        |

(Counts reflect the current code, including a post-review fix that removed the
duplicate aftertouch-envelope seam event; a scenario with an envelope emits two
fewer events than the first SET3-3 cut.)

`mamut-seq validate scenarios/*.toml` reports `valid:` for all six. The unit
test `all_shipped_scenarios_validate_and_expand` parses, validates, and expands
every shipped scenario against the locked profile and asserts each program change
stays within the locked 0..=7 slot set. `cargo test -p mamut-seq`: **18 passed,
0 failed**.

## Schema additions

Two first-class standard-controller steps were added so the playbook's Pass 2/3
gestures ("mod wheel", "sustain") are expressible without hardcoding CC numbers
in scenario files — the same category as the existing `pitch_bend` and
`aftertouch` steps:

- `mod_wheel { value }` → CC1 (honoured by the receiver's fallback map)
- `sustain { down }` → CC64 (honoured by the receiver)

The CC numbers live in `crates/mamut-seq/src/midi.rs`, not in the scenario files;
the locked profile remains the single source of truth for profile-mapped knobs.

## End-to-end play

`bcs-layer-basic.toml` played to completion on the reference host:

```
opened virtual MIDI port `mamut-seq` on channel 2
playing `bcs-layer-basic` (31 events)...
done: 31 events sent
send jitter: max 0.242 ms, mean 0.140 ms
```

On completion the `Drop` guard sent the panic sequence (held-note-offs + sustain
clear + profile/channel-mode resets), leaving no stuck notes.

## Workflow

`tools/run-seq-smoke.sh` builds `mamut-seq`, validates a scenario, and prints the
two-terminal quickstart (Mamut with `--midi-device mamut-seq --trace-midi` in one
terminal, `mamut-seq play` in the other). With `--run` and `--audio-device` it
drives a best-effort headless smoke. v1 stays two-process by design.

`docs/EPM1_GFM_LIVE_BUG_PLAYBOOK.md` gained a "Synthetic `mamut-seq` Rig" section
mapping Pass 1/2/3 to their scenarios and stating the synthetic-vs-hardware
evidence-class rule explicitly.

## Remaining hardware-confirmed step

The audible acceptance — "the gate-arm scenario audibly opens GFM on a real run
and the Mamut-side `status` counters plus `--trace-midi` confirm the expected
control stream" — requires connecting `mamut-seq` to a running Mamut audio
session on a real ALSA device. That is a rig step, recorded separately when
performed. Pass 3's underrun/xrun observations are likewise a hardware measurement.
