# Docs

This directory is the local `EPM1` documentation entrypoint for the
`Mamut EPM` software line.

Ownership split:

- this repo `mamut-sint-sw` is the canonical implementation repo for `EPM1`
- sibling repo `mamut-sint-hw` is the canonical implementation repo for `EPM2`
- shared `Mamut EPM` identity framing still remains anchored across the broader
  umbrella docs corpus
- shared Linux audio platform planning is now anchored in the separate
  `mamut-platform` docs track

Start here:

- `mamut-epm-program-map.md` - shared program map for `Mamut EPM`, `EPM1`, and `EPM2`
- `EPM1_TRANSPORT_FREEZE.md` - rescinded (ADR 0005, 2026-07-06) freeze note for
  the earlier local transport slice; preserved for history
- `EPM1_SPRINT_6_PC4_PERFORMANCE_RIG.md` - Sprint 6 backlog, acceptance bar, and live-rig contract
- `EPM1_SPRINT_6A_HOST_UNDERRUN_STABILITY.md` - next work item for real-host underrun diagnosis and performance stabilization
- `EPM1_BACKLOG_SET1_GFM_PLAYABLE_FIELD.md` - proposed backlog: note-driven
  field excitation, stereo field probe, and the `INSPECT` field view
- `EPM1_BACKLOG_SET2_GFM_RUPTURE_AND_COST.md` - proposed backlog: GFM cost
  baseline and rate policy plus rupture/recovery as a performance gesture
- `EPM1_BACKLOG_SET3_LAPTOP_MIDI_SEQUENCER.md` - proposed backlog:
  `mamut-seq` laptop MIDI sequencer as the default development-time input
  replacing the physical `PC4` rig
- `EPM1_BACKLOG_SET4_MIDI2_UMP_EXPRESSIVENESS.md` - active backlog:
  MIDI 2.0 UMP migration with per-note expressiveness deep in the engine
  and DSP (UMP-first internal protocol, translated MIDI1/MPE ingress,
  JR-timestamp scheduling); includes the ADR 0005 transport unfreeze and
  the `SET4-9..12` touch-surface track (the Android `Mamut Instrument`
  surface as the played native UMP source over a UDP link)
- `EPM1_BACKLOG_SET5_ORBITA_MOZAIK_KOSAVA.md` - active backlog
  (`SET5-3/4` Mozaik landed; `SET5-8` controller bindings landed host-side —
  audible rig pairing is the open cross-repo `TS-3`; Orbita, Kosava,
  consolidation open): three new DSP worlds beside GFM — `Orbita` (resonance
  capture, tidal dissipation, Roche breakup), `Mozaik` (quasicrystal
  cut-and-project oscillator with phason modulation), `Kosava` (gust-spectrum
  wind with per-note vortex lock-in); evidence-first, session-only controls,
  no schema growth. `SET5-8` adds the `mozaik_control` controller-profile
  binding kind and `profiles/android-touch.toml`, letting a MIDI CC (the
  PC4MS Android touch surface's configurable slide lane, tracked cross-repo in
  `pc4-microkit-studio` `docs/PC4MS-TOUCH-SURFACE-BACKLOG.md` `TS-1..3`) drive
  a Mozaik axis through the same path as the headless `mozaik set`
- `EPM1_BACKLOG_SET6_TONEWHEEL_GENERATOR.md` - accepted backlog (operator
  decisions 2026-07-14; no slices landed yet): a component-modeled
  electromechanical tonewheel organ as the first engine-global sound
  source — a shared 91-wheel generator with gear-ratio tuning, a
  key-contact/busbar model (foldback, tapering, loudness robbing, key
  click), single-trigger percussion, a dispersive vibrato/chorus scanner,
  preamp drive, and a source-agnostic two-rotor rotary-speaker layer;
  calibrated against operator-provided reference recordings through an
  offline analysis harness; evidence-first, session-only controls, no
  schema growth, and a binding generic-vocabulary naming policy
