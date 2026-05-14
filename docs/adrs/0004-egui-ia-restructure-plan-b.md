# ADR 0004: Restructure GUI Information Architecture In Place on egui (Plan B)

Status: Accepted

Date: 2026-05-13

## Context

ADR 0002 (`docs/adrs/0002-reopen-plugin-editor-track-via-vizia.md`)
reopened the plugin/editor track on a phased migration to the `vizia`
toolkit, with an explicit Phase 2 decision gate and a paradigm-portable
off-ramp to `egui`. Phase 1 landed: a new workspace member
`crates/mamut-vizia` carried five widgets (perform-rail, macro meter,
slot grid, oscilloscope, rotary knob), the `SessionStateSource` +
`SessionCommandSink` trait pair in `crates/mamut-runtime/src/session/state_source.rs`,
twelve passing tests, and a runnable PERFORM-screen scaffold.

The Phase 2 decision gate was reached on 2026-05-13. The first kill
criterion (rotary knob usability + general visual / interaction quality
on the running PERFORM screen) failed at the user's review of the
running binary. The other four criteria (oscilloscope frame rate,
snapshot stability under polling, LOC ratio against `egui`, end-to-end
engine interaction) were not exercised because the first criterion was
load-bearing and failed first. ADR 0002 records this in its
`## Withdrawal rationale` section.

The off-ramp planned in ADR 0002 was an `egui` restructure under the
same information architecture from ADR 0003. ADR 0003 is paradigm-
portable: the PERFORM / SOUND / SYSTEM / INSPECT cut, the cut rule, and
the legacy → new screen mapping all carry over to `egui` unchanged.

Two facts shape the cheaper-than-expected pivot:

- The `crates/mamut-runtime/src/session/state_source.rs` doctrine
  (read surface `SessionStateSource`, write surface
  `SessionCommandSink`, owned `'static`/`Send` handles, command-error
  enum, twelve tests) is paradigm-portable. It was framed against
  `vizia` in ADR 0002 but is structurally identical to what an `egui`
  consumer needs. Plan B retains the code body verbatim and neutralizes
  only its doc-comments.
- `crates/mamut-standalone` already polls snapshots at the
  `PERFORMANCE_UI_REFRESH = 75 ms` cadence, already publishes the same
  metrics surfaces, and already carries a `style.rs` token module. The
  reorganization is module-level, not architecture-level.

The plugin/editor distribution track returns to deferred per the pre-
ADR-0002 `CLAUDE.md` posture. No `VST3` / `CLAP` / `AU` work is in
scope for this ADR. If that track reopens in the future, the
`SessionStateSource` + `SessionCommandSink` surface is the natural
binding point for a plugin editor; no architectural rework is
predicted.

## Decision

Implement the ADR 0003 four-screen IA (`PERFORM` / `SOUND` / `SYSTEM`
/ `INSPECT`) inside the existing `crates/mamut-standalone` `egui`
codebase. Do not introduce a new GUI toolkit. Do not introduce a new
GUI crate. The redesign is module-level reorganization plus widget
extraction plus design-token application, alongside the existing
legacy tabs until cutover.

The redesign proceeds in seven phases (detailed in
`/home/dev/.claude/plans/general-ui-ux-redesign-iridescent-canyon.md`):

- **Phase 0** — this activation commit. Withdraw ADR 0002, accept this
  ADR 0004, delete `crates/mamut-vizia`, drop the workspace member,
  regenerate `Cargo.lock`, neutralize the doc-comments in
  `state_source.rs`, rewrite the `CLAUDE.md` GUI-track paragraph,
  rename `docs/ui/vizia-design-system.md` to
  `docs/ui/epm1-gui-design-system.md` and neutralize Vizia-specific
  implementation sections. Single atomic commit.
- **Phase 1** — adopt `SessionCommandSink` at every existing
  `session.{panic,reset_controllers,set_macro,set_direct_param}` call
  site in `crates/mamut-standalone/src/gui/`. Plumbing only.
- **Phase 2** — extract reusable widgets from `live_methods.rs` /
  `debug_methods.rs` / `engine_methods.rs` into
  `crates/mamut-standalone/src/gui/widgets/` (perform_rail, slot_grid,
  macro_meter, perform_badges, mini_scope, gfm_workbench-read-only).
- **Phase 3** — extend `crates/mamut-standalone/src/gui/style.rs`
  with the design tokens from `docs/ui/epm1-gui-design-system.md`.
