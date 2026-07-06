# EPM1 SEQ v0.1 — Virtual MIDI Port Evidence (SET3-1)

Date: 2026-07-06

Slice: `SET3-1` of `docs/EPM1_BACKLOG_SET3_LAPTOP_MIDI_SEQUENCER.md` — crate
scaffold plus virtual MIDI output port.

Evidence class: **synthetic development evidence**. `mamut-seq` runs on a virtual
ALSA port, not the PC4 rig. Real-rig classification still requires hardware; this
document does not replace `docs/live-sessions/`.

## Environment

- Host: Intel Core i7-4600U @ 2.10 GHz (4 threads), 8 GiB RAM
- Kernel: Linux 7.0.14-201.fc44.x86_64 (Fedora)
- ALSA: 1.2.16
- Toolchain: rustc 1.96.0 (stable), workspace edition 2024, MSRV 1.85
- Note: ALSA sequencer access was available on this host. Sandboxed or headless
  environments may restrict virtual-port creation; record the actual environment
  wherever a run is reproduced.

## What was verified

Port enumeration (`mamut-seq ports`) lists the system MIDI ports without error:

```
MIDI input ports:
  0: Midi Through:Midi Through Port-0 14:0
  1: AG06/AG03:AG06/AG03 MIDI 1 28:0
MIDI output ports:
  0: Midi Through:Midi Through Port-0 14:0
  1: AG06/AG03:AG06/AG03 MIDI 1 28:0
```

Virtual output port opens and is visible to other MIDI clients while the tool
runs. With `mamut-seq play` holding the port open, both `mamut-seq ports` and the
Mamut receiver's own enumeration list it:

```
# mamut-standalone list-midi (while mamut-seq play is running)
2: mamut-seq:mamut-seq 128:0
```

This satisfies the SET3-1 acceptance line: *"`cargo run --locked -p
mamut-standalone -- list-midi` enumerates the `mamut-seq` port while the tool
runs."* The port is created through `midir`'s `os::unix::VirtualOutput` trait; no
cargo feature or workspace-dependency change was required (`midir = "0.10.3"` is
unchanged). Name collisions suffix the port with the process id and retry once.

## Panic hygiene

`mamut-seq` is the first MIDI **output** path in the repo, so there was no
emitter to copy. The panic sequence was designed against the receiver's actual
decode behaviour (`crates/mamut-runtime/src/audio_runtime/midi_parse.rs`), which
ignores CC 120/121/123. The effective panic therefore is:

1. explicit `Note Off` for every tracked held note (what actually silences Mamut)
2. `CC64 = 0` sustain clear (honoured by the receiver)
3. the profile's own panic/reset controls, resolved from `profiles/pc4-full.toml`
   by `action` (`SW1 → CC80`, `SW2 → CC81`), fired at value `127`
4. conventional `CC123/120/121 = 0` — documented no-ops against Mamut, emitted
   for any other well-behaved receiver

A `Drop` guard sends this sequence on normal exit and on `?`/panic unwind. A hard
kill (SIGKILL/SIGTERM) bypasses `Drop`; that gap is documented, and interactive
`Ctrl-C`/`q` during `play`/`live` is handled through `crossterm` raw-mode key
capture (no `ctrlc` crate, no `unsafe`).

## Boundary compliance

- No changes to `mamut-runtime`, the transport boundary, or MIDI ingress —
  transport freeze respected by construction.
- `mamut-seq` depends only on external crates already in
  `[workspace.dependencies]`; no dependency on any other mamut crate.
- Workspace lint posture unchanged (`unsafe_code = "forbid"`; clippy clean on
  `--all-targets`).

## Remaining hardware-confirmed step

Audible voice triggering ("a smoke note sequence … audibly triggers voices")
requires connecting the `mamut-seq` port to a running Mamut audio session on a
real ALSA output device and confirming with `--trace-midi` plus `status`
counters. That is a rig step, recorded separately when performed.
