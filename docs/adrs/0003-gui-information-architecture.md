# ADR 0003: GUI Information Architecture

Status: Accepted

## Context

The current windowed GUI in `crates/mamut-standalone/src/gui/` has evolved
by accretion. It now ships five top-level tabs:

- `LIVE` — performance rig surface; GFM and BCS layer controls; macros;
  live slots; factory bank; take recorder.
- `SOUND LAB` — patch editing across ten sub-pages (`Osc1`, `Osc2`,
  `Noise`, `Spectral`, `Relations`, `Body/Filter`, `Motion`,
  `Performance`, `Layers`, `Masnoca`).
- `ENGINE` — DSP internals; ten workbench sub-pages overlapping
  `SOUND LAB` in name (`Osc1`, `Osc2`, `Spectral`, `Relations`,
  `Body/Filter`, `Masnoca`, `GfmField`, `Motion`, `Performance`,
  `Layers`); voice inspector; oscilloscope; 17-section parameter grid.
- `PC4` — read-only mirror of the connected hardware controller.
- `DEBUG` — runtime config; latest MIDI event; ALSA queue metrics.

Three structural problems result from this shape:

- **Live and debug concerns are interleaved.**
  `crates/mamut-standalone/src/gui/live_methods.rs` carries a performance
  status strip *and* the tab dispatch; the always-visible footer with
  `PANIC` and `RESET CTRLS` lives in
  `crates/mamut-standalone/src/gui/debug_methods.rs`. The module layout
  reflects history, not the user mental model.
- **`SOUND LAB` and `ENGINE` overlap.** Both surfaces carry sub-pages
  named `Osc1`, `Osc2`, `Spectral`, `Relations`, `Body/Filter`,
  `Performance`, `Layers`. The split was a development artifact (patch
  editing vs DSP internals), not a user task boundary.
- **`CLAUDE.md` already disagrees with the code.** Line 113 describes the
  GUI as "Live, PC4, and Debug tabs"; the live state has five tabs. The
  integrated reviewer's standing complaint about doc/code drift is
  already true here.

The `mamut-tui` terminal UI (`crates/mamut-tui/src/render.rs`) is also
relevant. It mirrors the same conceptual surfaces (`Live`, `SoundLab`,
`PC4`, `Debug`) but enforces a tighter signal hierarchy because terminal
space is scarce. Its `Live` screen layout — patch / take recorder /
runtime health row, macros and live set row, layers and factory bank row
— is a useful prior for what counts as "live-critical" on `EPM1`.

This ADR settles the information architecture for the GUI redesign,
independently of the toolkit choice in ADR 0002. The same IA survives the
ADR 0002 off-ramp; only widget code is paradigm-specific.

## Decision

The GUI is restructured around **one persistent perform-rail and four
screens**.

### Persistent perform-rail

The perform-rail is visible on every screen. It carries only state that
the performer wants to see at every moment, regardless of which screen
they are currently using:

- Patch name and live-slot index.
- Audio device label and sample rate.
- MIDI device, channel, and a single-character activity flash.
- Transport health as a single `xrun` count badge; expand-on-click to
  reveal the full transport metrics surface from
  `crates/mamut-runtime/src/types/metrics.rs`.
- Active voice count, sustain pedal state, and clip indicator.
- Last control event as a one-line label/value/verdict from
  `InputMetrics::last_control`.
- `PANIC`, `RESET CTRLS`, `QUIT` as always-available action affordances.

### Four screens

| Screen | Mental rule | Contents |
|---|---|---|
| `PERFORM` | look at it while playing | five macro meters or knobs; live-set 4×2 slot grid; factory bank as a side panel; held notes and voice activity; take-recorder strip (compact form); optional read-only GFM field mirror; optional `PC4` controller mirror as a collapsible sidebar |
| `SOUND` | design with it before playing | section panels grounded in the current `SOUND LAB` sub-pages (`Osc`, `Noise`, `Body/Filter`, `Spectral`, `Relations`, `Motion`, `Performance`, `Layers`, `Masnoca`) using knob-heavy panel layout, not stacked form rows; MIDI focus indicator; GFM field workbench in edit-capable form; BCS layer scenario chooser; master section and output safety |
| `SYSTEM` | configure it once and forget | audio device and ALSA tuning; MIDI device and channel; `PC4` controller-profile configuration; take-recorder destination and format; startup guard, filter, and channel truth; recovery actions |
| `INSPECT` | open it only when something is wrong | voice inspector and phase visualization; MIDI trace (raw events and verdicts); transport, input, and recording counters; identity and derived state floats; full oscilloscope; current-patch JSON dump |