- `EPM1_MIDI2_V0.1_ALSA_UMP_SPIKE_EVIDENCE.md` - SET4-1 evidence: ALSA UMP
  feasibility spike — native UMP MIDI 2.0 client from Rust, virtual UMP
  endpoint visible to tooling, 16-bit-velocity NoteOn decoded end to end,
  the legacy downconversion trap demonstrated, and the FFI-route (B) and
  message-model (own `mamut-midi2`) decisions with the unsafe-surface
  inventory
- `EPM1_SEQ_V0.1_VIRTUAL_PORT_EVIDENCE.md` - SET3-1 evidence: `mamut-seq` virtual
  MIDI output port opens and is enumerated by `mamut-standalone list-midi`
- `EPM1_SEQ_V0.2_SCENARIO_PLAYER_EVIDENCE.md` - SET3-2 evidence: deterministic
  scenario schema and expansion, plus measured send jitter on the reference host
- `EPM1_SEQ_V0.3_SCENARIO_LIBRARY_EVIDENCE.md` - SET3-3 evidence: the six shipped
  `scenarios/*.toml`, `mod_wheel`/`sustain` steps, and `tools/run-seq-smoke.sh`
- `EPM1_SEQ_V0.4_LIVE_MODE_EVIDENCE.md` - SET3-4 evidence: the interactive `live`
  TUI, kitty keyboard-protocol detection, and the gate/latch note-model fallback
- `EPM1_PC4_LIVE_PROFILE.md` - locked Sprint 6 `PC4` control map, live slot policy, and performance-window truth model
- `EPM1_FIRST_PERFORMANCE_PLAYBOOK.md` - shortest trustworthy path for the first real `PC4` performance session
- `adrs/0001-standalone-midi-ingress-hardening.md` - accepted narrow bugfix
  decision for MIDI callback hygiene, drop visibility, and controller burst
  coalescing under the then-active transport freeze (amended in part by
  ADR 0005; its realtime rules remain standing requirements)
- `adrs/0002-reopen-plugin-editor-track-via-vizia.md` - withdrawn
  2026-05-13; preserved for context. The Phase 1 Vizia spike failed the
  decision gate and the plugin/editor track returns to deferred per
  ADR 0004.
- `adrs/0003-gui-information-architecture.md` - accepted four-screen GUI
  information architecture (`PERFORM`, `SOUND`, `SYSTEM`, `INSPECT`) with a
  persistent perform-rail and a single cut rule for future GUI features
- `adrs/0004-egui-ia-restructure-plan-b.md` - accepted decision to
  implement the ADR 0003 IA inside the existing `crates/mamut-standalone`
  `egui` codebase (no new GUI crate); supersedes ADR 0002 in posture
- `adrs/0005-midi2-ump-transport-unfreeze.md` - accepted MIDI 2.0 UMP
  direction: UMP-first internal protocol, translated MIDI 1.0/MPE ingress,
  JR-timestamp scheduling; rescinds the `EPM1_TRANSPORT_FREEZE.md` freeze
  and amends ADR 0001's escape clause in part
- `adrs/0006-tonewheel-generator-track.md` - accepted tonewheel generator
  track decision record (Backlog Set 6): component model with one global
  shared 91-wheel generator (rejecting the per-voice additive extension),
  organ-bus chain position, session-only control doctrine with the
  registration vector as one control, the binding generic-vocabulary
  naming policy, reference-recording calibration strategy, and the rotary
  speaker as a source-agnostic engine-global layer
- `ui/epm1-gui-design-system.md` - palette, typography, knob geometry,
  panel chrome, density rules, screen wireframes, and high-risk widget
  rendering strategies for the GUI redesign
- `dsp/primitives-math.md` - formula companion for the low-level Rust DSP primitives
- `dsp/render-path-math.md` - frame-level math for voice allocation, mixing, filtering, and final render stages
- `dsp/control-identity-math.md` - macro, identity, and direct-parameter resolution math
- `dsp/masnoca.md` - curated tonal-architecture note for mass, warmth, power,
  and analog-inspired DSP direction
- `live-sessions/` - lab evidence from real hardware runs; exact device names,
  ALSA selectors, and raw MIDI snippets are preserved as evidence, not portable
  defaults
