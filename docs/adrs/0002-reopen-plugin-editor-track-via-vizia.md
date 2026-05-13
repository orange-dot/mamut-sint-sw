# ADR 0002: Reopen Plugin Editor Track via Vizia

Status: Accepted

## Context

`CLAUDE.md` states: "Plugin/editor work is **intentionally deferred**; the
strongest current proof path is the standalone runtime, not a plugin host."
That posture was correct while the engine, runtime, and standalone surface
were maturing. A plugin host would have multiplied surface area before the
standalone work was load-bearing.

Two facts have changed since that line was written:

- The standalone runtime now ships factory patches, GFM/BCS layers, the PC4
  live rig, and direct-output capture. The engine has been exercised
  end-to-end on real hardware
  (`docs/live-sessions/2026-05-08-cathedral-bloom-*.md`,
  `docs/live-sessions/2026-05-09-molten-horizon-*.md`). Standalone is no
  longer the validating case for the engine.
- Plugin distribution (`VST3` / `CLAP` / `AU`) is on the `EPM1` product
  roadmap. Plugin GUIs have lifecycle, threading, and parameter-binding
  constraints unlike a standalone window. Building the plugin track *after*
  a GUI is finalized would force a second GUI rewrite.

`crates/mamut-standalone` carries a windowed `egui`/`eframe` GUI of roughly
4200 lines across 15 files in `src/gui/`. Its information architecture is
also overdue for redesign (see ADR 0003). The toolkit question is upstream
of the IA question, because some choices (rotary knob density, panel
chrome, plugin-editor lifecycle) are easier to express in some toolkits
than others.

Three Rust GUI toolkits were considered:

- `egui` (current): immediate-mode, mature in production via `eframe`, a
  natural fit for the existing ~75 ms snapshot polling cadence
  (`PERFORMANCE_UI_REFRESH` in
  `crates/mamut-runtime/src/types/constants.rs:22`). Custom instrument
  widgets must be hand-rolled with `egui::Painter`. Plugin-editor support
  via `egui-baseview` exists but is not first-class.
- `iced`: retained-mode, mature, but plugin-editor integration is the same
  uphill story as `egui`. Less oriented toward instrument-style panels.
- `vizia`: retained-mode with declarative composition and CSS-like styling.
  First-class plugin-editor support via `vizia-plug` and `nih_plug_vizia`.
  Ships knob, fader, and meter widgets oriented toward synth and effect
  UIs. Pre-1.0; API churn is a known cost.

`vizia` is the toolkit whose value proposition specifically includes the
plugin-editor track that this ADR reopens. Its cost (rewrite of ~4200 LOC
of GUI code and dependence on a pre-1.0 toolkit) is real but bounded.

## Decision

Reopen the plugin editor track. Adopt `vizia` as the target toolkit for the
new GUI, behind an explicit phased plan with an off-ramp back to `egui`.

Phases:

- **Phase 0**: documentation foundation. This ADR, ADR 0003 for
  information architecture, `docs/ui/vizia-design-system.md`, and the
  `CLAUDE.md` update. No GUI code.
- **Phase 1**: `vizia` spike of the `PERFORM` screen in a new
  `crates/mamut-vizia` workspace member. `crates/mamut-standalone`
  (`egui`) keeps shipping unchanged.
- **Phase 2**: decision gate against the kill criteria below. Pass →
  Phase 3. Fail → ADR 0004 records the outcome; the redesign falls back to
  `egui` using the same IA from ADR 0003.
- **Phase 3**: port `SOUND`, `SYSTEM`, and `INSPECT` to `vizia`. Final
  cutover removes `crates/mamut-standalone/src/gui/` and re-targets
  `mamut-standalone` to launch `vizia` by default.
- **Phase 4**: wrap the `vizia` GUI as a `nih_plug_vizia` (or `vizia-plug`)
  editor. Ship `VST3` and `CLAP` targets.

Phase 1 kill criteria (all must pass at the Phase 2 gate):

1. Rotary knob (custom `vizia` view) usable with mouse drag, scroll wheel,
   and numeric value entry.