### Cut rule

The single rule that decides where a new control belongs:

- If the performer would look at it while playing, it goes on `PERFORM`.
- If a performer would design with it before playing, it goes on `SOUND`.
- If an operator would configure it once and forget, it goes on `SYSTEM`.
- If a developer or operator would only open it when something is wrong,
  it goes on `INSPECT`.

The rule is intended to absorb future-feature pressure without re-growing
the tab count.

### Concrete collapses vs the current GUI

- `SOUND LAB` and `ENGINE` merge into `SOUND`. The ten/ten overlapping
  sub-pages collapse into one set of section panels. The deeper DSP
  diagnostics (voice phase, raw counters, lattice introspection) move to
  `INSPECT`. The `GfmField` workbench, which today appears in both
  `SOUND LAB` and `ENGINE`, lives in `SOUND` as the canonical edit
  surface; `PERFORM` may carry an optional read-only mirror.
- `PC4` is demoted. Profile configuration moves to `SYSTEM`. The live
  mirror of hardware controller state moves to `PERFORM` as a collapsible
  sidebar, because controller activity is a live concern, not a
  configuration concern.
- `DEBUG` is renamed to `INSPECT` and broadened. It absorbs the parameter
  grid, voice inspector, full oscilloscope, and identity/derived floats
  that previously bled into `ENGINE`.
- The performance status strip in
  `crates/mamut-standalone/src/gui/live_methods.rs` and the always-visible
  footer in `crates/mamut-standalone/src/gui/debug_methods.rs` both
  collapse into the new perform-rail. The footer's `PANIC` and `RESET`
  actions are preserved verbatim.

### Module shape

ADR 0002 originally specified a parallel `crates/mamut-vizia/` skeleton
as the primary path; that path was withdrawn on 2026-05-13 (see ADR
0002 `## Withdrawal rationale` and ADR 0004). The same four-screen IA
is realized inside the existing `crates/mamut-standalone/src/gui/`
codebase (`egui`) per ADR 0004, with the following module
reorganization:

- `perform.rs` replaces the `LIVE`-tab portion of `live_methods.rs`.
- `sound.rs` replaces `sound_lab_methods.rs` and the patch-design portion
  of `engine_methods.rs`.
- `system.rs` absorbs `pc4_methods.rs` and the device-config portion of
  the current footer.
- `inspect.rs` replaces `debug_methods.rs` and absorbs the diagnostic
  portion of `engine_methods.rs`.
- `perform_rail.rs` absorbs the status strip from `live_methods.rs` and
  the action footer from `debug_methods.rs`.

The IA from this ADR is the contract; the toolkit is the variable.

## Consequences

`CLAUDE.md` line 113 must be rewritten to reflect this IA, not the
current five-tab state. That update is part of the Phase 0 deliverable in
ADR 0002.

The locked PC4 live profile in `docs/EPM1_PC4_LIVE_PROFILE.md` is
unaffected by this ADR. The `PC4` controller binding map and live-slot
policy remain spec; only the *display* of those bindings moves between
screens.

The locked 8-slot live-set in `crates/mamut-runtime` and the factory
bank under `patches/factory/*.toml` are unaffected. Their names remain
spec, per the existing `CLAUDE.md` rule.

The `mamut-tui` IA is not changed by this ADR. The TUI carries
similarly-named surfaces (`Live`, `SoundLab`, `PC4`, `Debug`) but its cut
was made against terminal-space constraints, not against the cut rule in
this ADR; the resemblance is structural, not contractual. Aligning the
TUI to the new IA, if pursued, is a separate decision.

Future GUI features should pin themselves to the cut rule above rather
than growing the tab count. New tabs should be a last resort, treated as
evidence that the cut rule has become wrong rather than that a new
top-level surface is needed.

If the ADR 0002 Phase 2 gate fails, the egui off-ramp uses the module
shape listed above. The IA decision in this ADR is unchanged in that
case.