- `live-sessions/2026-04-29-pc4-ag03-headless.md` - first documented real `PC4 -> mioXM DIN 1 -> EPM1 -> AG03` headless hardware session
- `live-sessions/2026-04-29-pc4-ag03-headless-performance-mode.md` - follow-up PC4 Performance mode run with startup entry events and stuck-held-note finding
- `live-sessions/2026-04-29-pc4-ag03-clean-idle-noteoff-fix.md` - clean-idle Global MIDI follow-up and repeated-pitch note-off engine fix
- `live-sessions/2026-04-29-pc4-ag03-noteoff-fix-validated.md` - hardware validation that the repeated-pitch note-off fix clears the stuck-note failure
- `live-sessions/2026-04-29-pc4-ag03-full-controller-map.md` - full one-way `PC4` sustain/K/S/SW/wheels/aftertouch hardware map validation into Mamut
- `live-sessions/2026-04-29-pc4-ag03-windowed-gui-redesign-live-take.md` - windowed GUI redesign live take, layout regression/fix, and real PC4 control evidence
- `live-sessions/2026-05-08-cathedral-bloom-gravitacija-live-take.md` - 60-second `Cathedral Bloom` direct-output live take with MIDI sidecar and full audio/control analysis
- `live-sessions/2026-04-30-direct-output-capture-analysis.md` - first direct-output f32 WAV analysis, including valid recorder evidence and the DC-offset follow-up
- `live-sessions/2026-04-30-direct-output-dc-blocker-validation.md` - direct-output f32 WAV validation after the master DC blocker, with the remaining headroom and denormal follow-ups
- `EXTERNAL_DSP_REFERENCES.md` - local notes on external DSP references such as
  `AudioNoise` and `dspc`
- `dsp/implementation-language-strategy.md` - Rust plus narrow C strategy for
  next-generation DSP implementation
- `dsp/dsp-core-polish-v0.1-output-safety-evidence.md` - core output safety
  polish evidence for headroom limiter and denormal flush
- `dsp/dsp-core-polish-v0.2-output-safety-sweep-evidence.md` - output safety
  telemetry and factory sweep evidence
- `dsp/gfm-v0.1-baseline.md` - first accepted audible `mamut-field` evidence
  baseline
- `dsp/gfm-v0.2-gesture-evidence.md` - attack-hold-release `mamut-field`
  gesture evidence
- `dsp/gfm-v0.3-gesture-variants-evidence.md` - combined short-strike,
  slow-press, and repeated-strike `mamut-field` evidence
- `dsp/gfm-v0.4-recovery-performance-evidence.md` - recovery-safe offline
  `mamut-field` performance evidence
- `dsp/gfm-v0.5-performance-contract.md` - public `mamut-field`
  performance program and gesture contract
- `dsp/gfm-v0.6-engine-dry-run-evidence.md` - first one-way
  `mamut-engine -> mamut-field` dry-run evidence
- `dsp/gfm-v0.7-engine-block-evidence.md` - block-rendering
  `mamut-engine -> mamut-field` evidence
- `dsp/gfm-v0.8-patch-selection-evidence.md` - offline factory patch
  identity-to-GFM program selection evidence
- `dsp/gfm-v0.9-patch-voice-factory-evidence.md` - patch-to-GFM voice
  factory contract evidence
- `dsp/gfm-v1.0-engine-layer-evidence.md` - first offline-only engine voice
  graph GFM layer evidence
- `dsp/gfm-v1.1-layer-gate-evidence.md` - explicit engine-owned GFM layer gate
  evidence
- `dsp/gfm-v1.2-standalone-runtime-flag-evidence.md` - standalone launch-time
  GFM layer flag evidence
- `dsp/gfm-v1.3-gui-runtime-toggle-evidence.md` - windowed GUI GFM runtime
  toggle evidence without full audio runtime rebuild
- `dsp/gfm-v1.4-midi-performance-controls-evidence.md` - MIDI aftertouch and
  K8/CC3 GFM amount control evidence
- `dsp/gfm-v1.5-momentary-midi-gate-evidence.md` - armed GFM layer with
  aftertouch momentary gate evidence
