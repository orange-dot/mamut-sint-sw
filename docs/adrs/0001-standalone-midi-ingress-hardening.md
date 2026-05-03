# ADR 0001: Standalone MIDI Ingress Hardening

Status: Accepted

## Context

`mamut-sint-sw` is under the local `EPM1` transport freeze. The standalone
runtime is still allowed to take narrow correctness fixes, but it must not
become the place where the shared Linux audio transport architecture evolves.

The live 96 kHz work exposed a smaller local problem: the MIDI callback was
doing optional evidence work before publishing runtime events, queue push
failures were silent, and dense continuous controller bursts could force more
render-thread work than one block needs.

## Decision

Keep this as a narrow `mamut-standalone` bugfix:

- MIDI ingress uses bounded `crossbeam_queue::ArrayQueue` instances only.
- The MIDI observer/trace queue capacity is `4096` fixed-size records.
- Raw trace records are fixed-size and contain only compact MIDI bytes,
  timing, parsed verdict, and compact overlay state; no `String` or `Vec`.
- The session owns one trace worker. The MIDI callback publishes records; the
  worker formats lines, writes the sidecar, updates latest-control UI state,
  and drains on shutdown before writing the footer.
- `panic` and `reset_controllers` from MIDI bypass the normal runtime-control
  queue and set the existing `PriorityActions` atomics directly.
- Drop counters are telemetry only and use relaxed atomics:
  `midi_messages_dropped`, `runtime_controls_dropped`,
  `trace_records_dropped`, and `controllers_coalesced`.
- `midi_messages` keeps its existing activity-counter role. A separate
  `midi_messages_accepted` counter tracks successful publication to runtime
  or priority action paths.
- Continuous controls are coalesced once per engine render block: pitch bend,
  mod wheel, channel aftertouch, macro by id, direct param by id, GFM amount,
  and BCS amount.
- Note on/off, sustain, and BCS enabled remain FIFO/event-preserving.

`ControllerEvent` remains channel-agnostic. Multi-channel isolation remains an
operator/runtime selection concern through `--midi-channel`; this bugfix does
not widen the engine API.

Polyphonic aftertouch is not a runtime event today. MIDI system real-time
messages do not enter the runtime queue; when tracing is active they are kept
as compact raw trace records.

## Consequences

The MIDI callback no longer performs file I/O, `stderr` logging, trace string
formatting, or latest-control string construction. Evidence remains available,
but evidence loss is visible through `trace_records_dropped`.

Coalescing trades intermediate continuous-controller values inside one render
block for lower render pressure. Larger render blocks therefore lose more
intermediate values; operators should watch `controllers_coalesced` when
tuning period and buffer sizes.

If a 96 kHz arpeggio pass without recording still shows growing ALSA xruns,
MIDI drops, runtime-control drops, or trace drops in normal no-trace mode,
the local freeze should not be bypassed with another queue redesign. The next
decision belongs in the shared Linux audio platform transport track.