2. Master oscilloscope (custom view, 8192-frame ring buffer) sustains
   ≥30 fps without main-thread stalls. Scope frames are consumed via the
   existing direct `&mut Consumer` (rtrb) path, not through
   `SessionStateSource`; the snapshot-polling surface and the scope path
   stay decoupled.
3. Snapshot poll → reactive update path produces no visible tear or
   flicker.
4. `vizia` LOC for the spike `PERFORM` screen is ≤2× the `egui`
   equivalent, as a rough proxy for full-port cost.
5. The spike interacts with the engine end-to-end: macros respond to live
   MIDI input; `PANIC` stops voices; the slot grid switches patches.

The `vizia` work lives in a separate workspace binary `crates/mamut-vizia`,
not a feature flag on `crates/mamut-standalone`. The `CLAUDE.md`
"Things to avoid" rule forbids adding a workspace member without an
explicit task; this ADR is the explicit task. The two toolkits coexist
only until the Phase 3 cutover.

State plumbing introduces a new `SessionStateSource` trait in
`crates/mamut-runtime/src/session/state_source.rs`, re-exported through
`crates/mamut-runtime/src/session/mod.rs`. The trait abstracts the read
surface that `vizia` view code consumes; it covers `EngineSnapshot`
polling and references to the published metrics counters
(`TransportMetrics`, `InputMetrics`, `RecordingMetrics`). Three
properties of the trait are load-bearing:

- The standalone implementation is a thin wrapper around
  `Session::request_snapshot()`. The existing surface is a 500 ms
  blocking `mpsc` request/reply (see
  `crates/mamut-runtime/src/session/status.rs:122`); the wrapper does
  not change that contract. The GUI poll cadence remains
  `PERFORMANCE_UI_REFRESH = 75 ms` from
  `crates/mamut-runtime/src/types/constants.rs:22`.
- Scope-frame access is **not** part of the trait. The render-time
  scope ring (rtrb `Consumer`) requires `&mut` and is drained by an
  owner-thread loop, not through a `&self` trait surface. The Phase 1
  spike consumes scope frames via the existing direct `&mut Consumer`
  path (see `crates/mamut-runtime/src/session/runtime.rs:243`),
  decoupled from the snapshot-polling surface.
- A push-style plugin implementation (engine writes into an
  `Arc<AtomicSnapshot>` consumed by the editor) does not exist yet. It
  is a Phase 4 prerequisite, not a Phase 1 deliverable. Phase 1 ships
  only the standalone wrapper; Phase 4 adds the plugin implementation
  and validates that the trait shape survived contact with a plugin
  host.

The `EPM1` transport freeze in `docs/EPM1_TRANSPORT_FREEZE.md` remains in
effect. This ADR does not reopen it. The `SessionStateSource` trait is a
*read* surface over already-published metrics and snapshots; it does not
reshape the transport boundary, the audio callback, the MIDI ingress
callback, or the voice allocator. The `panic` and `reset_controllers`
priority action path established in ADR 0001 is unchanged.

`mamut-tui` is unaffected by this ADR.

## Consequences

The workspace carries two GUI toolkits in parallel during Phases 1 through
3, estimated at ~9–12 weeks. This costs build-graph weight (`winit`,
`wgpu`, `femtovg` from `vizia` alongside existing `eframe`/`egui`), CI
time, and concentrated review attention. The mitigation is the separate
crate, separate binary, and no feature-flag entanglement.

`vizia` is pre-1.0. The dependency is pinned by git rev, not by version
range. Per-quarter time is budgeted to track upstream. If a breaking
`vizia` change lands during Phase 3 that cannot be tracked within budget,
the off-ramp from ADR 0003 applies and the redesign continues on `egui`.

Custom instrument widgets dominate port cost. The Phase 0 design spec
sketches the highest-risk widget (the `mamut-field` GFM lattice paint) so
its render strategy (`femtovg::Canvas::fill_path` or a `wgpu` fallback)
is settled before Phase 3 starts.

If the Phase 2 gate fails, the GUI redesign continues on `egui` per ADR
0003. ADR 0004 records the gate outcome (pass or fail) so the trail is
auditable.

The `mamut-platform` shared Linux audio platform track and the sibling
`mamit-sint-hw` hardware repo are unaffected. No platform-track invariants
are crossed by this work.