- `dsp/gfm-v1.6-midi-auto-arm-evidence.md` - K8/CC3 GFM auto-arm evidence for
  momentary MIDI performance without GUI pre-enable
- `dsp/gfm-v1.7-smoothed-momentary-color-evidence.md` - smoothed GFM
  aftertouch/K8 color entry and release evidence
- `dsp/gfm-v1.8-swapped-k8-aftertouch-controls-evidence.md` - swapped K8 gate
  and aftertouch amount control evidence
- `dsp/gfm-v1.9-field-workbench-evidence.md` - experimental field workbench
  surface (reverted from production; reference for the `INSPECT` field view)
- `dsp/gfm-v2.0-seven-program-evidence.md` - experimental seven-program
  surface (not production; the three-program contract stands)
- `dsp/gfm-v2.1-note-strike-excitation-evidence.md` - note-driven field
  strike excitation evidence (`SET1-1`; the field hears played notes)
- `dsp/gfm-v2.2-stereo-probe-evidence.md` - stereo field probe evidence
  (`SET1-2`; offset tap pair replaces the artificial GFM-layer spread)
- `dsp/gfm-v2.3-inspect-field-view-evidence.md` - read-only GFM field view
  on the `INSPECT` screen (`SET1-3`; live 16x16 terrain, strike/probe overlays)
- `dsp/bcs-v0.1-hopf-duffing-playground-evidence.md` - first offline
  Hopf/Duffing BCS playground render evidence
- `dsp/bcs-v1.0-engine-layer-smoke-evidence.md` - disabled-by-default BCS
  engine layer smoke evidence
- `dsp/bcs-v1.1-standalone-runtime-flag-evidence.md` - standalone dry-run/play
  BCS scenario flag and interactive command evidence
- `dsp/bcs-v1.2-playable-midi-layer-evidence.md` - playable BCS MIDI layer
  evidence for PC4 `S9` amount and `SW9` enable
- `dsp/mozaik-v0.1-quasicrystal-osc-evidence.md` - quasicrystal (cut-and-project)
  oscillator primitive evidence (`SET5-3`; Q32 Bresenham word, Hann tiles,
  phason latch, detent walk, pitch-anchor honesty table)
- `dsp/mozaik-v0.2-voice-source-evidence.md` - Mozaik engine voice-source
  integration evidence (`SET5-4`; per-voice pre-filter blend, five session
  controls, `--mozaik`/headless surface, disabled bit-identity vs the
  pre-slice baseline, measured per-block cost)

Current source-doc posture:

- local `EPM1` implementation truth lives in this repo's code, runtime, patches,
  and top-level README
- local transport architecture work was re-opened on 2026-07-06 by ADR 0005
  for the MIDI 2.0 UMP track (Backlog Set 4); the ADR 0001 realtime rules
  remain standing requirements, and transport changes ride the Set 4 review
  gates
- the current shared software/source architecture docs that shaped `EPM1`
  still live in the sibling `mamut-sint-hw` docs track
- the current shared Linux audio platform direction now lives in
  the sibling `mamut-platform` docs track

Current `EPM1` source anchors:

- sibling `mamut-sint-hw`: `docs/software-clone-architecture.md`
- sibling `mamut-sint-hw`: `docs/rust-workspace-architecture.md`
- sibling `mamut-sint-hw`: `docs/dsp-subsystem-spec.md`
- `dsp/primitives-math.md`
- `dsp/render-path-math.md`
- `dsp/control-identity-math.md`
- sibling `mamut-sint-hw`: `docs/patch-schema-v1.md`
- `../mamut-platform/docs/LINUX_AUDIO_PLATFORM_THESIS.md`
- `../mamut-platform/docs/LINUX_AUDIO_STACK_DECISION.md`
- `../mamut-platform/docs/C_RUST_BOUNDARY.md`
- `../mamut-platform/docs/LINUX_AUDIO_OPEN_QUESTIONS.md`
- `../mamut-platform/docs/LINUX_AUDIO_TRANSPORT_ARCHITECTURE.md`
- `../mamut-platform/docs/EPM1_FREEZE_AND_RESUME.md`