- **Phase 4** — add `perform.rs` as a new tab alongside legacy tabs.
  Six tabs visible during transition.
- **Phase 5** — add `sound.rs`, `system.rs`, `inspect.rs` alongside.
  Nine tabs visible during transition. Promote `gfm_workbench` to
  edit-capable.
- **Phase 6** — swap the tab strip to the four new screens. Delete
  `live_methods.rs`, `sound_lab_methods.rs`, `engine_methods.rs`,
  `pc4_methods.rs`, `debug_methods.rs`.
- **Phase 7** — replace hardcoded color/spacing/font-size literals in
  the new screens with `style.rs` tokens. Visual polish sweep.

The alongside cutover (Phases 4–5) is deliberate. It avoids a big-bang
migration, lets the user A/B compare the new screens against the
existing rig workflows, and keeps the live performance path running
through every intermediate state.

The `SessionStateSource` + `SessionCommandSink` doctrine is locked under
Plan B. `panic` and `reset_controllers` route through
`Arc<PriorityActions>`. `set_macro` and `set_direct_param` route
through `mpsc::Sender<EngineCommand>`. `switch_patch` is deliberately
not in the sink (it takes `&mut RuntimeSession` because it rebuilds the
audio runtime); slot-grid clicks route as an
`AppEvent::RequestSlotSwitch(slot)` consumed where the session is
still owned mutably. The doctrine survives Plan B unchanged; the
amendment in ADR 0002 (2026-05-13) that established it is preserved as
historical context.

The `EPM1` transport freeze in `docs/EPM1_TRANSPORT_FREEZE.md` remains
in effect. Plan B is module-level reorganization plus widget
extraction plus design-token application. It does not reshape the
transport boundary, the audio callback, the MIDI ingress callback, the
voice allocator, the patch-load path, the factory bank, or the PC4
controller-map truth in `docs/EPM1_PC4_LIVE_PROFILE.md`. Per-phase
verification gates include an audit that the commit's file-name set is
restricted to `crates/mamut-standalone/src/gui/`, `docs/`,
`CLAUDE.md`, and the Phase 0 single-shot files.

Review discipline: cross-cutting doc+code commits go through
`sel4-integrated-systems-reviewer`. Per-file Rust changes go through
`sel4-rust-single-file-reviewer`. Subsystem-scope changes go through
`sel4-rust-systems-reviewer`. Each phase has explicit reviewer
selection in the plan file.

## Consequences

The workspace loses `crates/mamut-vizia` and the transitive deps that
`vizia` pulled in: the nine `vizia_*` sub-crates, the Skia render
stack (`skia-safe`, `skia-bindings`), Vizia's CSS engine (`cssparser`,
`selectors`, `servo_arc`), the layout crate (`morphorm`), `mundy`,
`comrak`, the `fluent-*` i18n stack, and roughly seventy-five packages
in total. `winit`, `wgpu`, and `ttf-parser` remain — they were never
Vizia-exclusive and are reached transitively through `eframe` / `egui`.
The lockfile loses approximately 1380 lines.

The GUI redesign continues entirely inside `crates/mamut-standalone`.
No second binary is shipped. No dual-toolkit coexistence pressure.
The Phase 6 cutover deletes the legacy `*_methods.rs` files; the IA
realization in `perform.rs` / `sound.rs` / `system.rs` / `inspect.rs`
becomes the only GUI surface.

Plugin/editor distribution returns to deferred. If that track reopens
in the future, the `SessionStateSource` + `SessionCommandSink` surface
is the natural editor-binding point; an `Arc<AtomicSnapshot>` push
implementation can be added later without reshaping the GUI consumer
code. No commitment is made to a toolkit, a host, or a timeline for
that work.

`mamut-tui` is unaffected by this ADR. `mamit-sint-hw` is unaffected.
The shared `mamut-platform` Linux audio platform track is unaffected.

The end-of-plan verification gate requires `tools/run-pc4-ag03.sh` to
pass end-to-end on the live `PC4 → mioXM DIN 1 → AG06/AG03` rig. The
8-slot live set names (`molten-horizon, cathedral-bloom, ember-vault,
razor-thaw, gravity-wake, furnace-choir, granite-plain, glass-tide`)
remain locked and must render at their indices on the new PERFORM
screen.

ADR 0002 is recorded as Withdrawn, not Superseded — Plan B does not
inherit Vizia's direction. The two ADRs answer the same GUI-redesign
question with different toolkit choices and different posture toward
plugin distribution.