## Amendment 2026-05-13: command-sink doctrine

Phase 1 sub-step 2 (PERFORM widgets) surfaced an asymmetry in the
state-plumbing section above. The `SessionStateSource` trait covers the
*read* surface (snapshot polling, metrics counter references) but says
nothing about the GUI's *write* surface. Phase 1 kill criterion #5
requires three interactive paths:

- `PANIC` button must stop voices.
- The slot grid must switch patches.
- The rotary-knob spike for master output trim (kill criterion #1)
  must mutate a parameter to be exercised as anything more than a
  read-only display.

A second trait, `SessionCommandSink`, is introduced in
`crates/mamut-runtime/src/session/state_source.rs` (same file as
`SessionStateSource`; the two surfaces are siblings, not layered).
Minimum methods for Phase 1 kill criteria:

- `panic()` — fire-and-forget through the existing
  `Arc<PriorityActions>` path established in ADR 0001.
- `reset_controllers()` — same priority-action path.
- `set_macro(id: MacroId, value: f32)` — sends
  `EngineCommand::Controller(ControllerEvent::Macro { … })` through
  the existing `mpsc::Sender<EngineCommand>`. Clamps to `[0.0, 1.0]`
  as `RuntimeSession::set_macro` already does.
- `set_direct_param(id: ParamId, value: f32)` — sends
  `EngineCommand::Controller(ControllerEvent::DirectParam { … })`
  through the same channel. Clamps to the parameter's declared range
  as `RuntimeSession::set_direct_param` already does. Phase 1 only
  needs `output_trim_db` for the rotary spike; the surface is generic
  for symmetry with the standalone CLI.

Slot switching is **deliberately not** a command-sink method.
`RuntimeSession::switch_patch(path)` takes `&mut self` because it
rebuilds the audio runtime, replaces the worker thread, and reinstalls
the ALSA stream. Exposing that through a `&self` GUI trait would
require either interior mutability of the session (which crosses the
transport-freeze posture in `docs/EPM1_TRANSPORT_FREEZE.md`) or a new
runtime-control-message variant (an architecture change, not a Phase 1
delivery). Phase 1 instead routes slot-grid clicks as an
`AppEvent::RequestSlotSwitch(slot: u8)` consumed by `main.rs` of
`crates/mamut-vizia`, which holds `&mut RuntimeSession` and dispatches
`switch_patch` directly. Phase 4 (plugin) will revisit the
slot-switching surface; a runtime-control-message variant is the
expected design but is not in scope here.

Implementations parallel `SessionStateSource`:

- `StandaloneCommandSink<'a>` — borrows `&RuntimeSession`. Suitable
  for short synchronous CLI or TUI use; cannot be captured by a
  `'static` GUI handler.
- `StandaloneCommandHandle` — owned `'static`, clones
  `mpsc::Sender<EngineCommand>` and `Arc<PriorityActions>`. Suitable
  for capture into Vizia widget callbacks and timer closures.

`SessionCommandSink` carries no `Send`/`Sync` bound, matching
`SessionStateSource`. The owned `StandaloneCommandHandle` inherits
`Send` from `mpsc::Sender + Arc`; it is **not** `Sync` because of
`mpsc::Sender`'s single-producer-cell. The Vizia Phase 1 spike runs
all widget callbacks on the UI thread, so `Sync` is not load-bearing.

The `EPM1` transport freeze remains in effect. `SessionCommandSink`
adds no new channels, no new queues, and no new worker threads — it
exposes the *existing* command paths already used by
`RuntimeSession::panic`, `RuntimeSession::reset_controllers`,
`RuntimeSession::set_macro`, and `RuntimeSession::set_direct_param`.
The GUI's `SessionCommandSink` trait is a thin doctrine layer over
already-shipped command surfaces; it does not reshape the transport
boundary, the audio callback, the MIDI ingress callback, or the voice
allocator.

The push-style plugin implementation deferred above (the
`Arc<AtomicSnapshot>` consumer) is unchanged. Phase 4 will add both a
`PluginSessionSource` and a `PluginCommandSink` impl in parallel; the
trait pair is the surface the plugin editor binds against.
