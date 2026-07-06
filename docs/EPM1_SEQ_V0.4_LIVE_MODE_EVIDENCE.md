# EPM1 SEQ v0.4 — Live TUI Mode Evidence (SET3-4)

Date: 2026-07-06

Slice: `SET3-4` of `docs/EPM1_BACKLOG_SET3_LAPTOP_MIDI_SEQUENCER.md` — interactive
live TUI mode.

Evidence class: **synthetic development evidence**. `mamut-seq live` replaces the
*dev-time* role of the PC4 keybed, not its stage role.

## Environment

- Host: Intel Core i7-4600U @ 2.10 GHz (4 threads), 8 GiB RAM
- Kernel: Linux 7.0.14-201.fc44.x86_64 (Fedora); ALSA 1.2.16
- Toolchain: rustc 1.96.0 (stable), edition 2024, MSRV 1.85

## Feature surface (`mamut-seq live [scenario-to-fire]`)

- Computer-keyboard piano, two-row one-octave layout: white keys `z x c v b n m ,`,
  black keys `s d  g h j`. Octave shift `-` / `=`.
- Sustain toggle (Space), pitch-bend nudge (Up/Down, `0` centre), program change
  `1..8` → `0..7`, panic (Backspace).
- Channel-aftertouch lane driven by ramp keys (`w` up / `q` down / `e` hold),
  since computer keyboards have no pressure — enough to exercise the GFM
  aftertouch gate contract by hand.
- CC lane: select a named control from the profile (Tab / Shift-Tab), adjust with
  Left/Right, set min/max with PageDown/PageUp. Drives e.g. `K8 GFM Gate`.
- Fire-scenario-from-live: `f` fires a preloaded scenario into the live stream
  (non-blocking — its events are scheduled alongside live input in the tick loop).

## Terminal key-release capability (stated honestly)

Plain terminals deliver no key-release events. The note model is therefore
`gate` (fixed gate length, default 150 ms, `--gate-ms`) plus a `latch` toggle
mode; `Insert` cycles them. Where the terminal supports the kitty keyboard
protocol, `crossterm` keyboard-enhancement flags (`REPORT_EVENT_TYPES`) are
pushed and true `press/release` is used instead. The detected capability and the
active note mode are shown in the UI ("release events: available (kitty) /
unavailable") and default accordingly. No claim of piano-grade playability is
made either way.

Under the plain pty used for the smoke below, keyboard enhancement was
unavailable, so the tool defaulted to `gate` mode — the honest fallback.

## Verification

- Non-interactive guard: `mamut-seq live` with a non-TTY stdin exits with
  `error: live mode requires an interactive terminal`.
- pty-driven launch smoke (spawn under a pseudo-terminal, play a note, select a
  CC, program change, then quit with Ctrl-C):

  ```
  exit_code: 0
  port_visible_during_live: True
  exit_status: clean
  ```

  The virtual `mamut-seq` port is visible to other clients while `live` runs; the
  terminal is restored on exit (RAII guard) and the port's `Drop` guard sends the
  panic sequence, so quit always leaves the instrument clean.
- Input is event-driven: `event::poll(25 ms)` blocks between events and the screen
  is redrawn only when state changes (dirty flag) — no busy-loop CPU burn.
- Unit tests (no terminal/ALSA needed) cover the keymap and note-model logic:
  `note_offsets_cover_the_two_row_octave`, `note_of_tracks_the_octave`,
  `note_mode_defaults_and_cycles_by_capability`,
  `control_action_maps_program_and_panic`. `cargo test -p mamut-seq`: **22 passed,
  0 failed** at SET3-4. A subsequent post-review hardening pass (best-effort
  panic, broken-pipe-safe output, non-finite/ramp-step/expansion-budget guards,
  unknown-step-field rejection, envelope-seam dedup, live guard/timing fixes)
  added scenario-schema tests, bringing the suite to **27 passed, 0 failed**.

## Remaining hardware-confirmed step

The audible acceptance — "hold a chord, open GFM with the aftertouch ramp plus
`K8`, and hear the layer bloom — laptop only, no hardware attached" — requires a
running Mamut audio session on a real ALSA device with the `mamut-seq` port
selected as input. That is an operator step at the desk, recorded when performed;
panic-clean behaviour is additionally confirmed there via `--trace-midi` and
Mamut `status`.
